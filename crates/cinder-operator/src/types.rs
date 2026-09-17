use crate::{ClientOid, PubkeyBytes};
use sha2::{Digest, Sha256};

/// Current deterministic Phoenix client-order-ID domain separator.
pub const VENUE_OID_DOMAIN: &[u8] = b"cinder:phoenix:v1";

pub type OperationId = [u8; 32];
pub type VenueOid = [u8; 16];

/// Persisted before sending an ER acknowledgement. A missing RPC response
/// never loses the signature needed to find its receipt after a restart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedAck {
    pub signature: [u8; 64],
    pub last_valid_block_height: u64,
    pub observed_ledger_nonce: u64,
}

/// The private operation identity.  `client_oid` is deliberately not global:
/// this full tuple is the local idempotency key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OrderIdentity {
    pub user_ledger: PubkeyBytes,
    pub user_pubkey: PubkeyBytes,
    pub user_nonce: u64,
    pub client_oid: ClientOid,
    pub kind: OrderKind,
}

/// An order initiated by a user or by the liquidation flow.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i64)]
pub enum OrderKind {
    User = 1,
    Liquidation = 2,
}

impl OrderKind {
    pub(crate) fn from_db(value: i64) -> Result<Self, &'static str> {
        match value {
            1 => Ok(Self::User),
            2 => Ok(Self::Liquidation),
            _ => Err("unknown order kind"),
        }
    }
}

/// Immutable values that are committed before any venue submission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundedIntent {
    pub identity: OrderIdentity,
    pub asset_id: u16,
    pub requested_lots: i64,
    pub limit_price_ticks: u64,
    pub last_valid_slot: u64,
    pub post_fail_position_im_usdc: u64,
    pub created_at_ms: u64,
}

impl BoundedIntent {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.asset_id == 0 {
            return Err("asset_id must be nonzero");
        }
        if self.requested_lots == 0 || self.requested_lots == i64::MIN {
            return Err("requested_lots must be nonzero and representable as an absolute value");
        }
        if self.limit_price_ticks == 0 {
            return Err("limit_price_ticks must be nonzero");
        }
        if self.last_valid_slot == 0 {
            return Err("last_valid_slot must be nonzero");
        }
        Ok(())
    }
}

/// All deterministic venue-ID material persisted alongside an operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VenueIdentity {
    pub venue_oid: VenueOid,
    pub full_hash: OperationId,
    pub preimage: Vec<u8>,
}

/// Derive Phoenix's native 16-byte/u128 client-order ID.  The bytes are used
/// directly, rather than formatting through a decimal `u128`, so the on-wire
/// representation stays exactly reproducible.
pub fn derive_venue_identity(identity: &OrderIdentity) -> VenueIdentity {
    let mut preimage = Vec::with_capacity(
        VENUE_OID_DOMAIN.len() + identity.user_pubkey.len() + 8 + identity.client_oid.len(),
    );
    preimage.extend_from_slice(VENUE_OID_DOMAIN);
    preimage.extend_from_slice(&identity.user_pubkey);
    preimage.extend_from_slice(&identity.user_nonce.to_le_bytes());
    preimage.extend_from_slice(&identity.client_oid);
    let full_hash: OperationId = Sha256::digest(&preimage).into();
    let mut venue_oid = [0u8; 16];
    venue_oid.copy_from_slice(&full_hash[..16]);
    VenueIdentity {
        venue_oid,
        full_hash,
        preimage,
    }
}

/// Durable order-saga states.  `SubmissionIntent` and `AckSubmissionIntent`
/// are intentionally distinct: a crash after a successful send but before a
/// local write must be reconciled, never blindly repeated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(i64)]
pub enum OrderState {
    Prepared = 1,
    SubmissionIntent = 2,
    Submitted = 3,
    VenueOpen = 4,
    VenueFilled = 5,
    VenueRejected = 6,
    Reconciling = 7,
    AckSubmissionIntent = 8,
    AckSubmitted = 9,
    Acked = 10,
    FailSubmissionIntent = 11,
    FailSubmitted = 12,
    Failed = 13,
}

impl OrderState {
    pub(crate) fn from_db(value: i64) -> Result<Self, &'static str> {
        match value {
            1 => Ok(Self::Prepared),
            2 => Ok(Self::SubmissionIntent),
            3 => Ok(Self::Submitted),
            4 => Ok(Self::VenueOpen),
            5 => Ok(Self::VenueFilled),
            6 => Ok(Self::VenueRejected),
            7 => Ok(Self::Reconciling),
            8 => Ok(Self::AckSubmissionIntent),
            9 => Ok(Self::AckSubmitted),
            10 => Ok(Self::Acked),
            11 => Ok(Self::FailSubmissionIntent),
            12 => Ok(Self::FailSubmitted),
            13 => Ok(Self::Failed),
            _ => Err("unknown order state"),
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Acked | Self::Failed)
    }

    /// States whose venue result is still unknown and therefore block entries.
    pub fn has_unknown_venue_outcome(self) -> bool {
        matches!(
            self,
            Self::SubmissionIntent | Self::Submitted | Self::VenueOpen | Self::Reconciling
        )
    }
}

/// Typed local error codes.  The journal deliberately never stores remote
/// messages, RPC bodies, bearer tokens, transaction logs, or key material.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i64)]
pub enum ErrorCode {
    VenueUnavailable = 1,
    LedgerUnavailable = 2,
    ReconciliationUnavailable = 3,
    CorrelationMismatch = 4,
}

impl ErrorCode {
    pub(crate) fn from_db(value: i64) -> Result<Self, &'static str> {
        match value {
            1 => Ok(Self::VenueUnavailable),
            2 => Ok(Self::LedgerUnavailable),
            3 => Ok(Self::ReconciliationUnavailable),
            4 => Ok(Self::CorrelationMismatch),
            _ => Err("unknown error code"),
        }
    }
}

/// An authoritative, deduplicable venue fill event.  Aggregate values are
/// calculated with checked arithmetic, while each event remains durable for
/// authoritative partial-fill/cancellation handling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FillFact {
    pub event_id: [u8; 32],
    pub filled_lots: i64,
    pub vwap_quote_lots: i64,
    pub fee_usdc: u64,
    pub fill_price_ticks: u64,
    /// Stable authoritative event timestamp, not the local replay time.
    pub observed_at_ms: u64,
}

impl FillFact {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.filled_lots == 0 || self.filled_lots == i64::MIN {
            return Err("filled_lots must be nonzero and representable as an absolute value");
        }
        if self.fill_price_ticks == 0 {
            return Err("fill_price_ticks must be nonzero");
        }
        Ok(())
    }
}

/// A read-only durable operation projection.  It contains no remote payloads,
/// credentials, private account contents, or signing material.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operation {
    pub operation_id: OperationId,
    pub venue_identity: VenueIdentity,
    pub intent: BoundedIntent,
    pub state: OrderState,
    pub venue_signature: Option<[u8; 64]>,
    pub er_ack_signature: Option<[u8; 64]>,
    pub filled_lots: i64,
    pub fill_vwap_quote_lots: i64,
    pub fee_usdc: u64,
    pub venue_observed_at_ms: Option<u64>,
    pub last_error_code: Option<ErrorCode>,
    pub retry_count: u64,
    pub updated_at_ms: u64,
}

/// Typed authoritative venue outcome.  `Unknown` is fail-closed; elapsed
/// deadlines and local TTLs are never translated to `Rejected`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VenueObservation {
    DefinitelyNeverSubmitted,
    Open { fills: Vec<FillFact> },
    Rejected { fills: Vec<FillFact> },
    Filled { fills: Vec<FillFact> },
    Unknown,
}
