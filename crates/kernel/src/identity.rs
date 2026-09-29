//! Separate internal identities, native event namespaces and replay comparisons.
//!
//! A request ID cannot silently be substituted for an exposed attempt ID:
//! ```compile_fail
//! use cinder_kernel::identity::{AttemptKey, RequestKey};
//! fn mix(request: RequestKey) -> AttemptKey {
//!     AttemptKey { request, attempt: request.request }
//! }
//! ```

use crate::Error;
use alloc::vec::Vec;

macro_rules! id {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $name([u8; 32]);
        impl $name {
            /// Construct from exact opaque internal bytes; zero is reserved.
            /// This neither authenticates the identity nor hashes a native ID.
            pub fn new(bytes: [u8; 32]) -> Result<Self, Error> {
                if bytes == [0; 32] {
                    return Err(Error::InvalidId);
                }
                Ok(Self(bytes))
            }
            /// Exact bytes, with no text or integer reinterpretation.
            pub fn bytes(self) -> [u8; 32] {
                self.0
            }
        }
    };
}

id!(
    NetworkId,
    "Configured network identity; no implied mainnet/testnet alias."
);
id!(
    DeploymentId,
    "Independent Cinder deployment, not just a venue's network."
);
id!(AccountId, "Private Cinder ledger owner identity.");
id!(VenueId, "Configured external venue identity.");
id!(
    VenueAccountId,
    "External physical account, distinct from a private ledger owner."
);
id!(
    MarketId,
    "Internal instrument identity; symbols alone are not interchangeable."
);
id!(
    AssetId,
    "Internal asset identity; quote valuation is not automatically a token."
);
id!(
    NamespaceId,
    "Qualified semantic event namespace, not REST versus WebSocket transport."
);
id!(
    RequestId,
    "User operation identity; one request may cause multiple attempts."
);
id!(
    AttemptId,
    "A distinct signed-capability exposure; never an economic fill ID."
);

macro_rules! version {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $name(u32);
        impl $name {
            /// Nonzero revision reference, not evidence it is approved/known.
            pub fn new(value: u32) -> Result<Self, Error> {
                if value == 0 {
                    return Err(Error::UnknownVersion);
                }
                Ok(Self(value))
            }
            /// Wire revision number.
            pub fn get(self) -> u32 {
                self.0
            }
            /// Require the revision selected by trusted configuration.
            pub fn require(self, expected: Self) -> Result<(), Error> {
                if self == expected {
                    Ok(())
                } else {
                    Err(Error::UnknownVersion)
                }
            }
        }
    };
}

version!(
    PrecisionVersion,
    "Explicit asset/market unit-definition revision."
);
version!(
    PolicyVersion,
    "Explicit economic policy revision, not an automatic approval."
);

/// Protocol encoding schema. Unknown wire versions are rejected, not guessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaVersion {
    /// Initial canonical primitive/key schema.
    V1,
}

/// Scope common to all replay and authorization domains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Domain {
    /// Configured network.
    pub network: NetworkId,
    /// Cinder deployment.
    pub deployment: DeploymentId,
}

/// A customer request. Its account is private, not the external omnibus account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestKey {
    /// Deployment scope.
    pub domain: Domain,
    /// Private owner.
    pub account: AccountId,
    /// Customer operation identity.
    pub request: RequestId,
}

/// A distinct attempt to expose a signed action for one customer operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttemptKey {
    /// Parent operation; reuse of its request ID does not merge attempts.
    pub request: RequestKey,
    /// Durable attempt identity.
    pub attempt: AttemptId,
}

/// Qualified physical source scope, independent of the message's delivery channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventScope {
    /// Deployment scope.
    pub domain: Domain,
    /// External venue.
    pub venue: VenueId,
    /// External physical account.
    pub account: VenueAccountId,
    /// Qualified semantic namespace shared across equivalent feeds.
    pub namespace: NamespaceId,
}

/// Exact nonempty native execution ID, at most 128 bytes; never silently truncated.
/// Adapter qualification must establish its uniqueness and normalization rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EconomicEventId(Vec<u8>);

impl EconomicEventId {
    /// Preserve bytes including leading zeros; no arbitrary native-ID hashing.
    pub fn new(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.is_empty() || bytes.len() > 128 {
            return Err(Error::InvalidId);
        }
        Ok(Self(bytes.to_vec()))
    }
    /// Exact native identifier bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.0
    }
}

/// One economic event, not an order ID or an observation timestamp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventKey {
    /// Physical source scope.
    pub scope: EventScope,
    /// Qualified economic execution identity.
    pub event: EconomicEventId,
    /// Qualified participant leg; distinct executions must not collapse.
    pub leg: u32,
}

impl EventKey {
    /// Reject a message routed to a different configured physical source.
    pub fn require_scope(&self, expected: EventScope) -> Result<(), Error> {
        if self.scope == expected {
            Ok(())
        } else {
            Err(Error::ScopeMismatch)
        }
    }
}

/// Typed key family; identical raw ID bytes do not alias another family's key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordKey {
    /// Customer operation.
    Request(RequestKey),
    /// Signed action attempt.
    Attempt(AttemptKey),
    /// Actual economic execution.
    Economic(EventKey),
}

impl RecordKey {
    /// Require the configured deployment before processing; not authorization.
    pub fn require_domain(&self, expected: Domain) -> Result<(), Error> {
        let domain = match self {
            Self::Request(key) => key.domain,
            Self::Attempt(key) => key.request.domain,
            Self::Economic(key) => key.scope.domain,
        };
        if domain == expected {
            Ok(())
        } else {
            Err(Error::ScopeMismatch)
        }
    }
}

/// Canonical economic payload plus version binding, not untrusted feed JSON.
/// P03/P05 define the concrete event schemas and atomic durable consumption.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Payload<'a> {
    /// Known schema, validated by the concrete event decoder.
    pub schema: SchemaVersion,
    /// Must separately match approved runtime configuration.
    pub policy: PolicyVersion,
    /// Canonical semantic bytes; delivery timestamps/transport metadata stay outside.
    pub bytes: &'a [u8],
}

/// Comparison only, not an in-memory or durable consumed-event registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Replay {
    /// Different keys may represent distinct effects; authorization still required.
    Distinct,
    /// Same key and exact canonical payload; do not post twice.
    Duplicate,
    /// Same key but different economics/version; contain rather than overwrite.
    Conflict,
}

/// Compare exact payload bytes rather than a non-cryptographic fingerprint.
/// No mutation occurs. P05 must atomically commit consumption with ledger changes.
pub fn classify_replay(
    existing: &RecordKey,
    old: Payload<'_>,
    incoming: &RecordKey,
    new: Payload<'_>,
) -> Replay {
    if existing != incoming {
        Replay::Distinct
    } else if old == new {
        Replay::Duplicate
    } else {
        Replay::Conflict
    }
}
