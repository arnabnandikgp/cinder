//! Accepted participant metadata and original-deposit hints, not another ledger.
use crate::{
    Error,
    model::*,
    wire::{Reader, Writer},
};
use cinder_kernel::identity::*;
use sha2::{Digest, Sha256};

/// Immutable private owner/recipient binding, retained only in protected history.
#[derive(Clone, PartialEq, Eq)]
pub struct Binding {
    /// Private deployment-scoped customer identity.
    pub account: AccountId,
    /// Owner Ed25519 wallet verification key.
    pub wallet: [u8; 32],
    /// Immutable owner token recipient.
    pub tokens: [u8; 32],
}
impl std::fmt::Debug for Binding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Binding([PRIVATE])")
    }
}
/// An owner-reported locator; no amount, credit, finality or signing authority.
#[derive(Clone, PartialEq, Eq)]
pub struct Notice {
    /// Private deployment-scoped customer identity.
    pub account: AccountId,
    /// Original nonzero owner deposit operation.
    pub operation: [u8; 32],
    /// Original signature; not evidence of execution.
    pub signature: [u8; 64],
}
impl std::fmt::Debug for Notice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Notice([PRIVATE])")
    }
}
/// Trusted registration controller, after owner/session/policy authentication.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Action {
    /// Add an empty participant after trusted owner authentication.
    Register(Binding),
    /// Retain an original deposit hint without posting money.
    Notice(Notice),
}

/// Deterministic identity; does not expose a financial book.
pub fn account(domain: Domain, wallet: [u8; 32]) -> Result<AccountId, Error> {
    AccountId::new(
        Sha256::digest(
            [
                b"CINDER-ACCOUNT-1\0".as_slice(),
                &domain.network.bytes(),
                &domain.deployment.bytes(),
                &wallet,
            ]
            .concat(),
        )
        .into(),
    )
    .map_err(|_| Error::Invalid)
}
impl State {
    /// Replay-derived registrations beyond immutable genesis customers.
    pub fn registrations(&self) -> &[Binding] {
        &self.registrations
    }
    /// Bounded immutable locator inventory, never evidence of payment.
    pub fn deposit_notices(&self) -> &[Notice] {
        &self.deposit_notices
    }
    pub(crate) fn onboard(&mut self, action: &Action) -> Result<(), ControlError> {
        match action {
            Action::Register(b) => {
                if self.frozen
                    || self.recovery.is_some()
                    || b.wallet == [0; 32]
                    || b.tokens == [0; 32]
                    || account(self.config.domain, b.wallet).ok() != Some(b.account)
                    || self.registrations.iter().any(|old| {
                        old.account == b.account || old.wallet == b.wallet || old.tokens == b.tokens
                    })
                {
                    return Err(ControlError::Invalid);
                }
                self.ledger
                    .register_customer(b.account)
                    .map_err(|_| ControlError::Invalid)?;
                self.registrations.push(b.clone());
            }
            Action::Notice(n) => {
                // Existing owners may report already-sent originals during
                // containment/recovery. This neither enrolls nor credits anyone.
                if self.deposit_notices.len() >= 128
                    || n.operation == [0; 32]
                    || n.signature == [0; 64]
                    || !self.registrations.iter().any(|b| b.account == n.account)
                    || self.deposit_notices.iter().any(|old| {
                        // Public signatures are not owned by the first reporter.
                        // Wrong-owner hints must not squat the rightful deposit.
                        old.account == n.account
                            && (old.signature == n.signature || old.operation == n.operation)
                    })
                {
                    return Err(ControlError::Invalid);
                }
                self.deposit_notices.push(n.clone());
            }
        }
        Ok(())
    }
}
pub(crate) fn encode(action: &Action, w: &mut Writer) {
    match action {
        Action::Register(b) => {
            w.byte(0);
            w.raw(&b.account.bytes());
            w.raw(&b.wallet);
            w.raw(&b.tokens);
        }
        Action::Notice(n) => {
            w.byte(1);
            w.raw(&n.account.bytes());
            w.raw(&n.operation);
            w.raw(&n.signature);
        }
    }
}
pub(crate) fn decode(r: &mut Reader<'_>) -> Result<Action, Error> {
    Ok(match r.byte()? {
        0 => Action::Register(Binding {
            account: AccountId::new(r.array()?).map_err(|_| Error::Invalid)?,
            wallet: r.array()?,
            tokens: r.array()?,
        }),
        1 => Action::Notice(Notice {
            account: AccountId::new(r.array()?).map_err(|_| Error::Invalid)?,
            operation: r.array()?,
            signature: r.array()?,
        }),
        _ => return Err(Error::Codec),
    })
}
