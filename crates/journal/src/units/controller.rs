//! Selected component over the one existing shared writer, never a child store.
use super::*;
use crate::{
    Backend, Protection,
    controller::Controller,
    model::{Receipt as FinancialReceipt, State as FinancialState, Transaction as Proposal},
    read::Source,
};
use cinder_kernel::ledger::Owner;

/// Trusted in-enclave controller borrow. Cannot be constructed from raw host
/// state, clone a writer, retarget a unit or outlive its shared journal.
pub struct Selected<'a, B: Backend, P: Protection> {
    journal: &'a mut Journal<B, P>,
    unit: UnitId,
}
impl<B: Backend, P: Protection> fmt::Debug for Selected<'_, B, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SelectedUnit([PRIVATE])")
    }
}
impl<B: Backend, P: Protection> Journal<B, P> {
    /// Select an existing immutable unit for trusted native controllers. The
    /// selection itself is neither freshness, financial admission nor live permission.
    pub fn select(&mut self, unit: UnitId) -> Result<Selected<'_, B, P>, Error> {
        self.state()?.unit(unit).ok_or(Error::Invalid)?;
        Ok(Selected {
            journal: self,
            unit,
        })
    }
}
impl<B: Backend, P: Protection> crate::read::sealed::Sealed for Selected<'_, B, P> {}
impl<B: Backend, P: Protection> Source for Selected<'_, B, P> {
    fn configuration(&self) -> &Config {
        &self
            .journal
            .configuration()
            .units()
            .iter()
            .find(|d| d.id == self.unit)
            .expect("validated selection")
            .ledger
    }
    fn state(&self) -> Result<&FinancialState, Error> {
        self.journal.state()?.unit(self.unit).ok_or(Error::Invalid)
    }
    fn head(&self) -> Head {
        self.journal.head()
    }
    fn transactions_with_receipts(&self) -> impl Iterator<Item = (&Proposal, &FinancialReceipt)> {
        self.journal
            .transactions()
            .into_iter()
            .flatten()
            .filter_map(|(tx, result)| match (&tx.change, result) {
                (Change::Unit { unit, proposal }, Receipt::Unit { receipt, .. })
                    if *unit == self.unit =>
                {
                    Some((&**proposal, receipt))
                }
                (Change::Admit(a), Receipt::Admitted { unit, receipt }) if *unit == self.unit => {
                    Some((&a.proposal, receipt))
                }
                _ => None,
            })
    }
    fn customer_book_history(
        &self,
        account: AccountId,
    ) -> Result<Vec<crate::projection::BookChange>, Error> {
        self.state()?
            .ledger()
            .book(Owner::Customer(account))
            .map_err(|_| Error::Invalid)?;
        self.journal.book_history(self.unit, account)
    }
}
impl<B: Backend, P: Protection> Controller for Selected<'_, B, P> {
    fn shared_logical_time(&self) -> Result<u64, Error> {
        Ok(self.journal.state()?.logical_time())
    }
    fn recovery_cut(
        &mut self,
        expected: Head,
        check: EventKey,
    ) -> Result<crate::recovery::Cut, crate::recovery::CutError> {
        self.verified_state()
            .map_err(crate::recovery::CutError::Journal)?;
        if !self
            .journal
            .state()
            .map_err(crate::recovery::CutError::Journal)?
            .in_recovery()
        {
            return Err(crate::recovery::CutError::NotFrozen);
        }
        crate::recovery::cut(self, expected, check)
    }
    fn descriptor(&self) -> Option<&Descriptor> {
        self.journal
            .configuration()
            .units()
            .iter()
            .find(|d| d.id == self.unit)
    }
    fn verified_state(&mut self) -> Result<&FinancialState, Error> {
        self.journal
            .verified_state()?
            .unit(self.unit)
            .ok_or(Error::Invalid)
    }
    fn commit(&mut self, tx: Proposal) -> Result<crate::Committed, Error> {
        let deliveries = tx
            .controls
            .iter()
            .filter_map(|c| {
                if let model::Control::Expose(key) = c {
                    Some(*key)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let committed = self.journal.commit(Transaction::unit(self.unit, tx))?;
        let Receipt::Unit { receipt, .. } = committed.receipt else {
            return Err(Error::Invalid);
        };
        let mut exposures = Vec::new();
        if !committed.duplicate && receipt.controls.is_none() {
            for key in deliveries {
                let attempt = self
                    .state()?
                    .attempts()
                    .iter()
                    .find(|a| a.key == key && a.possibly_exposed)
                    .ok_or(Error::Invalid)?;
                exposures.push(crate::Exposure {
                    key,
                    message: attempt.message.clone(),
                });
            }
        }
        Ok(crate::Committed {
            head: committed.head,
            receipt,
            duplicate: committed.duplicate,
            exposures,
        })
    }
}
