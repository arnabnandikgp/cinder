//! Original owner-signed vault deposits, not seeded balances or venue credits.
use crate::{
    Error, chain_code, chain_receipt as receipt,
    chain_rpc::{Accounts, Finalized},
};
use cinder_journal::{Backend, Journal, Protection, model::*};
use cinder_kernel::{amounts::QuoteAtoms, identity::*, ledger::*};
use cinder_pacifica::funding::chain::CustomerDeposit;
use openssl::sha::sha256;
use serde::{Deserialize, Serialize};

/// Preconfigured test locator; financial facts come only from finalized evidence.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Locator {
    /// Registered private accounting identity.
    pub account: [u8; 32],
    /// Public P15 nonce; no amount or assumed completion.
    pub operation: [u8; 32],
    /// Original owner-signed transaction signature; never refreshed.
    pub signature: Vec<u8>,
}
impl Locator {
    /// Check exact identity bounds, not payment.
    pub fn validate(&self) -> Result<[u8; 64], Error> {
        if self.account == [0; 32]
            || self.operation == [0; 32]
            || self.signature.iter().all(|b| *b == 0)
        {
            return Err(Error);
        }
        self.signature.as_slice().try_into().map_err(|_| Error)
    }
    /// Stable domain-bound journal identity for repeated scheduling.
    pub fn commit(&self, domain: Domain) -> Result<CommitId, Error> {
        self.validate()?;
        CommitId::new(sha256(
            &[
                b"CINDER-CUSTOMER-DEPOSIT-1\0".as_slice(),
                &domain.network.bytes(),
                &domain.deployment.bytes(),
                &self.account,
                &self.operation,
                &self.signature,
            ]
            .concat(),
        ))
        .map_err(|_| Error)
    }
}
/// Verify a retained applied deposit, not merely the presence of its commit ID.
pub(crate) fn recorded<B: Backend, P: Protection>(
    journal: &Journal<B, P>,
    locator: &Locator,
) -> Result<bool, Error> {
    let id = locator.commit(journal.configuration().domain)?;
    let Some(tx) = journal.transaction(id) else {
        return Ok(false);
    };
    let receipt = journal.transaction_receipt(id).ok_or(Error)?;
    if !tx.controls.is_empty()
        || !tx.order_observations.is_empty()
        || !tx.funds_observations.is_empty()
        || tx.inputs.len() != 1
        || receipt.inputs.len() != 1
        || !matches!(
            receipt.inputs[0],
            InputResult::Normalized(
                cinder_kernel::ledger::evidence::Disposition::Applied
                    | cinder_kernel::ledger::evidence::Disposition::Duplicate
            )
        )
    {
        return Err(Error);
    }
    let input = &tx.inputs[0];
    let event = input.event.as_ref().ok_or(Error)?;
    let raw: serde_json::Value = serde_json::from_slice(input.raw.as_bytes()).map_err(|_| Error)?;
    if raw["schema"] != "cinder-customer-vault-deposit-v1"
        || raw["locator"] != serde_json::to_value(locator).map_err(|_| Error)?
        || event.policy != journal.configuration().policy
        || event.key
            != RecordKey::Economic(EventKey {
                scope: input.source,
                event: EconomicEventId::new(&locator.signature).map_err(|_| Error)?,
                leg: 0,
            })
        || !journal
            .configuration()
            .sources
            .iter()
            .any(|s| s.scope == input.source && s.location == Location::Vault)
        || !matches!(event.change, Change::Receipt { owner: Owner::Customer(a), location: Location::Vault, amount }
            if a.bytes() == locator.account && amount.unit() == journal.configuration().quote && amount.atoms() > 0)
    {
        return Err(Error);
    }
    Ok(true)
}
/// Immutable original-transaction eligibility, separate from current account/code
/// reads and persistence. Ineligibility cannot be repaired by polling this signature.
pub(crate) fn valid_transaction(
    locator: &Locator,
    deposit: &CustomerDeposit,
    tx: &Finalized,
    maximum_fee: u64,
) -> Result<bool, Error> {
    let signature = locator.validate()?;
    if maximum_fee == 0 {
        return Err(Error);
    }
    let t = deposit.target();
    if deposit.account() != locator.account
        || tx.network != t.network
        || tx.slot == 0
        || tx.wire.as_bytes() != t.verified.wire.as_bytes()
        || signature != t.verified.signature
        || tx.meta.get("err") != Some(&serde_json::Value::Null)
        || tx.fee_lamports > maximum_fee
        || t.operation != locator.operation
    {
        return Ok(false);
    }
    let deltas = || -> Result<bool, Error> {
        let pre = &tx.meta["preTokenBalances"];
        let post = &tx.meta["postTokenBalances"];
        Ok(
            receipt::balance(pre, t, t.source, t.source_owner)?.checked_sub(receipt::balance(
                post,
                t,
                t.source,
                t.source_owner,
            )?) == Some(t.amount)
                && receipt::balance(post, t, t.destination, t.destination_owner)?.checked_sub(
                    receipt::balance(pre, t, t.destination, t.destination_owner)?,
                ) == Some(t.amount),
        )
    };
    Ok(deltas().unwrap_or(false))
}
/// Post one authenticated P15 deposit through the existing normalization rules.
/// No native credit, API permission or wallet signing is created. Aggregate token
/// balances cannot substitute for original transaction deltas and immutable receipt.
#[allow(clippy::too_many_arguments)]
pub fn recognize<B: Backend, P: Protection>(
    journal: &mut Journal<B, P>,
    locator: &Locator,
    deposit: CustomerDeposit,
    tx: Finalized,
    accounts: Accounts,
    deployment: &receipt::Deployment,
    code: chain_code::Verified,
    maximum_fee: u64,
) -> Result<(), Error> {
    if !valid_transaction(locator, &deposit, &tx, maximum_fee)? {
        return Err(Error);
    }
    let account = deposit.account();
    let t = deposit.into_target();
    let domain = journal.configuration().domain;
    if account != locator.account
        || deployment.program != t.program
        || domain.network.bytes() != t.network
        || domain.deployment.bytes() != t.domain
        || tx.network != t.network
        || accounts.network != t.network
        || tx.slot == 0
        || accounts.slot < tx.slot
        || accounts.at < tx.at
        || tx.wire.as_bytes() != t.verified.wire.as_bytes()
        || locator.validate()? != t.verified.signature
        || tx.meta.get("err") != Some(&serde_json::Value::Null)
        || maximum_fee == 0
        || tx.fee_lamports > maximum_fee
        || t.operation != locator.operation
    {
        return Err(Error);
    }
    code.matches(deployment, t.network, tx.slot, tx.at, accounts.at)?;
    receipt::mint(receipt::get(&accounts, t.mint)?, &t)?;
    receipt::token(receipt::get(&accounts, t.source)?, t.mint, t.source_owner)?;
    receipt::token(
        receipt::get(&accounts, t.destination)?,
        t.mint,
        t.destination_owner,
    )?;
    receipt::config(receipt::get(&accounts, t.config)?, &t)?;
    let movement = receipt::discriminated(
        receipt::get(&accounts, t.receipt.ok_or(Error)?)?,
        t.program,
        "MovementReceipt",
        161,
    )?;
    if receipt::word(movement, 8)? != t.config
        || receipt::word(movement, 40)? != t.operation
        || receipt::number(movement, 72)? != t.epoch
        || movement[80] != 0
        || receipt::word(movement, 81)? != t.customer
        || receipt::word(movement, 113)? != t.vault
        || receipt::number(movement, 145)? != t.amount
        || receipt::number(movement, 153)? != 0
    {
        return Err(Error);
    }
    let counter = receipt::discriminated(
        receipt::get(&accounts, t.customer_counter.ok_or(Error)?)?,
        t.program,
        "CustomerCounter",
        97,
    )?;
    if receipt::word(counter, 8)? != t.config
        || receipt::word(counter, 40)? != t.customer
        || receipt::number(counter, 73)? < t.amount
    {
        return Err(Error);
    }
    let source = journal
        .configuration()
        .sources
        .iter()
        .filter(|s| s.location == Location::Vault)
        .collect::<Vec<_>>();
    if source.len() != 1 {
        return Err(Error);
    }
    let source = source[0].scope;
    let owner = AccountId::new(locator.account).map_err(|_| Error)?;
    if !journal.configuration().customers.contains(&owner) {
        return Err(Error);
    }
    if recorded(journal, locator)? {
        return Ok(());
    }
    let raw=PrivateBytes::new(serde_json::to_vec(&serde_json::json!({"schema":"cinder-customer-vault-deposit-v1","locator":locator,"transaction":serde_json::from_slice::<serde_json::Value>(tx.raw.as_bytes()).map_err(|_|Error)?,"accounts":serde_json::from_slice::<serde_json::Value>(accounts.raw.as_bytes()).map_err(|_|Error)?,"code":code.evidence(),"actual_fee_lamports":tx.fee_lamports})).map_err(|_|Error)?).map_err(|_|Error)?;
    let input = Input {
        source,
        // A signature lookup proves this receipt, not complete Vault history.
        source_cut: None,
        authority_epoch: t.epoch,
        observed_at: accounts.at,
        raw,
        event: Some(Event {
            key: RecordKey::Economic(EventKey {
                scope: source,
                event: EconomicEventId::new(&locator.signature).map_err(|_| Error)?,
                leg: 0,
            }),
            policy: journal.configuration().policy,
            change: Change::Receipt {
                owner: Owner::Customer(owner),
                location: Location::Vault,
                amount: QuoteAtoms::new(journal.configuration().quote, i128::from(t.amount)),
            },
        }),
    };
    let result = journal
        .commit(Transaction {
            id: locator.commit(domain)?,
            expected: journal.head(),
            // Preserve receive/effect times in evidence, but an unrelated API
            // commit may have advanced logical time while RPC I/O was in flight.
            at: accounts
                .at
                .max(journal.state().map_err(|_| Error)?.logical_time()),
            evidence: vec![],
            inputs: vec![input],
            order_observations: vec![],
            funds_observations: vec![],
            controls: vec![],
        })
        .map_err(|_| Error)?;
    if result.receipt.inputs.iter().any(|r| {
        !matches!(
            r,
            InputResult::Normalized(
                cinder_kernel::ledger::evidence::Disposition::Applied
                    | cinder_kernel::ledger::evidence::Disposition::Duplicate
            )
        )
    }) {
        return Err(Error);
    }
    Ok(())
}
