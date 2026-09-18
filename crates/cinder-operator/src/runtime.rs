//! Authenticated recovery and explicitly opted-in, bounded execution passes.
use crate::rpc::{data, string, unix_ms, Result, Rpc, RuntimeError};
use crate::transaction::{bytes, decode_ix, instruction_data, key, signature, Receipt};
use crate::{
    BoundedIntent, Journal, OrderIdentity, OrderKind, ReconciliationReport, RecoveryCoordinator,
};
use anchor_lang::{AccountDeserialize, Discriminator};
use cinder_common as cc;
use cinder_ledger::{Book, UserLedger};
use cinder_vault::Config;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use solana_keypair::Keypair;
use solana_signer::Signer;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    fs::File,
    path::{Path, PathBuf},
    rc::Rc,
};

const DELEGATION: &str = "DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh";

/// Only public deployment settings live in this file. RPC endpoints are read
/// from named environment variables; bearer tokens are obtained in memory.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfig {
    pub operator_keypair: PathBuf,
    pub pool_lock_directory: PathBuf,
    pub l1_rpc_env: String,
    pub qfs_rpc_env: String,
    pub phoenix_program: String,
    pub phoenix_global_config: String,
    pub phoenix_trader: String,
    pub phoenix_asset_map: String,
    pub phoenix_quote_mint: String,
    pub markets: Vec<MarketMapping>,
    pub global_trader_index: Vec<String>,
    pub active_trader_buffer: Vec<String>,
    /// Recovery can assess current health without a stress policy. Its absence
    /// can never authorize new venue risk; there are no trading defaults.
    #[serde(default)]
    pub solvency_policy: Option<crate::SolvencyPolicy>,
    /// Explicit IOC cost/notional policy; absent in recovery-only deployments.
    #[serde(default)]
    pub execution_policy: Option<crate::ExecutionPolicy>,
    #[serde(default)]
    pub maintenance_policy: Option<crate::MaintenancePolicy>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarketMapping {
    pub cinder_asset_id: u16,
    pub phoenix_asset_id: u32,
    pub symbol: String,
}
impl RuntimeConfig {
    pub fn read(path: &Path) -> Result<Self> {
        let file = File::open(path).map_err(|_| RuntimeError::Configuration)?;
        if file
            .metadata()
            .map_err(|_| RuntimeError::Configuration)?
            .len()
            > 64 * 1024
        {
            return Err(RuntimeError::Configuration);
        }
        serde_json::from_reader(file).map_err(|_| RuntimeError::Configuration)
    }
    pub(crate) fn validate(&self) -> Result<()> {
        use phoenix_rise_ix::constants::*;
        let p = key(&self.phoenix_program)?;
        let g = key(&self.phoenix_global_config)?;
        if !((p == PROD_PHOENIX_PROGRAM_ID && g == PROD_PHOENIX_GLOBAL_CONFIGURATION)
            || (p == BETA_PHOENIX_PROGRAM_ID && g == BETA_PHOENIX_GLOBAL_CONFIGURATION))
        {
            return Err(RuntimeError::Identity);
        }
        for value in [
            &self.phoenix_trader,
            &self.phoenix_asset_map,
            &self.phoenix_quote_mint,
        ] {
            key(value)?;
        }
        let mut ids = std::collections::BTreeSet::new();
        let mut native = std::collections::BTreeSet::new();
        if self.markets.is_empty()
            || self.markets.len() > 32
            || self.markets.iter().any(|m| {
                m.cinder_asset_id == 0
                    || m.symbol.is_empty()
                    || m.symbol.len() > 16
                    || !ids.insert(m.cinder_asset_id)
                    || !native.insert(m.phoenix_asset_id)
            })
        {
            return Err(RuntimeError::Configuration);
        }
        if let Some(policy) = &self.solvency_policy {
            policy.validate(&ids)?;
        }
        if let Some(policy) = &self.execution_policy {
            policy.validate(&ids)?;
        }
        if let Some(policy) = &self.maintenance_policy {
            policy.validate()?;
        }
        if self.global_trader_index.is_empty()
            || self.active_trader_buffer.is_empty()
            || self.global_trader_index.len() + self.active_trader_buffer.len() > 64
        {
            return Err(RuntimeError::Incomplete);
        }
        for value in self
            .global_trader_index
            .iter()
            .chain(&self.active_trader_buffer)
        {
            key(value)?;
        }
        for env in [&self.l1_rpc_env, &self.qfs_rpc_env] {
            if env.is_empty()
                || !env
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
            {
                return Err(RuntimeError::Configuration);
            }
        }
        Ok(())
    }
    pub(crate) fn market(&self, asset: u16) -> Result<&MarketMapping> {
        self.markets
            .iter()
            .find(|m| m.cinder_asset_id == asset)
            .ok_or(RuntimeError::Unsupported)
    }
}

pub(crate) struct Context {
    pub config: RuntimeConfig,
    pub signer: Keypair,
    pub l1: Rpc,
    pub qfs: Rpc,
    pub users: BTreeMap<[u8; 32], [u8; 32]>,
    pub execution_admission: Option<([u8; 32], crate::ExecutionBudget, crate::rise::RiseView)>,
    pub funding_checkpoint: Option<crate::FundingCheckpoint>,
}
pub(crate) type Shared = Rc<RefCell<Context>>;
pub(crate) type PrivateSnapshot = (Book, Vec<([u8; 32], UserLedger)>, u64);
pub(crate) fn ledger_id() -> [u8; 32] {
    cinder_ledger::ID.to_bytes()
}
pub(crate) fn vault_id() -> [u8; 32] {
    cinder_vault::ID.to_bytes()
}
pub(crate) fn pda(program: [u8; 32], seeds: &[&[u8]]) -> [u8; 32] {
    solana_pubkey::Pubkey::find_program_address(
        seeds,
        &solana_pubkey::Pubkey::new_from_array(program),
    )
    .0
    .to_bytes()
}
pub(crate) fn text(key: &[u8; 32]) -> String {
    bs58::encode(key).into_string()
}
pub(crate) fn config_key() -> [u8; 32] {
    pda(vault_id(), &[cc::SEED_CONFIG])
}
pub(crate) fn book_key() -> [u8; 32] {
    pda(ledger_id(), &[cc::SEED_BOOK])
}
pub(crate) fn fees_key() -> [u8; 32] {
    pda(ledger_id(), &[cc::SEED_FEES])
}
pub(crate) fn decode<T: AccountDeserialize>(row: &Value, owner: &str) -> Result<T> {
    T::try_deserialize(&mut data(row, owner)?.as_slice()).map_err(|_| RuntimeError::Decode)
}
impl Context {
    pub fn vault_config(&mut self) -> Result<Config> {
        let (_, rows) = self.l1.accounts(&[text(&config_key())], &self.signer)?;
        let cfg: Config = decode(&rows[0], &text(&vault_id()))?;
        if cfg.adapter.to_bytes() != self.signer.pubkey().to_bytes()
            || cfg.phoenix_trader.to_bytes() != bytes(&self.config.phoenix_trader)?
            || cfg.vault_authority.to_bytes() != pda(vault_id(), &[cc::SEED_VAULT_AUTHORITY])
        {
            return Err(RuntimeError::Identity);
        }
        Ok(cfg)
    }
    /// L1 retains the delegated account's public identity, not its current
    /// private balances. Use that identity to prove the QFS registry is whole.
    pub fn registry(&mut self) -> Result<BTreeMap<[u8; 32], [u8; 32]>> {
        let owner = text(&ledger_id());
        let mut users = BTreeMap::new();
        for program in [owner.as_str(), DELEGATION] {
            for row in self
                .l1
                .scan(program, UserLedger::DISCRIMINATOR, &self.signer)?
            {
                let ledger: UserLedger = decode(&row["account"], program)?;
                let address = bytes(string(&row["pubkey"])?)?;
                let user = ledger.user.to_bytes();
                if address != pda(ledger_id(), &[cc::SEED_USER, &user]) {
                    if program == DELEGATION {
                        continue;
                    }
                    return Err(RuntimeError::Identity);
                }
                if ledger.schema_version != cc::ACCOUNT_SCHEMA_VERSION
                    || users.insert(address, user).is_some()
                {
                    return Err(RuntimeError::Identity);
                }
            }
        }
        Ok(users)
    }
    pub fn private_ledgers(&mut self) -> Result<PrivateSnapshot> {
        let expected = self.registry()?;
        if expected != self.users {
            return Err(RuntimeError::Incomplete);
        }
        let scanned =
            self.qfs
                .scan(&text(&ledger_id()), UserLedger::DISCRIMINATOR, &self.signer)?;
        let mut visible = BTreeMap::new();
        for row in scanned {
            let ledger: UserLedger = decode(&row["account"], &text(&ledger_id()))?;
            let address = bytes(string(&row["pubkey"])?)?;
            if address != pda(ledger_id(), &[cc::SEED_USER, &ledger.user.to_bytes()])
                || visible.insert(address, ledger.user.to_bytes()).is_some()
            {
                return Err(RuntimeError::Identity);
            }
        }
        if visible != expected || expected.len() + 1 > 100 {
            return Err(RuntimeError::Incomplete);
        }
        let mut keys = vec![text(&book_key())];
        keys.extend(expected.keys().map(text));
        let (_, rows) = self.qfs.accounts(&keys, &self.signer)?;
        let observed = unix_ms();
        let book: Book = decode(&rows[0], &text(&ledger_id()))?;
        if book.schema_version != cc::ACCOUNT_SCHEMA_VERSION
            || book.residual_len as usize > book.residuals.len()
        {
            return Err(RuntimeError::Identity);
        }
        let mut ledgers = Vec::new();
        for ((address, user), row) in expected.iter().zip(rows.iter().skip(1)) {
            let ledger: UserLedger = decode(row, &text(&ledger_id()))?;
            validate_ledger(&ledger)?;
            if ledger.user.to_bytes() != *user {
                return Err(RuntimeError::Identity);
            }
            ledgers.push((*address, ledger));
        }
        if self.registry()? != expected {
            return Err(RuntimeError::Incomplete);
        }
        Ok((book, ledgers, observed))
    }
    pub fn ledger(&mut self, address: &[u8; 32]) -> Result<UserLedger> {
        let (_, rows) = self.qfs.accounts(&[text(address)], &self.signer)?;
        let ledger: UserLedger = decode(&rows[0], &text(&ledger_id()))?;
        validate_ledger(&ledger)?;
        if self.users.get(address) != Some(&ledger.user.to_bytes()) {
            return Err(RuntimeError::Identity);
        }
        Ok(ledger)
    }
    /// Most recent successful placement for each client OID. A full private
    /// compiled receipt, both required signers and exact account list are
    /// mandatory. Account bytes or a redacted status receipt cannot replace it.
    pub fn placements(
        &mut self,
        address: &[u8; 32],
        ledger: &UserLedger,
    ) -> Result<BTreeMap<[u8; 16], BoundedIntent>> {
        let user = ledger.user.to_bytes();
        let adapter = self.signer.pubkey().to_bytes();
        let mut latest = BTreeMap::new();
        let mut nonce = None;
        let mut liquidations = std::collections::BTreeSet::new();
        let history = self.qfs.history(&text(address), &self.signer)?;
        for row in history.iter().rev() {
            if !row["err"].is_null() {
                continue;
            }
            let sig = signature(string(&row["signature"])?)?;
            let value = self
                .qfs
                .transaction(&text_signature(&sig), &self.signer, false)?;
            if value.is_null() {
                return Err(RuntimeError::Incomplete);
            }
            let receipt = Receipt::decode(&value, &sig)?;
            if !receipt.succeeded {
                continue;
            }
            for ix in &receipt.instructions {
                if receipt.program(ix)? != ledger_id() {
                    continue;
                }
                let raw = instruction_data(ix)?;
                let placed = decode_ix::<cinder_ledger::instruction::PlaceOrder>(&raw)?;
                let liquidated = decode_ix::<cinder_ledger::instruction::LiquidateUser>(&raw)?;
                let bounded = decode_ix::<cinder_ledger::instruction::LiquidateUserBounded>(&raw)?;
                let liquidated = liquidated.or_else(|| {
                    bounded.map(|p| cinder_ledger::instruction::LiquidateUser {
                        asset_id: p.asset_id,
                        client_oid: p.client_oid,
                        limit_price_ticks: p.limit_price_ticks,
                        last_valid_slot: p.last_valid_slot,
                    })
                });
                if placed.is_none() && liquidated.is_none() {
                    continue;
                }
                if !receipt.requires_signature(&adapter) {
                    return Err(RuntimeError::Identity);
                }
                let (oid, asset, lots, bound, deadline, n, kind) = if let Some(p) = placed {
                    if receipt.account(ix, 0)? != user
                        || receipt.account(ix, 1)? != adapter
                        || receipt.account(ix, 2)? != config_key()
                        || receipt.account(ix, 3)? != book_key()
                        || receipt.account(ix, 4)? != *address
                        || !receipt.requires_signature(&user)
                    {
                        return Err(RuntimeError::Identity);
                    }
                    if nonce.is_some_and(|n| p.nonce != n) {
                        return Err(RuntimeError::Incomplete);
                    }
                    nonce = Some(p.nonce.checked_add(1).ok_or(RuntimeError::Decode)?);
                    (
                        p.client_oid,
                        p.asset_id,
                        p.lots_delta,
                        p.limit_price_ticks,
                        p.last_valid_slot,
                        p.nonce,
                        OrderKind::User,
                    )
                } else {
                    let p = liquidated.ok_or(RuntimeError::Decode)?;
                    if receipt.account(ix, 0)? != adapter
                        || receipt.account(ix, 1)? != config_key()
                        || receipt.account(ix, 2)? != *address
                        || receipt.account(ix, 3)? != book_key()
                    {
                        return Err(RuntimeError::Identity);
                    }
                    let n = nonce.ok_or(RuntimeError::Incomplete)?;
                    // Liquidation does not advance the user nonce. Reusing its
                    // OID at that nonce cannot be made restart-safe; reject it.
                    if !liquidations.insert((p.client_oid, n)) {
                        return Err(RuntimeError::Identity);
                    }
                    let pending = ledger
                        .open_oids
                        .iter()
                        .find(|o| o.client_oid == p.client_oid && o.state == cc::OID_LIQUIDATING);
                    let lots = pending.map(|o| o.lots_delta).unwrap_or(0);
                    (
                        p.client_oid,
                        p.asset_id,
                        lots,
                        p.limit_price_ticks,
                        p.last_valid_slot,
                        n,
                        OrderKind::Liquidation,
                    )
                };
                latest.insert(
                    oid,
                    BoundedIntent {
                        identity: OrderIdentity {
                            user_ledger: *address,
                            user_pubkey: user,
                            user_nonce: n,
                            client_oid: oid,
                            kind,
                        },
                        asset_id: asset,
                        requested_lots: lots,
                        limit_price_ticks: bound,
                        last_valid_slot: deadline,
                        post_fail_position_im_usdc: 0,
                        created_at_ms: 0,
                    },
                );
            }
        }
        if nonce.unwrap_or(0) != ledger.nonce {
            return Err(RuntimeError::Incomplete);
        }
        Ok(latest)
    }
}
pub(crate) fn text_signature(sig: &[u8; 64]) -> String {
    bs58::encode(sig).into_string()
}
pub(crate) fn validate_ledger(l: &UserLedger) -> Result<()> {
    if l.schema_version != cc::ACCOUNT_SCHEMA_VERSION
        || l.positions_len as usize > l.positions.len()
    {
        return Err(RuntimeError::Identity);
    }
    let active = l
        .open_oids
        .iter()
        .filter(|o| matches!(o.state, cc::OID_PENDING | cc::OID_LIQUIDATING) && o.lots_delta != 0)
        .count();
    if active != l.pending_oid_count as usize {
        return Err(RuntimeError::Identity);
    }
    let mut assets = std::collections::BTreeSet::new();
    if l.positions[..l.positions_len as usize]
        .iter()
        .any(|p| p.asset_id == 0 || p.lots == i64::MIN || !assets.insert(p.asset_id))
    {
        return Err(RuntimeError::Identity);
    }
    Ok(())
}
pub(crate) fn same_intent(a: &BoundedIntent, b: &BoundedIntent) -> bool {
    a.identity == b.identity
        && a.asset_id == b.asset_id
        && a.requested_lots == b.requested_lots
        && a.limit_price_ticks == b.limit_price_ticks
        && a.last_valid_slot == b.last_valid_slot
}

/// Pool-scoped lease is independent of the SQLite path. All local operators
/// MUST use the same private lock directory; cross-host fencing is not claimed.
pub struct OperatorRuntime {
    pub(crate) coordinator:
        RecoveryCoordinator<crate::venue::RiseRecovery, crate::ledger::QfsLedger>,
    pub(crate) context: Shared,
    _pool_lock: File,
    pub(crate) maintenance_snapshot_cache: Option<MaintenanceSnapshot>,
}
pub(crate) type MaintenanceSnapshot = (Book, Vec<([u8; 32], UserLedger)>, crate::rise::RiseView);
impl OperatorRuntime {
    pub(crate) fn maintenance_snapshot(&mut self) -> Result<MaintenanceSnapshot> {
        let mut c = self.context.borrow_mut();
        let (book, ledgers, _) = c.private_ledgers()?;
        let assets = c.config.markets.iter().map(|m| m.cinder_asset_id).collect();
        let view = crate::rise::load(&mut c, &assets)?;
        let (after, after_ledgers, _) = c.private_ledgers()?;
        if crate::ledger::book_fingerprint(&book)? != crate::ledger::book_fingerprint(&after)?
            || ledgers
                .iter()
                .map(|(a, l)| Ok((*a, crate::ledger::private_fingerprint(l)?)))
                .collect::<Result<Vec<_>>>()?
                != after_ledgers
                    .iter()
                    .map(|(a, l)| Ok((*a, crate::ledger::private_fingerprint(l)?)))
                    .collect::<Result<Vec<_>>>()?
        {
            return Err(RuntimeError::Stale);
        }
        Ok((book, ledgers, view))
    }
    /// Read-only whole-book assessment; does not complete a liquidation,
    /// authorize a heartbeat, or release ownership/entry gates.
    pub fn maintenance_health(&mut self) -> Result<crate::maintenance::MaintenanceHealth> {
        let (_, ledgers, view) = self.maintenance_snapshot()?;
        let ranked = crate::maintenance::scan_user_health(&view, &ledgers, unix_ms())?;
        Ok(crate::maintenance::MaintenanceHealth {
            users_scanned: ranked.len(),
            confirmed_positions_scanned: ranked
                .iter()
                .map(|h| {
                    h.confirmed_positions
                        .iter()
                        .filter(|(_, lots)| *lots != 0)
                        .count()
                })
                .sum(),
            users_below_mm: ranked.iter().filter(|h| h.below_mm()).count(),
            users_below_im: ranked
                .iter()
                .filter(|h| h.effective_equity_usdc < i128::from(h.initial_margin_usdc))
                .count(),
        })
    }
    /// Capture coherent rates/inventory for the maintenance WAL. Bootstrap and
    /// advancement are separate checked journal actions, not snapshot side effects.
    pub fn funding_checkpoint(&mut self) -> Result<crate::FundingCheckpoint> {
        let (book, ledgers, view) = self.maintenance_snapshot()?;
        crate::FundingCheckpoint::capture(&view, &book, &ledgers, unix_ms())
    }
    pub fn open(config: RuntimeConfig, journal_path: &Path) -> Result<Self> {
        Self::open_mode(config, journal_path, false)
    }
    /// Explicit opt-in. Recovery-only callers cannot accidentally dispatch.
    pub fn open_execution(config: RuntimeConfig, journal_path: &Path) -> Result<Self> {
        if config.execution_policy.is_none() || config.solvency_policy.is_none() {
            return Err(RuntimeError::Configuration);
        }
        Self::open_mode(config, journal_path, true)
    }
    fn open_mode(config: RuntimeConfig, journal_path: &Path, execution: bool) -> Result<Self> {
        config.validate()?;
        let signer = crate::load_signer(&config.operator_keypair)?;
        let mut hasher = Sha256::new();
        for identity in [
            ledger_id(),
            vault_id(),
            signer.pubkey().to_bytes(),
            bytes(&config.phoenix_program)?,
            bytes(&config.phoenix_global_config)?,
            bytes(&config.phoenix_trader)?,
            bytes(&config.phoenix_asset_map)?,
            bytes(&config.phoenix_quote_mint)?,
        ] {
            hasher.update(identity);
        }
        let mut mappings = config.markets.iter().collect::<Vec<_>>();
        mappings.sort_by_key(|m| m.cinder_asset_id);
        for m in mappings {
            hasher.update(m.cinder_asset_id.to_le_bytes());
            hasher.update(m.phoenix_asset_id.to_le_bytes());
            hasher.update([m.symbol.len() as u8]);
            hasher.update(m.symbol.as_bytes());
        }
        let binding: [u8; 32] = hasher.finalize().into();
        // The lease key depends only on the pooled trader and deployments,
        // not the operator credential or any user-supplied journal path.
        let mut hasher = Sha256::new();
        hasher.update(ledger_id());
        hasher.update(bytes(&config.phoenix_program)?);
        hasher.update(bytes(&config.phoenix_trader)?);
        let lease: [u8; 32] = hasher.finalize().into();
        let pool_lock = crate::journal::pool_lock(&config.pool_lock_directory, &lease)
            .map_err(|_| RuntimeError::Journal)?;
        let l1 = Rpc::new(
            &std::env::var(&config.l1_rpc_env).map_err(|_| RuntimeError::Configuration)?,
            false,
        )?;
        let qfs = Rpc::new(
            &std::env::var(&config.qfs_rpc_env).map_err(|_| RuntimeError::Configuration)?,
            true,
        )?;
        let mut context = Context {
            config,
            signer,
            l1,
            qfs,
            users: BTreeMap::new(),
            execution_admission: None,
            funding_checkpoint: None,
        };
        context.vault_config()?;
        let mut journal = Journal::open(journal_path).map_err(|_| RuntimeError::Journal)?;
        journal
            .bind_runtime(binding)
            .map_err(|_| RuntimeError::Journal)?;
        let users = context.registry()?;
        let old_users: BTreeMap<_, _> = journal
            .known_users()
            .map_err(|_| RuntimeError::Journal)?
            .into_iter()
            .collect();
        if old_users.iter().any(|(l, u)| users.get(l) != Some(u)) {
            return Err(RuntimeError::Incomplete);
        }
        if journal
            .funding_checkpoint()
            .map_err(|_| RuntimeError::Journal)?
            .is_none()
            || users == old_users
        {
            journal
                .remember_users(&users.iter().map(|(l, u)| (*l, *u)).collect::<Vec<_>>())
                .map_err(|_| RuntimeError::Journal)?;
        }
        context.users = users;
        context.funding_checkpoint = journal
            .funding_checkpoint()
            .map_err(|_| RuntimeError::Journal)?;
        let context = Rc::new(RefCell::new(context));
        let mut venue = crate::venue::RiseRecovery::new(context.clone());
        if execution {
            venue.enable_execution();
        }
        let ledger = crate::ledger::QfsLedger::new(context.clone());
        let mut runtime = Self {
            coordinator: RecoveryCoordinator::new(journal, venue, ledger),
            context,
            _pool_lock: pool_lock,
            maintenance_snapshot_cache: None,
        };
        runtime.incorporate_registry(&old_users)?;
        Ok(runtime)
    }
    /// One serialized startup/recovery pass. Discovery happens under both
    /// confirmed OPERATOR_DOWN gates, before any acknowledgement is sent.
    pub fn recover(&mut self) -> Result<ReconciliationReport> {
        crate::ledger::set_down(&mut self.context.borrow_mut(), true)?;
        // A service shutdown can leave its last signed heartbeat/root receipt
        // unresolved. Reconcile that exact write before discovering a new order;
        // journal preparation must never overlap an uncertain maintenance write.
        self.drain_maintenance_write()?;
        if self
            .journal()
            .has_unresolved_maintenance()
            .map_err(|_| RuntimeError::Journal)?
        {
            self.coordinator.maintenance_journal();
            return Ok(ReconciliationReport {
                entries_enabled: false,
                reason: Some(crate::HaltReason::UnresolvedMaintenance),
                unresolved_operations: self
                    .journal()
                    .nonterminal_operations()
                    .map_err(|_| RuntimeError::Journal)?
                    .len(),
            });
        }
        self.refresh_funding_inventory()?;
        let (_, ledgers, _) = self.context.borrow_mut().private_ledgers()?;
        let mut intents = Vec::new();
        for (address, ledger) in ledgers {
            // Complete account reconciliation still includes this user, but
            // placement history is needed only for an active pending order.
            if ledger.pending_oid_count == 0 {
                continue;
            }
            let placements = self.context.borrow_mut().placements(&address, &ledger)?;
            for oid in ledger.open_oids.iter().filter(|o| {
                matches!(o.state, cc::OID_PENDING | cc::OID_LIQUIDATING) && o.lots_delta != 0
            }) {
                let intent = placements
                    .get(&oid.client_oid)
                    .ok_or(RuntimeError::Incomplete)?
                    .clone();
                if intent.asset_id != oid.asset_id
                    || intent.requested_lots != oid.lots_delta
                    || intent.limit_price_ticks != oid.limit_price_ticks
                    || intent.last_valid_slot != oid.last_valid_slot
                {
                    return Err(RuntimeError::Identity);
                }
                intent.validate().map_err(|_| RuntimeError::Identity)?;
                intents.push(intent);
            }
        }
        for intent in intents {
            if let Some(existing) = self
                .coordinator
                .journal()
                .operation_by_identity(&intent.identity)
                .map_err(|_| RuntimeError::Journal)?
            {
                if !same_intent(&existing.intent, &intent) {
                    return Err(RuntimeError::Identity);
                }
            } else {
                self.coordinator
                    .prepare(intent)
                    .map_err(|_| RuntimeError::Journal)?;
            }
        }
        self.coordinator
            .recover_with_clock(unix_ms)
            .map_err(|_| RuntimeError::Journal)
    }
    /// Call before releasing ownership, including autonomous service shutdown;
    /// stopping the process must leave the entry/deposit gates closed.
    pub fn halt(&mut self) -> Result<()> {
        crate::ledger::set_down(&mut self.context.borrow_mut(), true)
    }
    pub fn journal(&self) -> &Journal {
        self.coordinator.journal()
    }
}
