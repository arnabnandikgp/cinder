//! Small, bounded, fixed-width application protocol, independent of journal schemas.
//! All integers are big endian. There is no float, JSON, unknown-field or batch path.
use crate::Error;
use cinder_journal::model::PrivateBytes;
use cinder_kernel::identity::*;
use sha2::{Digest, Sha256};
use std::fmt;
use zeroize::Zeroizing;

/// Maximum decrypted application request size, including its signature.
pub const MAX_REQUEST: usize = 1024;
const MAGIC: &[u8] = b"CINDER-API\0\x00\x01";

/// Read-only account/operation access in an agent grant.
pub const READ: u8 = 1;
/// Bounded order admission in an agent grant.
pub const TRADE: u8 = 2;
/// Cancellation of orders originally admitted by this agent.
pub const CANCEL: u8 = 4;

/// One market per grant keeps lot limits dimensionally explicit. No redelegation,
/// payout, private leverage change or pool-admin permission can be granted.
#[derive(Clone, PartialEq, Eq)]
pub struct Grant {
    /// Agent Ed25519 verification key, not a native venue key.
    pub key: [u8; 32],
    /// Supported READ/TRADE/CANCEL mask only.
    pub methods: u8,
    /// Configured market, including its server-bound precision definition.
    pub market: MarketId,
    /// Maximum absolute lots in each admitted order.
    pub maximum_lots: u64,
    /// Maximum signed fee cap per lot in this pool's quote atoms.
    pub maximum_fee: i128,
    /// Lifetime number of accepted orders in this grant epoch; retries do not spend it.
    pub maximum_orders: u64,
    /// Absolute expiry in injected milliseconds.
    pub expires_at: u64,
}

/// Supported strategy surface; unsupported admin/batch/modify actions do not decode.
#[derive(Clone, PartialEq, Eq)]
pub enum Command {
    /// Versioned owner-scoped read projection; no durable financial mutation.
    Read(crate::reads::Query),
    /// Fresh, authenticated private snapshot; not a public account dump.
    View,
    /// Current state of a stable private operation identity.
    Operation(RequestId),
    /// Exact bounded order; tif is 0=GTC, 1=ALO, 2=IOC.
    Order {
        /// Configured market.
        market: MarketId,
        /// Signed integer lots.
        lots: i64,
        /// Minimum allowed ticks.
        minimum: u64,
        /// Maximum allowed ticks.
        maximum: u64,
        /// Nonnegative quote-atom fee cap per lot.
        fee: i128,
        /// Explicit time-in-force tag.
        tif: u8,
        /// Private-account reduce-only, not native net-position reduce-only.
        reduce_only: bool,
        /// Financial dispatch expiry; immutable on an economic retry.
        good_until: u64,
    },
    /// Cancel only an order in this private account; attempt is distinct from request.
    Cancel {
        /// Original private operation.
        target: RequestId,
        /// Stable cancellation attempt, never generated again on retry.
        attempt: AttemptId,
        /// Cancellation dispatch expiry.
        good_until: u64,
    },
    /// Owner-only payout to the governed owner token account; no `to` parameter.
    Payout {
        /// Net recipient quote atoms, excluding fees.
        net: i128,
        /// Maximum extra customer-paid rail fee.
        maximum_fee: i128,
        /// Explicit consent to a deliberately partial dispatch.
        allow_partial: bool,
        /// Financial dispatch expiry.
        good_until: u64,
    },
    /// Owner installs a bounded Cinder grant, not a native agent.
    Grant(Grant),
    /// Owner revokes all grants and advances the financial authority epoch.
    Revoke,
    /// Owner's private leverage preference, validated by the joined risk engine.
    Leverage {
        /// Configured market.
        market: MarketId,
        /// Leverage scaled by 10,000.
        leverage: u64,
        /// Selection expiry.
        good_until: u64,
    },
}
impl Command {
    /// Reads have no durable financial side effect and do not consume operation IDs.
    pub fn is_read(&self) -> bool {
        matches!(self, Self::View | Self::Operation(_) | Self::Read(_))
    }
    pub(crate) fn encode(&self, w: &mut Writer) {
        match self {
            Self::Read(q) => {
                w.byte(8);
                q.encode(w);
            }
            Self::View => w.byte(0),
            Self::Operation(id) => {
                w.byte(1);
                w.raw(&id.bytes());
            }
            Self::Order {
                market,
                lots,
                minimum,
                maximum,
                fee,
                tif,
                reduce_only,
                good_until,
            } => {
                w.byte(2);
                w.raw(&market.bytes());
                w.raw(&lots.to_be_bytes());
                w.u64(*minimum);
                w.u64(*maximum);
                w.i128(*fee);
                w.byte(*tif);
                w.byte(u8::from(*reduce_only));
                w.u64(*good_until);
            }
            Self::Cancel {
                target,
                attempt,
                good_until,
            } => {
                w.byte(3);
                w.raw(&target.bytes());
                w.raw(&attempt.bytes());
                w.u64(*good_until);
            }
            Self::Payout {
                net,
                maximum_fee,
                allow_partial,
                good_until,
            } => {
                w.byte(4);
                w.i128(*net);
                w.i128(*maximum_fee);
                w.byte(u8::from(*allow_partial));
                w.u64(*good_until);
            }
            Self::Grant(g) => {
                w.byte(5);
                w.raw(&g.key);
                w.byte(g.methods);
                w.raw(&g.market.bytes());
                w.u64(g.maximum_lots);
                w.i128(g.maximum_fee);
                w.u64(g.maximum_orders);
                w.u64(g.expires_at);
            }
            Self::Revoke => w.byte(6),
            Self::Leverage {
                market,
                leverage,
                good_until,
            } => {
                w.byte(7);
                w.raw(&market.bytes());
                w.u64(*leverage);
                w.u64(*good_until);
            }
        }
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self, Error> {
        Ok(match r.byte()? {
            8 => Self::Read(crate::reads::Query::decode(r)?),
            0 => Self::View,
            1 => Self::Operation(RequestId::new(r.array()?).map_err(|_| Error::Invalid)?),
            2 => {
                let market = MarketId::new(r.array()?).map_err(|_| Error::Invalid)?;
                let lots = i64::from_be_bytes(r.array()?);
                let minimum = r.u64()?;
                let maximum = r.u64()?;
                let fee = r.i128()?;
                let tif = r.byte()?;
                if tif > 2 {
                    return Err(Error::Invalid);
                }
                Self::Order {
                    market,
                    lots,
                    minimum,
                    maximum,
                    fee,
                    tif,
                    reduce_only: r.boolean()?,
                    good_until: r.u64()?,
                }
            }
            3 => Self::Cancel {
                target: RequestId::new(r.array()?).map_err(|_| Error::Invalid)?,
                attempt: AttemptId::new(r.array()?).map_err(|_| Error::Invalid)?,
                good_until: r.u64()?,
            },
            4 => Self::Payout {
                net: r.i128()?,
                maximum_fee: r.i128()?,
                allow_partial: r.boolean()?,
                good_until: r.u64()?,
            },
            5 => Self::Grant(Grant {
                key: r.array()?,
                methods: r.byte()?,
                market: MarketId::new(r.array()?).map_err(|_| Error::Invalid)?,
                maximum_lots: r.u64()?,
                maximum_fee: r.i128()?,
                maximum_orders: r.u64()?,
                expires_at: r.u64()?,
            }),
            6 => Self::Revoke,
            7 => Self::Leverage {
                market: MarketId::new(r.array()?).map_err(|_| Error::Invalid)?,
                leverage: r.u64()?,
                good_until: r.u64()?,
            },
            _ => return Err(Error::Invalid),
        })
    }
}

/// Fully signed customer request. Public struct construction grants no authority.
#[derive(Clone, PartialEq, Eq)]
pub struct Request {
    /// Deployment/network isolation.
    pub domain: Domain,
    /// Opaque private account registered by trusted onboarding.
    pub account: AccountId,
    /// Stable application idempotency identity, also its economic nonce.
    pub id: RequestId,
    /// Exact approved economic/precision policy revision.
    pub policy: PolicyVersion,
    /// Current account authorization epoch.
    pub epoch: u64,
    /// Owner or granted agent verification key.
    pub signer: [u8; 32],
    /// Trusted confidential-channel transcript/challenge binding.
    pub session: [u8; 32],
    /// Authentication envelope expiry; reconnect may re-sign without changing intent.
    pub expires_at: u64,
    /// Exact permitted economic/read command.
    pub command: Command,
    /// Ed25519 signature of `message()`, never a native venue signature.
    pub signature: [u8; 64],
}
impl Request {
    /// Canonical bytes signed by wallet/agent. Returned memory is zeroized on drop.
    pub fn message(&self) -> Zeroizing<Vec<u8>> {
        let mut w = Writer::new(MAGIC);
        self.economic_prefix(&mut w);
        w.u64(self.epoch);
        w.raw(&self.signer);
        w.raw(&self.session);
        w.u64(self.expires_at);
        self.command.encode(&mut w);
        w.0
    }
    fn economic_prefix(&self, w: &mut Writer) {
        w.raw(&self.domain.network.bytes());
        w.raw(&self.domain.deployment.bytes());
        w.raw(&self.account.bytes());
        w.raw(&self.id.bytes());
        w.raw(&self.policy.get().to_be_bytes());
    }
    /// Stable economic digest excludes mutable session/authentication metadata.
    /// Financial deadlines and exact command parameters remain included.
    pub fn digest(&self) -> [u8; 32] {
        let mut w = Writer::new(b"CINDER-API-INTENT\0\x00\x01");
        self.economic_prefix(&mut w);
        self.command.encode(&mut w);
        Sha256::digest(&*w.0).into()
    }
    /// Full protected-channel wire, including signature.
    pub fn encode(&self) -> Result<PrivateBytes, Error> {
        let mut bytes = self.message();
        bytes.extend_from_slice(&self.signature);
        if bytes.len() > MAX_REQUEST {
            return Err(Error::Invalid);
        }
        PrivateBytes::new(bytes.to_vec()).map_err(|_| Error::Invalid)
    }
    /// Strict bounded parsing. Trailing bytes, noncanonical booleans/tags reject.
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_REQUEST {
            return Err(Error::Invalid);
        }
        let mut r = Reader::new(bytes, MAGIC)?;
        let value = Self {
            domain: Domain {
                network: NetworkId::new(r.array()?).map_err(|_| Error::Invalid)?,
                deployment: DeploymentId::new(r.array()?).map_err(|_| Error::Invalid)?,
            },
            account: AccountId::new(r.array()?).map_err(|_| Error::Invalid)?,
            id: RequestId::new(r.array()?).map_err(|_| Error::Invalid)?,
            policy: PolicyVersion::new(u32::from_be_bytes(r.array()?))
                .map_err(|_| Error::Invalid)?,
            epoch: r.u64()?,
            signer: r.array()?,
            session: r.array()?,
            expires_at: r.u64()?,
            command: Command::decode(&mut r)?,
            signature: r.array()?,
        };
        r.done()?;
        Ok(value)
    }
}
macro_rules! redacted {
    ($($ty:ty),*) => { $(impl fmt::Debug for $ty {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str("[PRIVATE API]") }
    })* };
}
redacted!(Request, Command, Grant);

pub(crate) struct Writer(pub Zeroizing<Vec<u8>>);
impl Writer {
    pub(crate) fn new(prefix: &[u8]) -> Self {
        Self(Zeroizing::new(prefix.to_vec()))
    }
    pub(crate) fn raw(&mut self, b: &[u8]) {
        self.0.extend_from_slice(b);
    }
    pub(crate) fn byte(&mut self, n: u8) {
        self.0.push(n);
    }
    pub(crate) fn u64(&mut self, n: u64) {
        self.raw(&n.to_be_bytes());
    }
    pub(crate) fn i128(&mut self, n: i128) {
        self.raw(&n.to_be_bytes());
    }
}
pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    pub(crate) fn new(bytes: &'a [u8], prefix: &[u8]) -> Result<Self, Error> {
        if !bytes.starts_with(prefix) {
            return Err(Error::Invalid);
        }
        Ok(Self {
            bytes,
            at: prefix.len(),
        })
    }
    pub(crate) fn array<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let end = self.at.checked_add(N).ok_or(Error::Invalid)?;
        let out = self
            .bytes
            .get(self.at..end)
            .ok_or(Error::Invalid)?
            .try_into()
            .map_err(|_| Error::Invalid)?;
        self.at = end;
        Ok(out)
    }
    pub(crate) fn byte(&mut self) -> Result<u8, Error> {
        Ok(self.array::<1>()?[0])
    }
    pub(crate) fn u64(&mut self) -> Result<u64, Error> {
        Ok(u64::from_be_bytes(self.array()?))
    }
    pub(crate) fn i128(&mut self) -> Result<i128, Error> {
        Ok(i128::from_be_bytes(self.array()?))
    }
    pub(crate) fn boolean(&mut self) -> Result<bool, Error> {
        match self.byte()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Error::Invalid),
        }
    }
    pub(crate) fn done(self) -> Result<(), Error> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(Error::Invalid)
        }
    }
}
