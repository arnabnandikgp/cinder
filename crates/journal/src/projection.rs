//! Read-only, replay-derived customer history. No projection is a second ledger,
//! a spend permit, a native timestamp or proof of external completeness.
use crate::{Backend, Error, Journal, Protection, model::State};
use cinder_kernel::{
    identity::AccountId,
    ledger::{Book, Config, Owner},
};

/// One committed change to this customer's financial book, including forced
/// liquidation/restoration and operator-ingested events, not merely API calls.
#[derive(Clone, PartialEq, Eq)]
pub struct BookChange {
    /// Qualified transaction observation/commit clock; not venue execution time.
    pub at: u64,
    /// Exact pre-transition customer entitlement.
    pub before: Book,
    /// Exact post-transition customer entitlement.
    pub after: Book,
}
impl std::fmt::Debug for BookChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BookChange([PRIVATE])")
    }
}
impl<B: Backend, P: Protection> Journal<B, P> {
    /// Read the immutable index derived at commit/replay, never a new durable
    /// ledger or a per-query financial replay. The caller must revalidate the witness and
    /// authorize the customer before accessing this private runtime projection.
    pub fn customer_book_history(&self, account: AccountId) -> Result<Vec<BookChange>, Error> {
        self.state()?
            .ledger()
            .book(Owner::Customer(account))
            .map_err(|_| Error::Invalid)?;
        Ok(self
            .history
            .iter()
            .flat_map(|r| &r.book_changes)
            .filter(|(id, _)| *id == account)
            .map(|(_, change)| change.clone())
            .collect())
    }
}
pub(crate) fn capture(
    before: &State,
    after: &State,
    config: &Config,
    at: u64,
) -> Result<Vec<(AccountId, BookChange)>, Error> {
    let mut changes = Vec::new();
    for id in &config.customers {
        let a = before
            .ledger()
            .book(Owner::Customer(*id))
            .map_err(|_| Error::Invalid)?;
        let b = after
            .ledger()
            .book(Owner::Customer(*id))
            .map_err(|_| Error::Invalid)?;
        if a != b {
            changes.push((
                *id,
                BookChange {
                    at,
                    before: a.clone(),
                    after: b.clone(),
                },
            ));
        }
    }
    Ok(changes)
}
