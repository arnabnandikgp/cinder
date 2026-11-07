//! Explicit internal working-collateral authority for ONE testnet demo workflow.
//! Policy is in encrypted configuration, not host-readable customer metadata.
//! P08 movement semantics remain; the explicit flat-only admission certificate
//! grants neither customer authentication nor trading readiness. No signature,
//! additional entitlement or external effect is created by accepting the grant.
use crate::{Error, chain_funding, customer_deposit};
use cinder_journal::{Backend, Journal, Protection, funds, model::*, orders};
use cinder_kernel::{
    amounts::QuoteAtoms,
    identity::*,
    ledger::{Config, Location, Owner, funds::Destination},
};
use cinder_pacifica::{
    execution::Gateway,
    funding::{Controller, demo},
};
use openssl::sha::sha256;
use serde::{Deserialize, Serialize};

const MARKER: &[u8] = b"CINDER-DEMO-ALLOCATION-1\0";

/// Governed private configuration, bound to actual loaded application/KMS context.
/// Presence is not live-run permission or customer authentication.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    /// Nonzero approval revision.
    pub revision: u64,
    /// Sole original finalized owner deposit eligible for this workflow.
    pub original: customer_deposit::Locator,
    /// Exact working collateral in quote atoms, not perp notional.
    pub amount: u64,
    /// Fixed Vault -> Broker request identity.
    pub release_request: [u8; 32],
    /// Distinct fixed Broker -> Venue request identity.
    pub deposit_request: [u8; 32],
    /// Current customer authority epoch, not a storage lease or Solana slot.
    pub authority_epoch: u64,
    /// Absolute logical dispatch expiry in milliseconds; never refreshed.
    pub expires_at: u64,
}
impl std::fmt::Debug for Policy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DemoAllocationPolicy([PRIVATE])")
    }
}
impl Policy {
    /// Structural limits only; no payment/authority follows from this check.
    pub fn validate(&self) -> Result<(), Error> {
        self.original.validate()?;
        if self.revision == 0
            || self.amount == 0
            || self.authority_epoch == 0
            || self.expires_at == 0
            || self.release_request == [0; 32]
            || self.deposit_request == [0; 32]
            || self.release_request == self.deposit_request
        {
            return Err(Error);
        }
        Ok(())
    }
}
/// Created from actual controllers and encrypted governed configuration only.
pub(crate) struct Authorization {
    policy: Policy,
    funding: [u8; 32],
    demo: [u8; 32],
    digest: [u8; 32],
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Step {
    Waiting,
    Accepted,
    Complete,
}
impl Authorization {
    pub(crate) fn bind(
        policy: Policy,
        config: &Config,
        funding: &Controller,
        gateway: &Gateway,
        demo: &demo::Policy,
        chain: &chain_funding::Loaded,
    ) -> Result<Self, Error> {
        policy.validate()?;
        if demo.initial_setup.is_none()
            || policy.amount > funding.route().maximum_movement
            || !chain.permits_demo_deposit(&policy.original)
            || !config
                .customers
                .iter()
                .any(|a| a.bytes() == policy.original.account)
        {
            return Err(Error);
        }
        let funding_hash = funding.release_commitment(config).map_err(|_| Error)?;
        let demo_hash = demo
            .release_commitment(funding, gateway)
            .map_err(|_| Error)?;
        let digest = sha256(
            &[
                b"CINDER-LOADED-DEMO-ALLOCATION-2\0".as_slice(),
                &funding_hash,
                &demo_hash,
                &chain.commitment(),
                &serde_cbor::to_vec(&policy).map_err(|_| Error)?,
            ]
            .concat(),
        );
        Ok(Self {
            policy,
            funding: funding_hash,
            demo: demo_hash,
            digest,
        })
    }
    pub(crate) fn digest(&self) -> [u8; 32] {
        self.digest
    }
    /// Measured funding may dispatch ONLY the two retained grant acceptances.
    /// A customer API intent with the same request ID is not this authority.
    pub(crate) fn permits<B: Backend, P: Protection>(
        &self,
        j: &Journal<B, P>,
        attempt: AttemptKey,
    ) -> Result<bool, Error> {
        let marker = self.marker()?;
        for deposit in [false, true] {
            let intent = self.intent(j.configuration(), deposit)?;
            if intent.request == attempt.request {
                return Ok(self
                    .retained(j, &intent, deposit, &marker)?
                    .is_some_and(|op| !op.terminal && op.attempt == Some(attempt)));
            }
        }
        Ok(false)
    }
    fn intent(&self, config: &Config, deposit: bool) -> Result<funds::Intent, Error> {
        Ok(funds::Intent {
            request: RequestKey {
                domain: config.domain,
                account: AccountId::new(self.policy.original.account).map_err(|_| Error)?,
                request: RequestId::new(if deposit {
                    self.policy.deposit_request
                } else {
                    self.policy.release_request
                })
                .map_err(|_| Error)?,
            },
            source: if deposit {
                Location::Broker
            } else {
                Location::Vault
            },
            destination: Destination::Location(if deposit {
                Location::Venue
            } else {
                Location::Broker
            }),
            net: QuoteAtoms::new(config.quote, i128::from(self.policy.amount)),
            maximum_fee: QuoteAtoms::new(config.quote, 0),
            fee_payer: Owner::House,
            allow_partial: false,
            policy: config.policy,
            authority_epoch: self.policy.authority_epoch,
            expires_at: self.policy.expires_at,
        })
    }
    fn marker(&self) -> Result<PrivateBytes, Error> {
        PrivateBytes::new(
            [
                MARKER,
                &self.digest,
                &serde_cbor::to_vec(&self.policy).map_err(|_| Error)?,
            ]
            .concat(),
        )
        .map_err(|_| Error)
    }
    // Stable once-only slots: changing policy, deposit or IDs cannot create a
    // second grant in the same journal. Full governed binding lives in evidence.
    fn commit(&self, domain: Domain, deposit: bool) -> Result<CommitId, Error> {
        CommitId::new(sha256(
            &[
                b"CINDER-DEMO-ALLOCATION-ACCEPT-1\0".as_slice(),
                &domain.network.bytes(),
                &domain.deployment.bytes(),
                &[u8::from(deposit)],
            ]
            .concat(),
        ))
        .map_err(|_| Error)
    }
    fn approval(intent: &funds::Intent) -> Result<orders::Approval, Error> {
        // Trusted INTERNAL governed port, not evidence of a wallet signature.
        Ok(orders::Approval {
            account: intent.request.account,
            authority_epoch: intent.authority_epoch,
            intent_hash: intent.digest().map_err(|_| Error)?,
        })
    }
    fn control<B: Backend, P: Protection>(
        &self,
        j: &Journal<B, P>,
        intent: &funds::Intent,
    ) -> Result<Control, Error> {
        let original_amount =
            customer_deposit::recorded_amount(j, &self.policy.original)?.ok_or(Error)?;
        let tx = j
            .transaction(self.policy.original.commit(j.configuration().domain)?)
            .ok_or(Error)?;
        let RecordKey::Economic(original) = &tx.inputs[0].event.as_ref().ok_or(Error)?.key else {
            return Err(Error);
        };
        Ok(Control::Funds(funds::Action::FlatAccept {
            intent: Box::new(intent.clone()),
            approval: Self::approval(intent)?,
            allocation: Box::new(funds::FlatAllocation {
                authorization: self.digest,
                original: original.clone(),
                original_amount,
                release: self.intent(j.configuration(), false)?,
                deposit: self.intent(j.configuration(), true)?,
            }),
        }))
    }
    fn retained<B: Backend, P: Protection>(
        &self,
        j: &Journal<B, P>,
        intent: &funds::Intent,
        deposit: bool,
        marker: &PrivateBytes,
    ) -> Result<Option<funds::Operation>, Error> {
        let id = self.commit(j.configuration().domain, deposit)?;
        let Some(tx) = j.transaction(id) else {
            return Ok(None);
        };
        if tx.controls != [self.control(j, intent)?]
            || tx.evidence != [marker.clone()]
            || !tx.inputs.is_empty()
            || !tx.order_observations.is_empty()
            || !tx.funds_observations.is_empty()
            || j.transaction_receipt(id).ok_or(Error)?.controls.is_some()
        {
            return Err(Error);
        }
        let op = j
            .state()
            .map_err(|_| Error)?
            .funds()
            .iter()
            .find(|o| o.intent.request == intent.request)
            .ok_or(Error)?;
        if op.intent != *intent || op.recovery || op.faulted {
            return Err(Error);
        }
        Ok(Some(op.clone()))
    }
    /// Accept at most the next fixed intent. No signing/I/O occurs. A persisted
    /// original keeps reconciliation-only behavior beyond expiry/rotation.
    pub(crate) fn accept_next<B: Backend, P: Protection>(
        &self,
        j: &mut Journal<B, P>,
        funding: &Controller,
        gateway: &Gateway,
        demo: &demo::Policy,
        at: u64,
    ) -> Result<Step, Error> {
        if funding
            .release_commitment(j.configuration())
            .map_err(|_| Error)?
            != self.funding
            || demo
                .release_commitment(funding, gateway)
                .map_err(|_| Error)?
                != self.demo
            || j.verified_state().map_err(|_| Error)?.logical_time() > at
        {
            return Err(Error);
        }
        let marker = self.marker()?;
        if j.transactions().any(|tx| {
            tx.evidence
                .iter()
                .any(|e| e.as_bytes().starts_with(MARKER) && e != &marker)
        }) {
            return Err(Error);
        }
        let release = self.intent(j.configuration(), false)?;
        let deposit = self.intent(j.configuration(), true)?;
        let first = self.retained(j, &release, false, &marker)?;
        let second = self.retained(j, &deposit, true, &marker)?;
        if j.state()
            .map_err(|_| Error)?
            .funds()
            .iter()
            .any(|o| o.intent.request != release.request && o.intent.request != deposit.request)
            || first.is_none() && second.is_some()
        {
            return Err(Error);
        }
        if let Some(op) = second {
            return if !op.terminal {
                Ok(Step::Waiting)
            } else if op.proof.is_none()
                && op.demo.as_ref().is_some_and(|d| d.amount == deposit.net)
            {
                Ok(Step::Complete)
            } else {
                Err(Error)
            };
        }
        let next = if let Some(op) = first {
            if !op.terminal {
                return Ok(Step::Waiting);
            }
            let proof = op.proof.as_ref().ok_or(Error)?;
            let movement = j
                .state()
                .map_err(|_| Error)?
                .ledger()
                .movements()
                .iter()
                .find(|m| Some(m.mandate.attempt) == op.attempt)
                .ok_or(Error)?;
            if op.demo.is_some()
                || proof.debit != release.net
                || proof.settled != release.net
                || proof.no_later_execution == [0; 32]
                || movement.faulted
                || movement.debit != release.net
                || movement.arrived != release.net
                || movement.settled != release.net
                || movement.returned.atoms() != 0
                || movement.impaired.atoms() != 0
                || movement.fees.atoms() != 0
            {
                return Err(Error);
            }
            deposit
        } else {
            if !j.state().map_err(|_| Error)?.funds().is_empty() {
                return Err(Error);
            }
            release
        };
        let state = j.state().map_err(|_| Error)?;
        if at >= self.policy.expires_at
            || state.frozen()
            || state.recovery_epoch().is_some()
            || state.authority_epoch(next.request.account) != Some(self.policy.authority_epoch)
        {
            return Err(Error);
        }
        let Some(original) = customer_deposit::recorded_amount(j, &self.policy.original)? else {
            return Ok(Step::Waiting);
        };
        if original.atoms() < i128::from(self.policy.amount)
            || state
                .ledger()
                .book(Owner::Customer(next.request.account))
                .map_err(|_| Error)?
                .cash()
                .atoms()
                < i128::from(self.policy.amount)
            || state
                .ledger()
                .book(Owner::House)
                .map_err(|_| Error)?
                .positions()
                .iter()
                .any(|p| p.quantity().lots() != 0)
            || j.configuration().customers.iter().any(|a| {
                state.ledger().book(Owner::Customer(*a)).map_or(true, |b| {
                    b.funding().atoms() != 0
                        || b.positions().iter().any(|p| p.quantity().lots() != 0)
                })
            })
        {
            return Err(Error);
        }
        if !funding
            .initial_demo_ingress_ready(j, gateway, demo, at)
            .map_err(|_| Error)?
        {
            return Ok(Step::Waiting);
        }
        let id = self.commit(j.configuration().domain, next.source == Location::Broker)?;
        let result = j
            .commit(Transaction {
                id,
                expected: j.head(),
                at,
                evidence: vec![marker],
                inputs: vec![],
                controls: vec![self.control(j, &next)?],
                order_observations: vec![],
                funds_observations: vec![],
            })
            .map_err(|_| Error)?;
        if result.receipt.controls.is_some() {
            return Err(Error);
        }
        Ok(Step::Accepted)
    }
}
