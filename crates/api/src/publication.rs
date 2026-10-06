//! Independent fresh reads reuse the writer's authorization and projection.
use crate::{
    Admission, ConfidentialChannel, Error, PositionView, Response, Service, View, authorization,
    records::Records,
    wire::{Command, Request},
};
use cinder_journal::{
    model::{PrivateBytes, Resource},
    read::{Source, Verified},
};
use cinder_kernel::ledger::{Config, Owner};

pub(crate) fn is_read(command: &Command) -> bool {
    matches!(
        command,
        Command::Read(_) | Command::View | Command::Operation(_)
    )
}
impl<A: Admission> Service<A> {
    /// Validate the signed confidential read envelope before remote witness I/O.
    /// This is NOT authority to return state: a verified ticket is still required.
    pub fn authenticate_read(
        &self,
        config: &Config,
        channel: &impl ConfidentialChannel,
        bytes: &PrivateBytes,
        now: u64,
    ) -> Result<(), Error> {
        let req = Request::decode(bytes.as_bytes())?;
        if !is_read(&req.command) {
            return Err(Error::Invalid);
        }
        authorization::envelope(&self.contract, config, channel, &req, now)?;
        Ok(())
    }
    /// Prepare a private response from this request's independently verified view.
    /// The runtime must still perform the final generation/time release gate.
    pub fn handle_read(
        &self,
        source: &Verified,
        channel: &impl ConfidentialChannel,
        bytes: &PrivateBytes,
        now: u64,
    ) -> Result<Response, Error> {
        let (req, owner, records) = self.authorized_read(source, channel, bytes, now)?;
        self.project_read(source, &req, &records, owner, now)
    }
    /// Recheck expiry, epoch and grant immediately before ticket release. Uses
    /// the SAME rules as normal commands; no separately cached read permission.
    pub fn revalidate_read(
        &self,
        source: &Verified,
        channel: &impl ConfidentialChannel,
        bytes: &PrivateBytes,
        now: u64,
    ) -> Result<(), Error> {
        self.authorized_read(source, channel, bytes, now)
            .map(|_| ())
    }
    fn authorized_read(
        &self,
        source: &Verified,
        channel: &impl ConfidentialChannel,
        bytes: &PrivateBytes,
        now: u64,
    ) -> Result<(Request, bool, std::sync::Arc<Records>), Error> {
        if source.binding().api != self.release_commitment(source.configuration())?
            || source.binding().stream.domain != source.configuration().domain
        {
            return Err(Error::Unavailable);
        }
        let req = Request::decode(bytes.as_bytes())?;
        if !is_read(&req.command) {
            return Err(Error::Invalid);
        }
        let owner =
            authorization::envelope(&self.contract, source.configuration(), channel, &req, now)?;
        authorization::authority(source.state().map_err(|_| Error::Unavailable)?, &req, now)?;
        let records = self.records(source)?;
        let is_owner = req.signer == owner.wallet;
        if !is_owner {
            authorization::agent(&req, &records, now)?;
        }
        Ok((req, is_owner, records))
    }
    pub(crate) fn project_read(
        &self,
        source: &impl Source,
        req: &Request,
        records: &Records,
        is_owner: bool,
        now: u64,
    ) -> Result<Response, Error> {
        let state = source.state().map_err(|_| Error::Unavailable)?;
        match &req.command {
            Command::Read(query) => self
                .read(source, req, records, *query, is_owner, now)
                .map(Response::Read),
            Command::View => {
                let book = state
                    .ledger()
                    .book(Owner::Customer(req.account))
                    .map_err(|_| Error::Unavailable)?;
                Ok(Response::View(View {
                    epoch: req.epoch,
                    cash: book.cash().atoms(),
                    funding: book.funding().atoms(),
                    held: state
                        .reserved(Resource::Customer(req.account))
                        .map_err(|_| Error::Unavailable)?
                        .atoms(),
                    positions: book
                        .positions()
                        .iter()
                        .map(|p| PositionView {
                            market: p.quantity().unit(),
                            lots: p.quantity().lots(),
                            basis: p.basis().atoms(),
                        })
                        .collect(),
                    operations: records
                        .iter()
                        .filter(|r| r.request.account == req.account)
                        .map(|r| r.request.id)
                        .collect(),
                }))
            }
            Command::Operation(id) => {
                let record = records.find(req.account, *id).ok_or(Error::NotFound)?;
                self.receipt(state, record).map(Response::Receipt)
            }
            _ => Err(Error::Invalid),
        }
    }
}
