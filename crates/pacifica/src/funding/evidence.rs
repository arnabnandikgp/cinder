//! Documented native evidence components. Parsing authenticates neither source
//! nor completeness: only in-enclave authenticated observation may supply these
//! bytes, and operation finality/cuts remain separately qualified provider gates.
use crate::{Error, observation::MAX_BODY, profile::Profile};
use serde::{Deserialize, Deserializer};

fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}
fn signature(text: &str) -> Result<[u8; 64], Error> {
    let decoded = bs58::decode(text).into_vec().map_err(|_| Error::Codec)?;
    let bytes: [u8; 64] = decoded.try_into().map_err(|_| Error::Codec)?;
    if bytes == [0; 64] || bs58::encode(bytes).into_string() != text {
        return Err(Error::Codec);
    }
    Ok(bytes)
}
fn atoms(profile: &Profile, value: &str) -> Result<u64, Error> {
    u64::try_from(profile.quote(value)?.atoms()).map_err(|_| Error::Codec)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    channel: String,
    data: Transfer,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Transfer {
    u: String,
    e: String,
    a: String,
    am: String,
    t: u64,
    #[serde(default, deserialize_with = "present")]
    tx: Option<String>,
    #[serde(default, deserialize_with = "present")]
    s: Option<String>,
    #[serde(default, deserialize_with = "present")]
    r: Option<String>,
    #[serde(default, deserialize_with = "present")]
    bn: Option<u64>,
    #[serde(default, deserialize_with = "present")]
    ra: Option<String>,
    #[serde(default, deserialize_with = "present")]
    f: Option<String>,
}
/// A linked wire observation, NOT a Credit/Withdrawal/Coverage certificate.
/// No native causal frontier or no-later-effect statement is inferred here.
#[derive(Clone, PartialEq, Eq)]
pub enum TransferObservation {
    /// Exact native account/asset and original chain signature; total finality
    /// and mapping to the persisted deposit attempt remain provider work.
    Deposit {
        /// Canonical native transaction identifier, not independently finalized.
        signature: [u8; 64],
        /// Exact positive quote atoms reported in the event.
        amount: u64,
        /// Source timestamp in milliseconds, not a causal frontier.
        at: u64,
    },
    /// Native batch identity is not the request's idempotency UUID. A qualified
    /// original UUID -> batch binding must survive a lost acknowledgment.
    Withdrawal {
        /// Required for confirmed events; still needs chain verification.
        signature: Option<[u8; 64]>,
        /// Native batch nonce, not the persisted request UUID.
        batch: u64,
        /// Reported net quote atoms after the fee.
        amount: u64,
        /// Reported gross requested quote atoms.
        requested: u64,
        /// Reported nonnegative fee atoms; net plus fee equals gross.
        fee: u64,
        /// Source timestamp in milliseconds, not proof of terminality.
        at: u64,
        /// Native event label only, not finalized chain payment.
        confirmed: bool,
    },
}
impl std::fmt::Debug for TransferObservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TransferObservation([PRIVATE])")
    }
}
/// Decode one bounded documented account_transfers message. Missing linkage,
/// explicit-null optional fields, aliases, duplicate fields, wrong account/asset,
/// inexact decimals and malformed signatures refuse. Does not change financial state.
pub fn transfer(
    profile: &Profile,
    quote_symbol: &str,
    bytes: &[u8],
    received_at: u64,
    maximum_age_ms: u64,
) -> Result<TransferObservation, Error> {
    profile.commitment()?;
    if bytes.is_empty()
        || bytes.len() > MAX_BODY
        || maximum_age_ms == 0
        || maximum_age_ms > 60000
        || quote_symbol.is_empty()
        || quote_symbol.len() > 16
        || !quote_symbol
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
    {
        return Err(Error::Qualification);
    }
    let envelope: Envelope = serde_json::from_slice(bytes).map_err(|_| Error::Codec)?;
    let t = envelope.data;
    if envelope.channel != "account_transfers"
        || t.u != profile.account
        || t.a != quote_symbol
        || t.t == 0
        || t.t > received_at
        || received_at - t.t > maximum_age_ms
        || t.s.is_some()
        || t.r.is_some()
    {
        return Err(Error::Qualification);
    }
    let amount = atoms(profile, &t.am)?;
    if amount == 0 {
        return Err(Error::Codec);
    }
    match t.e.as_str() {
        "deposit" if t.bn.is_none() && t.ra.is_none() && t.f.is_none() => {
            Ok(TransferObservation::Deposit {
                signature: signature(t.tx.as_deref().ok_or(Error::Qualification)?)?,
                amount,
                at: t.t,
            })
        }
        "withdrawal_pending" | "withdrawal_confirmed" => {
            let confirmed = t.e == "withdrawal_confirmed";
            let tx = t.tx.as_deref().map(signature).transpose()?;
            if confirmed && tx.is_none() {
                return Err(Error::Qualification);
            }
            let requested = atoms(profile, t.ra.as_deref().ok_or(Error::Qualification)?)?;
            let fee = atoms(profile, t.f.as_deref().ok_or(Error::Qualification)?)?;
            if requested == 0 || amount.checked_add(fee) != Some(requested) {
                return Err(Error::Qualification);
            }
            Ok(TransferObservation::Withdrawal {
                signature: tx,
                batch: t.bn.ok_or(Error::Qualification)?,
                amount,
                requested,
                fee,
                at: t.t,
                confirmed,
            })
        }
        _ => Err(Error::Qualification),
    }
}

#[derive(Deserialize)]
struct Response<T> {
    success: bool,
    data: T,
    error: Option<String>,
}
#[derive(Deserialize)]
struct Settings {
    auto_lend_disabled: Option<bool>,
    margin_settings: Vec<Margin>,
    error: Option<String>,
    code: Option<i64>,
}
#[derive(Deserialize)]
struct Margin {
    symbol: String,
    isolated: bool,
    leverage: u64,
}
#[derive(Deserialize)]
struct Loan {
    borrowed: String,
    pending_interest: String,
    updated_at: u64,
}
/// Concrete readiness observations. They do not certify atomic/fresh native
/// setup and cannot automatically construct Controller::Setup.complete.
#[derive(Clone, PartialEq, Eq)]
pub struct SetupObservation {
    /// Explicit true only; null/missing means not disabled.
    pub lending_disabled: bool,
    /// Cross-only mapped markets and nonzero leverage.
    pub compatible_margin: bool,
    /// Exact positive/zero borrowed quote atoms.
    pub borrowed: u64,
    /// Exact pending interest atoms, never a missing-cache zero.
    pub interest: u64,
    /// Loan source update timestamp, not settings atomicity or a causal cut.
    pub loan_at: u64,
}
impl std::fmt::Debug for SetupObservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SetupObservation([PRIVATE])")
    }
}
/// Parse bounded documented settings/loan bodies, with no credit/mode mutation.
pub fn setup(profile: &Profile, settings: &[u8], loan: &[u8]) -> Result<SetupObservation, Error> {
    profile.commitment()?;
    if settings.is_empty() || loan.is_empty() || settings.len() > MAX_BODY || loan.len() > MAX_BODY
    {
        return Err(Error::Limit);
    }
    let s: Response<Settings> = serde_json::from_slice(settings).map_err(|_| Error::Codec)?;
    let l: Response<Loan> = serde_json::from_slice(loan).map_err(|_| Error::Codec)?;
    if !s.success
        || !l.success
        || s.error.is_some()
        || l.error.is_some()
        || s.data.error.is_some()
        || s.data.code.is_some()
        || s.data.margin_settings.len() > 32
        || l.data.updated_at == 0
    {
        return Err(Error::Qualification);
    }
    let compatible = s.data.margin_settings.iter().enumerate().all(|(i, m)| {
        !m.isolated
            && m.leverage != 0
            && profile.markets.iter().any(|p| p.symbol == m.symbol)
            && !s.data.margin_settings[..i]
                .iter()
                .any(|old| old.symbol == m.symbol)
    });
    Ok(SetupObservation {
        lending_disabled: s.data.auto_lend_disabled == Some(true),
        compatible_margin: compatible,
        borrowed: atoms(profile, &l.data.borrowed)?,
        interest: atoms(profile, &l.data.pending_interest)?,
        loan_at: l.data.updated_at,
    })
}
