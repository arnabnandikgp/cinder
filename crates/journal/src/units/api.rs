//! Trusted API-controller records under the one unit journal. This layer checks
//! ownership/atomicity, not the application wire signature. The API interpreter
//! must validate the retained wire and its typed header before using a grant.
use super::*;

/// Financial capture and administrative completion are different contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// Original intent retained, without holds, admission, quota use or exposure.
    Recorded,
    /// Owner's accepted grant; application semantics live in the retained wire.
    Grant,
    /// Owner's global revoke-all, atomically advancing all unit epochs once.
    Revoke,
}

/// Original qualified signed mutation. Construct only inside the trusted API
/// controller; raw caller data is not authorization to append this record.
#[derive(Clone, PartialEq, Eq)]
pub struct Record {
    /// Immutable execution unit.
    pub unit: UnitId,
    /// Globally unique owner/request identity.
    pub request: RequestKey,
    /// Signed economic policy of the unit.
    pub policy: PolicyVersion,
    /// V2 economic digest, not a foundation reservation commitment.
    pub digest: [u8; 32],
    /// Original authenticated global epoch, before a possible revoke effect.
    pub epoch: u64,
    /// Original authenticated signer. Only the owner can grant/revoke.
    pub signer: [u8; 32],
    /// Immutable loaded API/registry/limits commitment, pinned by first capture.
    pub contract: [u8; 32],
    /// Recorded-only financial or completed administrative effect.
    pub effect: Effect,
    /// Original canonical signed v2 request, max 1,024 bytes, retained privately.
    pub wire: model::PrivateBytes,
}
impl fmt::Debug for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiRecord([PRIVATE])")
    }
}

/// Immutable state-derived accepted record. Fields cannot be supplied to mutate
/// journal state; constructing a copy is not evidence of accepted history.
#[derive(Clone, PartialEq, Eq)]
pub struct Accepted {
    /// Original immutable record.
    pub record: Record,
    /// Original trusted journal capture time, not retry time.
    pub at: u64,
}
impl fmt::Debug for Accepted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiAccepted([PRIVATE])")
    }
}

impl State {
    pub(super) fn capture_api(&mut self, tx: &Transaction, record: &Record) -> Result<(), Error> {
        let owner = self
            .customers
            .iter()
            .find(|c| c.account == record.request.account)
            .ok_or(Error::Invalid)?;
        let descriptor = self
            .config
            .units
            .iter()
            .find(|u| u.id == record.unit)
            .ok_or(Error::Invalid)?;
        if record.request.domain != self.config.domain
            || record.policy != descriptor.ledger.policy
            || record.epoch == 0
            || record.epoch != owner.epoch
            || record.signer == [0; 32]
            || record.contract == [0; 32]
            || record.digest == [0; 32]
            || record.wire.as_bytes().is_empty()
            || record.wire.as_bytes().len() > 1024
        {
            return Err(Error::Invalid);
        }
        if self.api.len() >= wire::MAX_ITEMS {
            return Err(Error::Limit);
        }
        if self.api.iter().any(|a| a.record.request == record.request)
            || self.route(record.request).is_some()
            || self
                .api
                .first()
                .is_some_and(|a| a.record.contract != record.contract)
        {
            return Err(Error::Conflict);
        }
        if record.effect != Effect::Recorded && record.signer != owner.wallet {
            return Err(Error::Invalid);
        }
        if record.effect == Effect::Revoke {
            self.advance_authority(
                tx,
                record.request.account,
                record.epoch.checked_add(1).ok_or(Error::Limit)?,
            )?;
        }
        self.api.push(Accepted {
            record: record.clone(),
            at: tx.at,
        });
        Ok(())
    }
}
