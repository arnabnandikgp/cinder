//! Unsigned canonical bytes for one bounded limit or scoped cancellation.
//! No key, dispatch, nonce allocation, financial reservation or retry authority.
use crate::{
    Error, Scope,
    precision::{Market, scaled},
    symbol,
};
use sha2::{Digest, Sha256};

/// Conservative encoding bound, not leverage or financial admission policy.
/// Below 2^48 scaled units, decimal-to-binary64 and multiplication errors stay
/// below half an atom for the native round(x*1e8) convention. Never use f64 here.
pub const MAX_WIRE: u64 = 1_u64 << 48;

/// Supported native time-in-force, no trigger/builder/isolated/batch schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TimeInForce {
    /// Resting good-till-cancelled.
    Gtc = 0,
    /// Bounded immediate execution; unfilled remainder cancels.
    Ioc = 1,
    /// Native add-liquidity-only.
    Alo = 2,
}
impl TimeInForce {
    fn name(self) -> &'static str {
        match self {
            Self::Gtc => "GTC",
            Self::Ioc => "IOC",
            Self::Alo => "ALO",
        }
    }
}

/// Validated native action, not an authorized private customer order.
#[derive(Clone)]
pub struct Action {
    kind: Kind,
}
#[derive(Clone)]
enum Kind {
    Limit {
        symbol: String,
        buy: bool,
        price: u64,
        size: u64,
        tif: TimeInForce,
    },
    Cancel {
        symbol: String,
        order: [u8; 32],
    },
}
impl std::fmt::Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Action([PRIVATE])")
    }
}
impl Action {
    /// Exact positive price collar/size; only resting TIFs require tick alignment.
    /// Native reduce-only is intentionally false:
    /// a private customer's reduction is not necessarily a reduction of pooled net.
    pub fn limit(
        market: &Market,
        buy: bool,
        price: &str,
        size: &str,
        tif: TimeInForce,
    ) -> Result<Self, Error> {
        market.quantity(size)?;
        if tif != TimeInForce::Ioc {
            market.price(price)?;
        }
        let bounded = |s| {
            u64::try_from(scaled(s, 8)?)
                .ok()
                .filter(|n| *n > 0 && *n <= MAX_WIRE)
                .ok_or(Error::Limit)
        };
        Ok(Self {
            kind: Kind::Limit {
                symbol: market.symbol.clone(),
                buy,
                price: bounded(price)?,
                size: bounded(size)?,
                tif,
            },
        })
    }
    /// Encode an already-attributed native original order ID. The future controller
    /// must establish ownership/attempt identity before exposing a signature.
    pub fn cancel(name: &str, order: [u8; 32]) -> Result<Self, Error> {
        symbol(name)?;
        if order == [0; 32] {
            return Err(Error::Invalid);
        }
        Ok(Self {
            kind: Kind::Cancel {
                symbol: name.into(),
                order,
            },
        })
    }
    fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let name = match &self.kind {
            Kind::Limit { symbol, .. } => {
                out.extend(1_u32.to_le_bytes());
                symbol
            }
            Kind::Cancel { symbol, .. } => {
                out.extend(3_u32.to_le_bytes());
                symbol
            }
        };
        out.extend((name.len() as u64).to_le_bytes());
        out.extend(name.as_bytes());
        match &self.kind {
            Kind::Limit {
                buy,
                price,
                size,
                tif,
                ..
            } => {
                out.push(u8::from(*buy));
                out.extend(price.to_le_bytes());
                out.extend(size.to_le_bytes());
                out.extend((*tif as u32).to_le_bytes());
                out.extend([0, 0]); // reduce_only, iso; no Option tag in legacy layout.
            }
            Kind::Cancel { order, .. } => out.extend(order),
        }
        out
    }
    /// Exact single-action raw-signing message. Cinder scope commitment must be
    /// durably bound separately; native bytes do not include the Cinder source.
    pub fn message(&self, scope: &Scope, nonce: u64) -> Result<Vec<u8>, Error> {
        if nonce == 0 {
            return Err(Error::Invalid);
        }
        let mut out = 1_u64.to_le_bytes().to_vec();
        out.extend(self.bytes());
        out.extend(nonce.to_le_bytes());
        out.extend(scope.account);
        out.push(scope.network as u8);
        Ok(out)
    }
    /// Deterministic native order hash at the sole transaction action index 0. No network
    /// domain enters BULK's hash; always combine it with `Scope::commitment` locally.
    /// The caller cannot select an index inconsistent with the single-action message.
    pub fn order_id(&self, scope: &Scope, nonce: u64) -> Result<[u8; 32], Error> {
        if !matches!(self.kind, Kind::Limit { .. }) {
            return Err(Error::Unsupported);
        }
        if nonce == 0 {
            return Err(Error::Invalid);
        }
        Ok(self.hash(scope.account, nonce, 0))
    }
    fn hash(&self, account: [u8; 32], nonce: u64, index: u32) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(index.to_le_bytes());
        h.update(self.bytes());
        h.update(account);
        h.update(nonce.to_le_bytes());
        h.finalize().into()
    }
    /// Native action JSON with exact quoted decimal fields, not an authenticated
    /// envelope. Never add a caller's raw JSON action to this encoded message.
    pub fn json(&self) -> Result<Vec<u8>, Error> {
        let decimal = |n: u64| format!("{}.{:08}", n / 100_000_000, n % 100_000_000);
        let value = match &self.kind {
            Kind::Limit {
                symbol,
                buy,
                price,
                size,
                tif,
            } => {
                serde_json::json!({"l":{"c":symbol,"b":buy,"px":decimal(*price),"sz":decimal(*size),"tif":tif.name(),"r":false,"i":false}})
            }
            Kind::Cancel { symbol, order } => {
                serde_json::json!({"cx":{"c":symbol,"oid":bs58::encode(order).into_string()}})
            }
        };
        serde_json::to_vec(&value).map_err(|_| Error::Invalid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn upstream_default_meta_hash_uses_zero_account_nonce_and_index() {
        let market = Market::new("BTC-USD", "0.00000001", "0.1", 8).unwrap();
        let action = Action::limit(&market, true, "100", "3", TimeInForce::Gtc).unwrap();
        // Upstream fixture's default_meta_order_ids use default metadata, not its
        // signable message's [1;32]/nonce 42 or the action's position in the array.
        assert_eq!(
            bs58::encode(action.hash([0; 32], 0, 0)).into_string(),
            "6hA5kud5xUXx69i6bgaf4ZDRZUW75qftwQuhv6Ubd3gg"
        );
    }
}
