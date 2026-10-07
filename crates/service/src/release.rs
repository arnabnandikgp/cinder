//! Application-policy commitment derived from the actual loaded controllers.
//! This is ONE component of the runtime release manifest. Storage, witness,
//! clock, egress and key-release policy must also be bound by the assembled
//! runtime; this component alone must not be advertised as a complete release.
use crate::Error;
use cinder_api::{Admission, Service};
use cinder_kernel::ledger::{Config, Ledger};
use cinder_pacifica::{execution::Gateway, funding::Controller};
use openssl::sha::sha256;
use zeroize::Zeroizing;

/// Immutable commitment to the financial/API/venue policy actually loaded.
/// No caller-provided policy digest can substitute for these loaded objects.
pub struct ApplicationContract {
    digest: [u8; 32],
}
impl ApplicationContract {
    /// Derive from validated configuration and the actual initialized objects.
    /// Native capabilities are not activated and no journal mutation occurs.
    pub fn derive<A: Admission>(
        config: &Config,
        api: &Service<A>,
        execution: &Gateway,
        funding: &Controller,
    ) -> Result<Self, Error> {
        Ledger::new(config.clone()).map_err(|_| Error)?;
        let encoded =
            Zeroizing::new(cinder_journal::wire::encode_config(config).map_err(|_| Error)?);
        let config_hash = sha256(&encoded);
        let api_hash = api.release_commitment(config).map_err(|_| Error)?;
        let execution_hash = execution.release_commitment(config).map_err(|_| Error)?;
        let funding_hash = funding.release_commitment(config).map_err(|_| Error)?;
        let route = funding.route();
        if execution.agent_key() == route.broker
            || execution.agent_key() == route.funds
            || api.owner_bindings().iter().any(|owner| {
                !route.beneficiaries.iter().any(|beneficiary| {
                    beneficiary.account == owner.account.bytes()
                        && beneficiary.wallet == owner.wallet
                        && beneficiary.tokens == owner.tokens
                })
            })
        {
            return Err(Error);
        }
        Ok(Self {
            digest: sha256(
                &[
                    b"CINDER-LOADED-APPLICATION-1\0".as_slice(),
                    &config_hash,
                    &api_hash,
                    &execution_hash,
                    &funding_hash,
                ]
                .concat(),
            ),
        })
    }
    /// Public opaque component; never emits owner bindings or signer secrets.
    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }
    /// Extend the old component with the actual purpose-separated chain policy.
    /// Neither a caller-provided digest nor a parent assertion substitutes for it.
    pub fn with_chain(self, chain: &crate::chain_funding::Loaded) -> Self {
        Self {
            digest: sha256(
                &[
                    b"CINDER-LOADED-APPLICATION-2\0".as_slice(),
                    &self.digest,
                    &chain.commitment(),
                ]
                .concat(),
            ),
        }
    }
    /// Bind the consumed retained-pack policy as well as its governed manifest.
    pub fn with_history(self, policy: &crate::boot::HistoryPolicy) -> Result<Self, Error> {
        policy.limits()?;
        Ok(Self {
            digest: sha256(
                &[
                    b"CINDER-LOADED-HISTORY-1\0".as_slice(),
                    &self.digest,
                    &serde_cbor::to_vec(policy).map_err(|_| Error)?,
                ]
                .concat(),
            ),
        })
    }
}
