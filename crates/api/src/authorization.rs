//! Pure checks shared by mutation and read interpretation. Callers must qualify
//! time and accepted-state freshness separately; these functions do no I/O.
use crate::{
    ConfidentialChannel, Contract, Error, OwnerBinding,
    records::Records,
    wire::{self, Command, Request},
};
use cinder_journal::model::State;
use cinder_kernel::ledger::Config;
use ed25519_dalek::{Signature, VerifyingKey};

pub(crate) fn envelope<'a>(
    contract: &'a Contract,
    config: &Config,
    channel: &impl ConfidentialChannel,
    req: &Request,
    now: u64,
) -> Result<&'a OwnerBinding, Error> {
    let owner = contract
        .owners
        .iter()
        .find(|b| b.account == req.account)
        .ok_or(Error::Unauthorized)?;
    if channel.domain() != req.domain
        || req.domain != config.domain
        || req.policy != config.policy
        || channel.binding() == [0; 32]
        || req.session != channel.binding()
        || now >= channel.expires_at()
        || now >= req.expires_at
        || req.expires_at > channel.expires_at()
        || req.expires_at.saturating_sub(now) > contract.maximum_auth_lifetime
    {
        return Err(Error::Unauthorized);
    }
    let key = VerifyingKey::from_bytes(&req.signer).map_err(|_| Error::Unauthorized)?;
    key.verify_strict(&req.message(), &Signature::from_bytes(&req.signature))
        .map_err(|_| Error::Unauthorized)?;
    Ok(owner)
}

pub(crate) fn authority(state: &State, req: &Request, now: u64) -> Result<(), Error> {
    if now < state.logical_time() {
        return Err(Error::Unavailable);
    }
    if req.epoch == 0 || state.authority_epoch(req.account) != Some(req.epoch) {
        return Err(Error::Unauthorized);
    }
    Ok(())
}

pub(crate) fn agent(req: &Request, records: &Records, now: u64) -> Result<(), Error> {
    let grant = records.grant(req).ok_or(Error::Unauthorized)?;
    if now >= grant.expires_at {
        return Err(Error::Unauthorized);
    }
    match &req.command {
        Command::View | Command::Operation(_) | Command::Read(_)
            if grant.methods & wire::READ != 0 =>
        {
            Ok(())
        }
        Command::Order {
            market,
            lots,
            fee,
            good_until,
            ..
        } if grant.methods & wire::TRADE != 0
            && *market == grant.market
            && lots.unsigned_abs() <= grant.maximum_lots
            && *fee >= 0
            && *fee <= grant.maximum_fee
            && *good_until <= grant.expires_at =>
        {
            Ok(())
        }
        Command::Cancel { good_until, .. }
            if grant.methods & wire::CANCEL != 0 && *good_until <= grant.expires_at =>
        {
            Ok(())
        }
        _ => Err(Error::Unauthorized),
    }
}
