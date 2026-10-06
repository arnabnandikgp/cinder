//! Immutable interpretation of retained accepted history, not financial state or
//! freshness authority. Indexes never retain an authorization result.
use crate::{
    Admission, Error, INIT, RECORD, Record, Service,
    wire::{Command, Grant, Request},
};
use cinder_journal::{Backend, Head, Journal, Protection};
use cinder_kernel::identity::{AccountId, RequestId};
use std::{collections::BTreeMap, ops::Deref, sync::Arc};

type RequestKey = ([u8; 32], [u8; 32]);
type GrantKey = ([u8; 32], u64, [u8; 32]);

pub(crate) struct Records {
    head: Head,
    fingerprint: [u8; 32],
    ordered: Vec<Record>,
    requests: BTreeMap<RequestKey, usize>,
    grants: BTreeMap<GrantKey, usize>,
}
impl Deref for Records {
    type Target = [Record];
    fn deref(&self) -> &Self::Target {
        &self.ordered
    }
}
impl Records {
    pub(crate) fn find(&self, account: AccountId, id: RequestId) -> Option<&Record> {
        self.requests
            .get(&(account.bytes(), id.bytes()))
            .map(|i| &self.ordered[*i])
    }
    pub(crate) fn grant(&self, req: &Request) -> Option<&Grant> {
        let i = self
            .grants
            .get(&(req.account.bytes(), req.epoch, req.signer))?;
        match &self.ordered[*i].request.command {
            Command::Grant(g) => Some(g),
            _ => None,
        }
    }
    fn build<B: Backend, P: Protection>(
        journal: &Journal<B, P>,
        fingerprint: [u8; 32],
    ) -> Result<Self, Error> {
        let mut installed = false;
        let mut result = Self {
            head: journal.head(),
            fingerprint,
            ordered: Vec::new(),
            requests: BTreeMap::new(),
            grants: BTreeMap::new(),
        };
        let marker = [INIT, &fingerprint].concat();
        for (tx, receipt) in journal.transactions_with_receipts() {
            let accepted = receipt.controls.is_none();
            for e in &tx.evidence {
                if e.as_bytes().starts_with(INIT) {
                    if installed || e.as_bytes() != marker || !accepted {
                        return Err(Error::Unavailable);
                    }
                    installed = true;
                } else if let Some(tail) = e.as_bytes().strip_prefix(RECORD) {
                    if !installed || tail.get(..32) != Some(fingerprint.as_slice()) {
                        return Err(Error::Unavailable);
                    }
                    let request = Request::decode(tail.get(32..).ok_or(Error::Unavailable)?)
                        .map_err(|_| Error::Unavailable)?;
                    let i = result.ordered.len();
                    if result
                        .requests
                        .insert((request.account.bytes(), request.id.bytes()), i)
                        .is_some()
                    {
                        return Err(Error::Unavailable);
                    }
                    if accepted && let Command::Grant(g) = &request.command {
                        // Preserve the original first-accepted-grant semantics.
                        result
                            .grants
                            .entry((request.account.bytes(), request.epoch, g.key))
                            .or_insert(i);
                    }
                    result.ordered.push(Record {
                        request,
                        accepted,
                        at: tx.at,
                    });
                }
            }
        }
        if !installed {
            return Err(Error::Unavailable);
        }
        Ok(result)
    }
}

impl<A: Admission> Service<A> {
    pub(crate) fn records<B: Backend, P: Protection>(
        &self,
        journal: &Journal<B, P>,
    ) -> Result<Arc<Records>, Error> {
        // Even an exact cached head cannot reopen a poisoned journal. The handler
        // additionally performs verified_state BEFORE calling this interpreter.
        journal.state().map_err(|_| Error::Unavailable)?;
        let fingerprint = self.fingerprint(
            journal.configuration().domain,
            journal.configuration().policy,
        );
        if let Some(index) = self
            .records
            .lock()
            .map_err(|_| Error::Unavailable)?
            .as_ref()
            && index.head == journal.head()
            && index.fingerprint == fingerprint
        {
            return Ok(Arc::clone(index));
        }
        // Parsing/building never holds the index latch. This snapshot owns exactly
        // the supplied immutable journal history, even if another caller installs
        // an index for a different head before we publish this derived one.
        let index = Arc::new(Records::build(journal, fingerprint)?);
        let previous = self
            .records
            .lock()
            .map_err(|_| Error::Unavailable)?
            .replace(Arc::clone(&index));
        // The last reference may own a full retained index: deallocate it outside
        // the short pointer latch, not while another interpreter waits on it.
        drop(previous);
        Ok(index)
    }
}
