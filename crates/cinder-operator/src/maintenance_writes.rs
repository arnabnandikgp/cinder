//! Single-writer maintenance WAL. Only public parameters and opaque private
//! state/write commitments survive restart; unsigned plans may be cancelled,
//! but uncertain signed transactions may only be observed, never re-signed.
use super::{u64_blob, Journal, JournalError};
use crate::rpc::{Result, RuntimeError};
use crate::runtime::{book_key, config_key, ledger_id, vault_id};
use crate::transaction::{decode_ix, instruction_data, Receipt};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use solana_keypair::Keypair;
use solana_signer::Signer;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WriteMetadata {
    attempt: [u8; 32],
    created_ms: u64,
    parameters: MaintenanceParameters,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) enum MaintenanceParameters {
    Fold {
        epoch: u64,
    },
    Liquidate {
        asset: u16,
        oid: [u8; 16],
        bound: u64,
        deadline: u64,
        close_lots: u64,
        post_im: u64,
    },
    Root {
        epoch: u64,
        root: [u8; 32],
        users: u32,
        free: u64,
        reserved: u64,
        debt: u64,
        book_hash: [u8; 32],
        ack_count: u64,
        observed_ms: u64,
    },
    Heartbeat {
        now_ms: u64,
    },
    Collateral {
        usdc: u64,
    },
    PostCollateral {
        nonce: [u8; 32],
        amount: u64,
        trader: [u8; 32],
        program: [u8; 32],
        gti: u8,
    },
}
impl MaintenanceParameters {
    pub fn kind(&self) -> i64 {
        match self {
            Self::Fold { .. } => 1,
            Self::Liquidate { .. } => 2,
            Self::Root { .. } => 3,
            Self::Heartbeat { .. } => 4,
            Self::Collateral { .. } => 5,
            Self::PostCollateral { .. } => 6,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WriteState {
    Prepared,
    Unknown,
    Applied(u64),
    Rejected(u64),
    Cancelled,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MaintenanceWrite {
    pub id: [u8; 32],
    pub scope: [u8; 32],
    pub body_hash: [u8; 32],
    pub private_hash: [u8; 32],
    pub evidence_hash: [u8; 32],
    pub parameters: MaintenanceParameters,
    pub attempt: [u8; 32],
    pub created_ms: u64,
    pub signature: Option<[u8; 64]>,
    pub expiry: Option<u64>,
    pub state: WriteState,
}
impl MaintenanceWrite {
    fn valid(&self) -> bool {
        if self.attempt == [0; 32]
            || self.created_ms == 0
            || self.scope == [0; 32]
            || self.body_hash == [0; 32]
            || self.private_hash == [0; 32]
            || self.evidence_hash == [0; 32]
        {
            return false;
        }
        match self.parameters {
            MaintenanceParameters::Fold { .. } => self.scope != book_key(),
            MaintenanceParameters::Liquidate {
                asset,
                oid,
                bound,
                deadline,
                close_lots,
                ..
            } => {
                self.scope != book_key()
                    && asset != 0
                    && oid != [0; 16]
                    && bound != 0
                    && deadline != 0
                    && close_lots != 0
            }
            MaintenanceParameters::Root {
                epoch,
                root,
                book_hash,
                observed_ms,
                ..
            } => {
                epoch != u64::MAX
                    && root != [0; 32]
                    && book_hash != [0; 32]
                    && observed_ms != 0
                    && self.scope == crate::runtime::pda(vault_id(), &[cinder_common::SEED_RESERVE])
            }
            MaintenanceParameters::Heartbeat { now_ms } => self.scope == book_key() && now_ms != 0,
            MaintenanceParameters::Collateral { .. } => self.scope == book_key(),
            MaintenanceParameters::PostCollateral {
                nonce,
                amount,
                trader,
                program,
                gti,
            } => {
                nonce != [0; 32]
                    && amount > 0
                    && trader != [0; 32]
                    && program != [0; 32]
                    && gti > 0
                    && gti < 64
                    && self.scope
                        == crate::runtime::pda(
                            vault_id(),
                            &[
                                cinder_vault::SEED_PHOENIX_FUNDING,
                                &crate::FundingIntent::new(nonce, amount, trader, program)
                                    .funding_id,
                            ],
                        )
            }
        }
    }
    pub fn new(
        scope: [u8; 32],
        body: &[u8],
        private_hash: [u8; 32],
        evidence_hash: [u8; 32],
        parameters: MaintenanceParameters,
    ) -> Self {
        let body_hash = Sha256::digest(body).into();
        let mut out = Self {
            id: [0; 32],
            scope,
            body_hash,
            private_hash,
            evidence_hash,
            parameters,
            attempt: Keypair::new().pubkey().to_bytes(),
            created_ms: crate::unix_ms(),
            signature: None,
            expiry: None,
            state: WriteState::Prepared,
        };
        out.id = out.identity();
        out
    }
    fn identity(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(b"cinder:maintenance-write:v1");
        h.update(self.scope);
        h.update(self.body_hash);
        h.update(self.private_hash);
        h.update(self.evidence_hash);
        h.update(self.attempt);
        h.update(self.created_ms.to_le_bytes());
        h.update(
            serde_json::to_vec(&self.parameters).expect("fixed serializable maintenance metadata"),
        );
        h.finalize().into()
    }
    pub fn receipt(&self, value: &Value, adapter: [u8; 32]) -> Result<WriteState> {
        let sig = self.signature.ok_or(RuntimeError::Identity)?;
        let r = Receipt::decode(value, &sig)?;
        let native = matches!(
            self.parameters,
            MaintenanceParameters::PostCollateral { .. }
        );
        if r.slot == 0
            || !r.requires_signature(&adapter)
            || r.instructions.len() != if native { 2 } else { 1 }
        {
            return Err(RuntimeError::Identity);
        }
        if native {
            let budget = &r.instructions[0];
            let mut expected = vec![2];
            expected.extend_from_slice(&1_400_000u32.to_le_bytes());
            if r.program(budget)?
                != crate::transaction::bytes("ComputeBudget111111111111111111111111111111")?
                || instruction_data(budget)? != expected
                || !crate::rpc::array(&budget["accounts"])?.is_empty()
            {
                return Err(RuntimeError::Identity);
            }
        }
        let ix = &r.instructions[usize::from(native)];
        let data = instruction_data(ix)?;
        let program = if matches!(
            self.parameters,
            MaintenanceParameters::Root { .. } | MaintenanceParameters::PostCollateral { .. }
        ) {
            vault_id()
        } else {
            ledger_id()
        };
        if r.program(ix)? != program
            || r.account(ix, 0)? != adapter
            || r.account(ix, 1)? != config_key()
            || <[u8; 32]>::from(Sha256::digest(&data)) != self.body_hash
        {
            return Err(RuntimeError::Identity);
        }
        let accounts = crate::rpc::array(&ix["accounts"])?;
        if native {
            let MaintenanceParameters::PostCollateral {
                nonce,
                amount,
                trader,
                program,
                gti,
            } = self.parameters
            else {
                unreachable!()
            };
            let intent = crate::FundingIntent::new(nonce, amount, trader, program);
            let p = decode_ix::<cinder_vault::instruction::FundPhoenix>(&data)?
                .ok_or(RuntimeError::Identity)?;
            if accounts.len() < 20
                || accounts.len() > 82
                || accounts.len() <= 18 + usize::from(gti)
                || r.account(ix, 2)?
                    != crate::runtime::pda(vault_id(), &[cinder_common::SEED_VAULT_AUTHORITY])
                || r.account(ix, 3)? != self.scope
                || r.account(ix, 11)? != program
                || r.account(ix, 14)? != trader
                || p.funding_id != intent.funding_id
                || p.amount != amount
                || p.global_trader_index_count != gti
            {
                return Err(RuntimeError::Identity);
            }
            return Ok(if r.succeeded {
                WriteState::Applied(r.slot)
            } else {
                WriteState::Rejected(r.slot)
            });
        }
        if accounts.len()
            != if matches!(
                self.parameters,
                MaintenanceParameters::Fold { .. } | MaintenanceParameters::Liquidate { .. }
            ) {
                4
            } else {
                3
            }
            || r.account(ix, 2)? != self.scope
        {
            return Err(RuntimeError::Identity);
        }
        match &self.parameters {
            MaintenanceParameters::Fold { epoch } => {
                let p = decode_ix::<cinder_ledger::instruction::AllocateFunding>(&data)?
                    .ok_or(RuntimeError::Identity)?;
                if !p.fold
                    || p.epoch != *epoch
                    || p.entries.iter().any(|e| e.delta_usdc != 0)
                    || r.account(ix, 3)? != book_key()
                {
                    return Err(RuntimeError::Identity);
                }
            }
            MaintenanceParameters::Liquidate {
                asset,
                oid,
                bound,
                deadline,
                close_lots,
                post_im,
            } => {
                let p = decode_ix::<cinder_ledger::instruction::LiquidateUserBounded>(&data)?
                    .ok_or(RuntimeError::Identity)?;
                if p.asset_id != *asset
                    || p.client_oid != *oid
                    || p.limit_price_ticks != *bound
                    || p.last_valid_slot != *deadline
                    || p.close_lots != *close_lots
                    || p.post_position_im_usdc != *post_im
                    || r.account(ix, 3)? != book_key()
                {
                    return Err(RuntimeError::Identity);
                }
            }
            MaintenanceParameters::Heartbeat { now_ms } => {
                let p = decode_ix::<cinder_ledger::instruction::HeartbeatScan>(&data)?
                    .ok_or(RuntimeError::Identity)?;
                if self.scope != book_key() || p.now_ms != *now_ms {
                    return Err(RuntimeError::Identity);
                }
            }
            MaintenanceParameters::Collateral { usdc } => {
                let p = decode_ix::<cinder_ledger::instruction::UpdateBookCollateral>(&data)?
                    .ok_or(RuntimeError::Identity)?;
                if self.scope != book_key() || p.phoenix_collateral != *usdc {
                    return Err(RuntimeError::Identity);
                }
            }
            MaintenanceParameters::Root {
                epoch,
                root,
                users,
                free,
                reserved,
                debt,
                book_hash,
                ..
            } => {
                let p = decode_ix::<cinder_vault::instruction::WriteReserveRootGuarded>(&data)?
                    .ok_or(RuntimeError::Identity)?;
                if self.scope != crate::runtime::pda(vault_id(), &[cinder_common::SEED_RESERVE])
                    || p.expected_epoch != *epoch
                    || p.root != *root
                    || p.user_count != *users
                    || p.total_free != *free
                    || p.total_reserved != *reserved
                    || p.total_bad_debt != *debt
                    || p.book_hash != *book_hash
                {
                    return Err(RuntimeError::Identity);
                }
            }
            MaintenanceParameters::PostCollateral { .. } => unreachable!(),
        }
        Ok(if r.succeeded {
            WriteState::Applied(r.slot)
        } else {
            WriteState::Rejected(r.slot)
        })
    }
}
fn fixed<const N: usize>(b: Vec<u8>) -> std::result::Result<[u8; N], JournalError> {
    b.try_into()
        .map_err(|_| JournalError::CorruptState("invalid maintenance field"))
}
impl Journal {
    pub(crate) fn reserve_debt_changed(
        &self,
        debt: u64,
    ) -> std::result::Result<bool, JournalError> {
        let id:Option<Vec<u8>>=self.connection.query_row("SELECT id FROM maintenance_writes WHERE kind=3 AND state=2 ORDER BY rowid DESC LIMIT 1",[],|r|r.get(0)).optional()?;
        let previous = if let Some(id) = id {
            let w = self
                .maintenance_write(fixed(id)?)?
                .ok_or(JournalError::IntentConflict)?;
            let MaintenanceParameters::Root { debt, .. } = w.parameters else {
                return Err(JournalError::IntentConflict);
            };
            debt
        } else {
            0
        };
        Ok(previous != debt)
    }
    pub(crate) fn reserve_due(&self, now: u64) -> std::result::Result<bool, JournalError> {
        let latest:Option<(i64,Vec<u8>)>=self.connection.query_row("SELECT rowid,id FROM maintenance_writes WHERE kind=3 AND state=2 ORDER BY rowid DESC LIMIT 1",[],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
        let (rowid, previous_count, previous_ms) = if let Some((rowid, id)) = latest {
            let w = self
                .maintenance_write(fixed(id)?)?
                .ok_or(JournalError::IntentConflict)?;
            let MaintenanceParameters::Root {
                ack_count,
                observed_ms,
                ..
            } = w.parameters
            else {
                return Err(JournalError::IntentConflict);
            };
            (rowid, ack_count, observed_ms)
        } else {
            (0, 0, 0)
        };
        if previous_ms > now {
            return Err(JournalError::IntentConflict);
        }
        let incident:bool=self.connection.query_row("SELECT EXISTS(SELECT 1 FROM maintenance_writes WHERE rowid>? AND kind IN (1,2) AND state=2)",params![rowid],|r|r.get(0))?;
        if incident {
            return Ok(true);
        }
        let count = self.acknowledged_fills()?;
        let outstanding = count
            .checked_sub(previous_count)
            .ok_or(JournalError::IntentConflict)?;
        if outstanding >= cinder_common::COMMIT_EVERY_FILLS {
            return Ok(true);
        }
        if outstanding == 0 {
            return Ok(false);
        }
        let first:Vec<u8>=self.connection.query_row("SELECT updated_at_ms FROM operations WHERE state=? AND filled_lots!=0 ORDER BY updated_at_ms,rowid LIMIT 1 OFFSET ?",params![crate::OrderState::Acked as i64,i64::try_from(previous_count).map_err(|_|JournalError::IntentConflict)?],|r|r.get(0))?;
        let first = u64::from_be_bytes(fixed(first)?);
        let start = if previous_ms == 0 { first } else { previous_ms };
        Ok(now.checked_sub(start).ok_or(JournalError::IntentConflict)?
            >= cinder_common::COMMIT_EVERY_MS)
    }
    pub(crate) fn acknowledged_fills(&self) -> std::result::Result<u64, JournalError> {
        let count: i64 = self.connection.query_row(
            "SELECT count(*) FROM operations WHERE state=? AND filled_lots!=0",
            params![crate::OrderState::Acked as i64],
            |r| r.get(0),
        )?;
        u64::try_from(count).map_err(|_| JournalError::CorruptState("invalid acknowledged count"))
    }
    pub(crate) fn maintenance_write(
        &self,
        id: [u8; 32],
    ) -> std::result::Result<Option<MaintenanceWrite>, JournalError> {
        let mut q=self.connection.prepare("SELECT scope,body_hash,private_hash,evidence_hash,parameters,signature,expiry,state,slot,kind FROM maintenance_writes WHERE id=?")?;
        let raw = q
            .query_row(params![id.as_slice()], |r| {
                Ok((
                    r.get::<_, Vec<u8>>(0)?,
                    r.get::<_, Vec<u8>>(1)?,
                    r.get::<_, Vec<u8>>(2)?,
                    r.get::<_, Vec<u8>>(3)?,
                    r.get::<_, Vec<u8>>(4)?,
                    r.get::<_, Option<Vec<u8>>>(5)?,
                    r.get::<_, Option<Vec<u8>>>(6)?,
                    r.get::<_, i64>(7)?,
                    r.get::<_, Option<Vec<u8>>>(8)?,
                    r.get::<_, i64>(9)?,
                ))
            })
            .optional()?;
        let Some((scope, body, private, evidence, p, sig, expiry, state, slot, kind)) = raw else {
            return Ok(None);
        };
        let metadata: WriteMetadata = serde_json::from_slice(&p)
            .map_err(|_| JournalError::CorruptState("invalid maintenance metadata"))?;
        if p.len() > 4096
            || serde_json::to_vec(&metadata)
                .map_err(|_| JournalError::CorruptState("invalid maintenance metadata"))?
                != p
            || metadata.parameters.kind() != kind
        {
            return Err(JournalError::CorruptState("invalid maintenance metadata"));
        }
        let slot = slot
            .map(|s| fixed::<8>(s).map(u64::from_be_bytes))
            .transpose()?;
        let state = match (state, slot) {
            (0, None) => WriteState::Prepared,
            (1, None) => WriteState::Unknown,
            (2, Some(s)) if s > 0 => WriteState::Applied(s),
            (3, Some(s)) if s > 0 => WriteState::Rejected(s),
            (4, None) => WriteState::Cancelled,
            _ => return Err(JournalError::CorruptState("invalid maintenance finality")),
        };
        let signature = sig.map(fixed::<64>).transpose()?;
        let expiry = expiry
            .map(|e| fixed::<8>(e).map(u64::from_be_bytes))
            .transpose()?;
        let out = MaintenanceWrite {
            id,
            scope: fixed(scope)?,
            body_hash: fixed(body)?,
            private_hash: fixed(private)?,
            evidence_hash: fixed(evidence)?,
            parameters: metadata.parameters,
            attempt: metadata.attempt,
            created_ms: metadata.created_ms,
            signature,
            expiry,
            state,
        };
        if out.identity() != id
            || !out.valid()
            || out.attempt == [0; 32]
            || out.created_ms == 0
            || signature == Some([0; 64])
            || expiry == Some(0)
            || matches!(state, WriteState::Prepared | WriteState::Cancelled) != signature.is_none()
            || signature.is_none() != expiry.is_none()
        {
            return Err(JournalError::CorruptState("invalid maintenance write"));
        }
        Ok(Some(out))
    }
    pub(crate) fn active_maintenance_write(
        &self,
    ) -> std::result::Result<Option<MaintenanceWrite>, JournalError> {
        let id: Option<Vec<u8>> = self
            .connection
            .query_row(
                "SELECT id FROM maintenance_writes WHERE state IN (0,1)",
                [],
                |r| r.get(0),
            )
            .optional()?;
        id.map(|b| self.maintenance_write(fixed(b)?))
            .transpose()
            .map(Option::flatten)
    }
    pub(crate) fn prepare_maintenance_write(
        &mut self,
        w: &MaintenanceWrite,
    ) -> std::result::Result<(), JournalError> {
        if w.state != WriteState::Prepared
            || !w.valid()
            || w.signature.is_some()
            || w.expiry.is_some()
            || w.identity() != w.id
            || self.has_unresolved_maintenance()?
            || self.has_unresolved_funding()?
            || !self.nonterminal_operations()?.is_empty()
        {
            return Err(JournalError::IntentConflict);
        }
        if self.funding_checkpoint()?.is_none() {
            return Err(JournalError::IntentConflict);
        }
        if matches!(
            w.parameters,
            MaintenanceParameters::Fold { .. } | MaintenanceParameters::Liquidate { .. }
        ) && !self.known_users()?.iter().any(|(a, _)| *a == w.scope)
        {
            return Err(JournalError::IntentConflict);
        }
        self.connection.execute("INSERT INTO maintenance_writes(id,kind,scope,body_hash,private_hash,evidence_hash,parameters) VALUES(?,?,?,?,?,?,?)",params![w.id.as_slice(),w.parameters.kind(),w.scope.as_slice(),w.body_hash.as_slice(),w.private_hash.as_slice(),w.evidence_hash.as_slice(),serde_json::to_vec(&WriteMetadata{attempt:w.attempt,created_ms:w.created_ms,parameters:w.parameters.clone()}).map_err(|_|JournalError::IntentConflict)?])?;
        Ok(())
    }
    pub(crate) fn sign_maintenance_write(
        &mut self,
        id: [u8; 32],
        sig: [u8; 64],
        expiry: u64,
    ) -> std::result::Result<(), JournalError> {
        let w = self
            .maintenance_write(id)?
            .ok_or(JournalError::IntentConflict)?;
        if w.state != WriteState::Prepared || sig == [0; 64] || expiry == 0 {
            return Err(JournalError::IntentConflict);
        }
        self.connection.execute(
            "UPDATE maintenance_writes SET signature=?,expiry=?,state=1 WHERE id=? AND state=0",
            params![sig.as_slice(), u64_blob(expiry).as_slice(), id.as_slice()],
        )?;
        Ok(())
    }
    pub(crate) fn finish_maintenance_write(
        &mut self,
        id: [u8; 32],
        state: WriteState,
    ) -> std::result::Result<(), JournalError> {
        let w = self
            .maintenance_write(id)?
            .ok_or(JournalError::IntentConflict)?;
        if w.state == state {
            return Ok(());
        }
        let (s, slot) = match (w.state, state) {
            (WriteState::Unknown, WriteState::Applied(s)) if s > 0 => (2, Some(u64_blob(s))),
            (WriteState::Unknown, WriteState::Rejected(s)) if s > 0 => (3, Some(u64_blob(s))),
            (WriteState::Prepared, WriteState::Cancelled) => (4, None),
            _ => return Err(JournalError::IntentConflict),
        };
        self.connection.execute(
            "UPDATE maintenance_writes SET state=?,slot=? WHERE id=?",
            params![s, slot.map(|b| b.to_vec()), id.as_slice()],
        )?;
        Ok(())
    }
    pub(super) fn validate_maintenance_writes(&self) -> std::result::Result<(), JournalError> {
        let mut q = self
            .connection
            .prepare("SELECT id FROM maintenance_writes")?;
        for id in q.query_map([], |r| r.get::<_, Vec<u8>>(0))? {
            self.maintenance_write(fixed(id?)?)?
                .ok_or(JournalError::CorruptState("missing maintenance write"))?;
        }
        if self.active_maintenance_write()?.is_some() {
            let active_epochs: bool = self.connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM funding_epochs WHERE completed=0)",
                [],
                |r| r.get(0),
            )?;
            if active_epochs
                || self.funding_checkpoint()?.is_none()
                || self.has_unresolved_funding()?
                || !self.nonterminal_operations()?.is_empty()
            {
                return Err(JournalError::CorruptState(
                    "overlapping maintenance mutation",
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/support/maintenance_writes.rs"]
mod tests;
