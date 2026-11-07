//! Canonical, explicitly versioned binary encoding. Not hashing or signing.
//! Fixed integers are big-endian; native event IDs have a bounded u16 length.
//! Decoding validates structure, not authority. Amounts offer `decode_in` to
//! additionally require trusted units; record keys require trusted scope checks.

use crate::{Error, amounts::*, identity::*};
use alloc::vec::Vec;

const MAGIC: &[u8; 7] = b"CINDER\0";
const V1: u16 = 1;

/// Exact canonical encoding. Unsupported schemas/kinds and trailing bytes reject.
pub trait Canonical: Sized {
    /// Serialize all identity/unit bytes; no native Rust layout is persisted.
    fn encode(&self) -> Vec<u8>;
    /// Structural decode only; callers must also check configured scope/units.
    fn decode(bytes: &[u8]) -> Result<Self, Error>;
}

fn start(kind: u8) -> Vec<u8> {
    let mut bytes = Vec::from(MAGIC.as_slice());
    bytes.extend_from_slice(&V1.to_be_bytes());
    bytes.push(kind);
    bytes
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8], kind: u8) -> Result<Self, Error> {
        let mut reader = Self { bytes, offset: 0 };
        if reader.array::<7>()? != *MAGIC {
            return Err(Error::InvalidEncoding);
        }
        if u16::from_be_bytes(reader.array()?) != V1 {
            return Err(Error::UnknownVersion);
        }
        if reader.array::<1>()? != [kind] {
            return Err(Error::InvalidEncoding);
        }
        Ok(reader)
    }
    fn take(&mut self, count: usize) -> Result<&'a [u8], Error> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(Error::InvalidEncoding)?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or(Error::InvalidEncoding)?;
        self.offset = end;
        Ok(bytes)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        self.take(N)?.try_into().map_err(|_| Error::InvalidEncoding)
    }
    fn done(self) -> Result<(), Error> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(Error::InvalidEncoding)
        }
    }
    fn asset(&mut self) -> Result<AssetUnit, Error> {
        Ok(AssetUnit {
            asset: AssetId::new(self.array()?)?,
            precision: PrecisionVersion::new(u32::from_be_bytes(self.array()?))?,
        })
    }
    fn market(&mut self) -> Result<MarketUnit, Error> {
        Ok(MarketUnit {
            market: MarketId::new(self.array()?)?,
            precision: PrecisionVersion::new(u32::from_be_bytes(self.array()?))?,
            quote: self.asset()?,
        })
    }
    fn domain(&mut self) -> Result<Domain, Error> {
        Ok(Domain {
            network: NetworkId::new(self.array()?)?,
            deployment: DeploymentId::new(self.array()?)?,
        })
    }
    fn request(&mut self) -> Result<RequestKey, Error> {
        Ok(RequestKey {
            domain: self.domain()?,
            account: AccountId::new(self.array()?)?,
            request: RequestId::new(self.array()?)?,
        })
    }
}

fn asset(bytes: &mut Vec<u8>, unit: AssetUnit) {
    bytes.extend_from_slice(&unit.asset.bytes());
    bytes.extend_from_slice(&unit.precision.get().to_be_bytes());
}
fn market(bytes: &mut Vec<u8>, unit: MarketUnit) {
    bytes.extend_from_slice(&unit.market.bytes());
    bytes.extend_from_slice(&unit.precision.get().to_be_bytes());
    asset(bytes, unit.quote);
}
fn domain(bytes: &mut Vec<u8>, value: Domain) {
    bytes.extend_from_slice(&value.network.bytes());
    bytes.extend_from_slice(&value.deployment.bytes());
}
fn request(bytes: &mut Vec<u8>, value: RequestKey) {
    domain(bytes, value.domain);
    bytes.extend_from_slice(&value.account.bytes());
    bytes.extend_from_slice(&value.request.bytes());
}

macro_rules! amount_codec {
    ($ty:ty, $unit:ty, $tag:literal, $unit_codec:ident, $value:ident, $raw:ty, $construct:expr) => {
        impl Canonical for $ty {
            fn encode(&self) -> Vec<u8> {
                let mut bytes = start($tag);
                $unit_codec(&mut bytes, self.unit());
                bytes.extend_from_slice(&self.$value().to_be_bytes());
                bytes
            }
            fn decode(bytes: &[u8]) -> Result<Self, Error> {
                let mut reader = Reader::new(bytes, $tag)?;
                let unit = reader.$unit_codec()?;
                let value = <$raw>::from_be_bytes(reader.array()?);
                reader.done()?;
                ($construct)(unit, value)
            }
        }
        impl $ty {
            /// Decode and require trusted asset/market/precision metadata.
            pub fn decode_in(bytes: &[u8], expected: $unit) -> Result<Self, Error> {
                let value = Self::decode(bytes)?;
                value.unit().require(expected)?;
                Ok(value)
            }
        }
    };
}

amount_codec!(
    QuoteAtoms,
    AssetUnit,
    1,
    asset,
    atoms,
    i128,
    |unit, value| Ok(QuoteAtoms::new(unit, value))
);
amount_codec!(
    QuantityLots,
    MarketUnit,
    2,
    market,
    lots,
    i64,
    |unit, value| Ok(QuantityLots::new(unit, value))
);
amount_codec!(
    PriceTicks,
    MarketUnit,
    3,
    market,
    ticks,
    u64,
    PriceTicks::new
);
amount_codec!(
    BasisAtoms,
    MarketUnit,
    4,
    market,
    atoms,
    i128,
    |unit, value| Ok(BasisAtoms::new(unit, value))
);

impl Canonical for RequestKey {
    fn encode(&self) -> Vec<u8> {
        let mut bytes = start(16);
        request(&mut bytes, *self);
        bytes
    }
    fn decode(bytes: &[u8]) -> Result<Self, Error> {
        let mut reader = Reader::new(bytes, 16)?;
        let result = reader.request()?;
        reader.done()?;
        Ok(result)
    }
}

impl Canonical for AttemptKey {
    fn encode(&self) -> Vec<u8> {
        let mut bytes = start(17);
        request(&mut bytes, self.request);
        bytes.extend_from_slice(&self.attempt.bytes());
        bytes
    }
    fn decode(bytes: &[u8]) -> Result<Self, Error> {
        let mut reader = Reader::new(bytes, 17)?;
        let result = Self {
            request: reader.request()?,
            attempt: AttemptId::new(reader.array()?)?,
        };
        reader.done()?;
        Ok(result)
    }
}

impl Canonical for EventKey {
    fn encode(&self) -> Vec<u8> {
        let mut bytes = start(18);
        domain(&mut bytes, self.scope.domain);
        bytes.extend_from_slice(&self.scope.venue.bytes());
        bytes.extend_from_slice(&self.scope.account.bytes());
        bytes.extend_from_slice(&self.scope.namespace.bytes());
        // Constructor bounds the identifier to 128 bytes.
        bytes.extend_from_slice(&(self.event.bytes().len() as u16).to_be_bytes());
        bytes.extend_from_slice(self.event.bytes());
        bytes.extend_from_slice(&self.leg.to_be_bytes());
        bytes
    }
    fn decode(bytes: &[u8]) -> Result<Self, Error> {
        let mut reader = Reader::new(bytes, 18)?;
        let scope = EventScope {
            domain: reader.domain()?,
            venue: VenueId::new(reader.array()?)?,
            account: VenueAccountId::new(reader.array()?)?,
            namespace: NamespaceId::new(reader.array()?)?,
        };
        let length = usize::from(u16::from_be_bytes(reader.array()?));
        if length == 0 || length > 128 {
            return Err(Error::InvalidId);
        }
        let event = EconomicEventId::new(reader.take(length)?)?;
        let leg = u32::from_be_bytes(reader.array()?);
        reader.done()?;
        Ok(Self { scope, event, leg })
    }
}

impl Canonical for RecordKey {
    fn encode(&self) -> Vec<u8> {
        match self {
            Self::Request(k) => k.encode(),
            Self::Attempt(k) => k.encode(),
            Self::Economic(k) => k.encode(),
        }
    }
    fn decode(bytes: &[u8]) -> Result<Self, Error> {
        match bytes.get(9) {
            Some(16) => Ok(Self::Request(RequestKey::decode(bytes)?)),
            Some(17) => Ok(Self::Attempt(AttemptKey::decode(bytes)?)),
            Some(18) => Ok(Self::Economic(EventKey::decode(bytes)?)),
            _ => Err(Error::InvalidEncoding),
        }
    }
}

impl<'a> Payload<'a> {
    /// Frame normalized event bytes together with their schema and policy revision.
    /// Body is bounded to 1..=65,536 bytes; concrete event validation is still required.
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        if self.bytes.is_empty() || self.bytes.len() > 65_536 {
            return Err(Error::InvalidEncoding);
        }
        let mut bytes = start(32);
        bytes.extend_from_slice(&self.policy.get().to_be_bytes());
        bytes.extend_from_slice(&(self.bytes.len() as u32).to_be_bytes());
        bytes.extend_from_slice(self.bytes);
        Ok(bytes)
    }

    /// Zero-copy structural decode. Require approved policy and the concrete event
    /// schema before accounting; this generic frame cannot validate economic fields.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, Error> {
        let mut reader = Reader::new(bytes, 32)?;
        let policy = PolicyVersion::new(u32::from_be_bytes(reader.array()?))?;
        let length = u32::from_be_bytes(reader.array()?);
        if length == 0 || length > 65_536 {
            return Err(Error::InvalidEncoding);
        }
        let data = reader.take(usize::try_from(length).map_err(|_| Error::InvalidEncoding)?)?;
        reader.done()?;
        Ok(Self {
            schema: SchemaVersion::V1,
            policy,
            bytes: data,
        })
    }
}
