//! Original exact-batch ingress qualification, never raw balance-based credit.
use crate::{
    Error, chain_code, chain_receipt as receipt,
    chain_rpc::{Accounts, Finalized},
    customer_deposit::Locator,
};
use cinder_journal::{
    Backend, Journal, Protection,
    escrow::Batch,
    model::{CommitId, PrivateBytes},
};
use cinder_kernel::{amounts::QuoteAtoms, identity::*};
use cinder_pacifica::funding::{
    Controller,
    chain::{CustomerDeposit, Target},
};
use serde_json::json;

pub(crate) fn pre_sweep(
    accounts: &Accounts,
    route: &cinder_pacifica::funding::Route,
    s: &cinder_pacifica::funding::chain::Escrow,
    owner: [u8; 32],
    amount: u64,
    slot: u64,
) -> Result<(), Error> {
    let r = receipt::discriminated(
        receipt::get(accounts, s.route)?,
        route.program,
        "VenueRoute",
        106,
    )?;
    let e = receipt::discriminated(
        receipt::get(accounts, s.escrow)?,
        route.program,
        "VenueEscrow",
        83,
    )?;
    let b = receipt::discriminated(
        receipt::get(accounts, s.batch)?,
        route.program,
        "EscrowBatch",
        179,
    )?;
    if r[8] != 1
        || receipt::word(r, 10)? != route.config
        || receipt::word(r, 74)? != route.broker
        || e[8] != 1
        || receipt::word(e, 11)? != s.route
        || receipt::word(e, 43)? != owner
        || b[8] != 1
        || b[10] != 0
        || receipt::word(b, 11)? != s.escrow
        || receipt::word(b, 43)? != s.operation
        || receipt::number(b, 75)? != amount
        || receipt::number(e, 75)? < amount
        || receipt::number(b, 83)? != route.epoch
        || receipt::number(b, 91)?.to_string() != s.expires_at_slot
        || slot > receipt::number(b, 91)?
        || receipt::token(receipt::get(accounts, s.tokens)?, route.mint, s.escrow)?
            < receipt::number(e, 75)?
        || receipt::word(b, 99)? != [0; 32]
        || receipt::word(b, 131)? != [0; 32]
        || receipt::number(b, 163)? != 0
        || receipt::number(b, 171)? != 0
    {
        return Err(Error);
    }
    Ok(())
}

pub(crate) fn state<'a>(accounts: &'a Accounts, t: &Target) -> Result<&'a [u8], Error> {
    let s = t.sweep.as_ref().ok_or(Error)?;
    let route = receipt::discriminated(
        receipt::get(accounts, s.route)?,
        t.program,
        "VenueRoute",
        106,
    )?;
    let escrow = receipt::discriminated(
        receipt::get(accounts, s.escrow)?,
        t.program,
        "VenueEscrow",
        83,
    )?;
    let batch = receipt::discriminated(
        receipt::get(accounts, s.batch)?,
        t.program,
        "EscrowBatch",
        179,
    )?;
    if route[8] != 1
        || receipt::word(route, 10)? != t.config
        || receipt::word(route, 74)? != t.broker
        || escrow[8] != 1
        || receipt::word(escrow, 11)? != s.route
        || receipt::word(escrow, 43)? != t.customer
        || batch[8] != 1
        || batch[10] > 2
        || receipt::word(batch, 11)? != s.escrow
        || receipt::word(batch, 43)? != s.operation
        || receipt::number(batch, 75)? != t.amount
        || receipt::number(batch, 83)? != t.epoch
        || receipt::number(batch, 91)?.to_string() != s.expires_at_slot
        || batch[10] == 0 && receipt::number(escrow, 75)? < t.amount
        || receipt::token(receipt::get(accounts, s.tokens)?, t.mint, s.escrow)?
            < receipt::number(escrow, 75)?
    {
        return Err(Error);
    }
    Ok(batch)
}
/// Permanent full sweep, in addition to original transaction deltas/receipt.
pub(crate) fn swept(accounts: &Accounts, t: &Target, slot: u64) -> Result<(), Error> {
    let b = state(accounts, t)?;
    if b[10] != 1
        || receipt::word(b, 99)? != t.operation
        || receipt::word(b, 131)? != t.broker_tokens
        || Some(receipt::number(b, 163)?) != t.sequence.checked_add(1)
        || receipt::number(b, 171)? != slot
    {
        return Err(Error);
    }
    Ok(())
}
pub(crate) fn id(locator: &Locator, domain: Domain) -> Result<CommitId, Error> {
    locator.validate()?;
    let mut bytes = b"CINDER-ESCROW-DEPOSIT-2\0".to_vec();
    bytes.extend(domain.network.bytes());
    bytes.extend(domain.deployment.bytes());
    bytes.extend(locator.account);
    bytes.extend(locator.operation);
    bytes.extend(&locator.signature);
    CommitId::new(openssl::sha::sha256(&bytes)).map_err(|_| Error)
}
pub(crate) fn recorded<B: Backend, P: Protection>(
    j: &Journal<B, P>,
    locator: &Locator,
) -> Result<bool, Error> {
    let id = id(locator, j.configuration().domain)?;
    let Some(receipt) = j.transaction_receipt(id) else {
        return Ok(false);
    };
    if receipt.controls.is_some() {
        return Err(Error);
    }
    Ok(j.state().map_err(|_| Error)?.escrows().iter().any(|e| {
        e.batch.request.account.bytes() == locator.account
            && e.batch.operation == locator.operation
            && e.batch.signature.as_slice() == locator.signature
            && matches!(
                e.status,
                cinder_journal::escrow::Status::Refunded
                    | cinder_journal::escrow::Status::Funding
                    | cinder_journal::escrow::Status::Available
                    | cinder_journal::escrow::Status::Returned
            )
    }))
}
/// Same authenticated RPC/code boundary as legacy ingress, but no initial cash
/// posting. A refund already finalized when observed becomes terminal inventory.
#[allow(clippy::too_many_arguments)]
pub(crate) fn recognize<B: Backend, P: Protection>(
    j: &mut Journal<B, P>,
    funding: &Controller,
    locator: &Locator,
    deposit: CustomerDeposit,
    tx: Finalized,
    accounts: Accounts,
    deployment: &receipt::Deployment,
    code: chain_code::Verified,
    maximum_fee: u64,
) -> Result<(), Error> {
    if !crate::customer_deposit::valid_transaction(locator, &deposit, &tx, maximum_fee)? {
        return Err(Error);
    }
    let account = deposit.account();
    let t = deposit.into_target();
    let routing = funding.escrow_routing().ok_or(Error)?;
    if t.network != j.configuration().domain.network.bytes()
        || t.domain != j.configuration().domain.deployment.bytes()
        || deployment.program != t.program
        || accounts.network != t.network
        || accounts.slot < tx.slot
        || accounts.at < tx.at
        || t.sweep.as_ref().is_none_or(|s| s.route != routing.route)
    {
        return Err(Error);
    }
    code.matches(deployment, t.network, tx.slot, tx.at, accounts.at)?;
    receipt::mint(receipt::get(&accounts, t.mint)?, &t)?;
    receipt::token(receipt::get(&accounts, t.source)?, t.mint, t.source_owner)?;
    receipt::config(receipt::get(&accounts, t.config)?, &t)?;
    let customer = receipt::discriminated(
        receipt::get(&accounts, t.customer_counter.ok_or(Error)?)?,
        t.program,
        "CustomerCounter",
        97,
    )?;
    if receipt::word(customer, 8)? != t.config || receipt::word(customer, 40)? != t.customer {
        return Err(Error);
    }
    let s = t.sweep.as_ref().ok_or(Error)?;
    let route = receipt::discriminated(
        receipt::get(&accounts, s.route)?,
        t.program,
        "VenueRoute",
        106,
    )?;
    if receipt::word(route, 42)? != routing.venue {
        return Err(Error);
    }
    let b = state(&accounts, &t)?;
    if b[10] == 1 {
        // Our original inventory can be polled before the retained sweep is
        // reconciled. Do not create an intent or credit an external sweep.
        return if j.state().map_err(|_| Error)?.escrows().iter().any(|e| {
            e.batch.batch == s.batch
                && e.batch.signature == t.verified.signature
                && e.attempt.is_some()
        }) {
            Ok(())
        } else {
            Err(Error)
        };
    }
    let refunded = b[10] == 2;
    if receipt::word(b, 99)? != [0; 32]
        || receipt::number(b, 163)? != 0
        || !refunded && (receipt::word(b, 131)? != [0; 32] || receipt::number(b, 171)? != 0)
        || refunded && (receipt::word(b, 131)? == [0; 32] || receipt::number(b, 171)? < tx.slot)
    {
        return Err(Error);
    }
    let request = RequestKey {
        domain: j.configuration().domain,
        account: AccountId::new(account).map_err(|_| Error)?,
        request: RequestId::new(t.operation).map_err(|_| Error)?,
    };
    let batch = Batch {
        request,
        route: s.route,
        batch: s.batch,
        operation: t.operation,
        owner: t.customer,
        amount: QuoteAtoms::new(j.configuration().quote, i128::from(t.amount)),
        epoch: t.epoch,
        expires_at_slot: s.expires_at_slot.parse().map_err(|_| Error)?,
        signature: t.verified.signature,
    };
    let raw=PrivateBytes::new(serde_json::to_vec(&json!({"schema":"cinder-customer-escrow-deposit-v2","locator":locator,
        "transaction":serde_json::from_slice::<serde_json::Value>(tx.raw.as_bytes()).map_err(|_|Error)?,
        "accounts":serde_json::from_slice::<serde_json::Value>(accounts.raw.as_bytes()).map_err(|_|Error)?,"code":code.evidence(),"program_hash":deployment.hash,"actual_fee_lamports":tx.fee_lamports})).map_err(|_|Error)?).map_err(|_|Error)?;
    let at = accounts
        .at
        .max(j.state().map_err(|_| Error)?.logical_time());
    let commit = if refunded
        && j.transaction(id(locator, j.configuration().domain)?)
            .is_some()
    {
        let mut bytes = b"CINDER-ESCROW-REFUND-2\0".to_vec();
        bytes.extend(s.batch);
        CommitId::new(openssl::sha::sha256(&bytes)).map_err(|_| Error)?
    } else {
        id(locator, j.configuration().domain)?
    };
    funding
        .observe_escrow_inventory(j, commit, at, batch, refunded, raw)
        .map_err(|_| Error)
}
