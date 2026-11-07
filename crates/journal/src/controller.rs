//! Sealed financial-controller view of either the shipping journal or one
//! explicitly selected component of the same shared-unit accepted history.
use crate::{
    Backend, Committed, Error, Journal, Protection,
    model::{CommitId, Receipt, State, Transaction},
    read::Source,
};

/// Existing native controllers may use this boundary without owning storage or
/// creating a second journal. Source is sealed: a host projection cannot supply
/// state, freshness, commits or one-shot exposure deliveries.
pub trait Controller: Source {
    /// Physical registry binding for a selected shared component. Legacy
    /// controllers return None; this never invents an implicit execution unit.
    fn descriptor(&self) -> Option<&crate::units::Descriptor> {
        None
    }
    /// Recheck the actual shared backend/witness before dependent work.
    fn verified_state(&mut self) -> Result<&State, Error>;
    /// Accepted deployment chronology, not a fresh risk/price observation. A
    /// selected component can lag this clock when another unit commits.
    fn shared_logical_time(&self) -> Result<u64, Error> {
        Ok(self.state()?.logical_time())
    }
    /// Commit against the exact common head. Unknown outcomes never deliver an
    /// exposure; exact duplicate commits return no second local delivery.
    fn commit(&mut self, transaction: Transaction) -> Result<Committed, Error>;
    /// Fresh advisory control preview under the same sealed state. It neither
    /// accepts history nor returns an exposure; commit must revalidate normally.
    fn preview(&mut self, transaction: &Transaction) -> Result<Receipt, Error> {
        if transaction.expected != self.head() {
            return Err(Error::Stale);
        }
        Ok(self.verified_state()?.advance(transaction)?.1)
    }
    /// Settled projection only, never root activation or native-key fencing.
    fn recovery_cut(
        &mut self,
        expected: crate::Head,
        check: cinder_kernel::identity::EventKey,
    ) -> Result<crate::recovery::Cut, crate::recovery::CutError>
    where
        Self: Sized,
    {
        self.verified_state()
            .map_err(crate::recovery::CutError::Journal)?;
        crate::recovery::cut(self, expected, check)
    }
    /// Retained component provenance, not independent freshness/authority.
    fn transactions(&self) -> impl Iterator<Item = &Transaction> {
        self.transactions_with_receipts().map(|(t, _)| t)
    }
    /// Same-component original only; another unit's ID cannot supply a plan.
    fn transaction(&self, id: CommitId) -> Option<&Transaction> {
        self.transactions().find(|t| t.id == id)
    }
    /// Actual all-or-none controller result, including a refused proposal.
    fn transaction_receipt(&self, id: CommitId) -> Option<&Receipt> {
        self.transactions_with_receipts()
            .find(|(t, _)| t.id == id)
            .map(|(_, r)| r)
    }
}
impl<B: Backend, P: Protection> Controller for Journal<B, P> {
    fn verified_state(&mut self) -> Result<&State, Error> {
        Journal::verified_state(self)
    }
    fn commit(&mut self, tx: Transaction) -> Result<Committed, Error> {
        Journal::commit(self, tx)
    }
}
