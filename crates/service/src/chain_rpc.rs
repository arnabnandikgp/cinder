//! Narrow authenticated Solana RPC. Provider TLS is a trust assumption, not a
//! Solana light-client proof. Requests, path credentials and signing material
//! terminate in the enclave; parent routes opaque TLS bytes to one fixed host.
use crate::{
    Error,
    egress::{Trust, exchange_bounded},
    transport::Clock,
    vsock::{Target as VsockTarget, VsockStream},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use cinder_journal::model::PrivateBytes;
use cinder_pacifica::{
    execution::Reply,
    funding::{
        ChainDelivery,
        chain::{self, Context, Prepared, Simulation},
    },
};
use openssl::sha::sha256;
use serde::{
    Deserialize, Serialize,
    de::{self, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fmt,
    sync::Arc,
    time::{Duration, Instant},
};
use zeroize::{Zeroize, Zeroizing};

/// Bounded transaction/account read size; no unbounded chain-history downloads.
pub const MAX_RPC_BODY: usize = 262_144;
const DEADLINE: Duration = Duration::from_secs(10);

/// Immutable configured HTTPS route, including a PRIVATE RPC credential path.
/// No URL selected by customer input, redirects, system CAs or environment fallback.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    /// Fixed lowercase DNS hostname, HTTPS port 443 only.
    pub host: String,
    /// Fixed private request target, e.g. a provider's API-key query; never logged.
    pub path: String,
    /// Parent CID 3 vsock port bound in the measured release.
    pub port: u32,
    /// Loaded public CA, canonical DER.
    pub root: Vec<u8>,
    /// Independently approved hash of that CA.
    pub root_hash: [u8; 32],
    /// Exact expected devnet/local qualification genesis, not an RPC URL alias.
    pub network: [u8; 32],
}
impl Drop for Endpoint {
    fn drop(&mut self) {
        self.path.zeroize();
    }
}
impl Endpoint {
    /// Validate before opening a socket; does not authorize a live test.
    pub fn validate(&self) -> Result<(), Error> {
        if self.host.len() > 253
            || self.host.len() < 3
            || self.host.split('.').count() < 2
            || self.host.split('.').any(|s| {
                s.is_empty()
                    || s.len() > 63
                    || s.starts_with('-')
                    || s.ends_with('-')
                    || !s
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            })
            || self.host.parse::<std::net::IpAddr>().is_ok()
            || !self.path.starts_with('/')
            || self.path.starts_with("//")
            || self.path.len() > 2048
            || self.path.contains('#')
            || !self.path.bytes().all(|b| (33..=126).contains(&b))
            || self.network == [0; 32]
        {
            return Err(Error);
        }
        VsockTarget::new(3, self.port)?;
        Trust::from_der(&self.root, self.root_hash)?;
        Ok(())
    }
    /// Bind the actual consumed route without revealing path credentials.
    pub fn commitment(&self) -> Result<[u8; 32], Error> {
        self.validate()?;
        Ok(sha256(
            &[
                b"CINDER-CHAIN-RPC-1\0".as_slice(),
                self.host.as_bytes(),
                b"\0",
                &sha256(self.path.as_bytes()),
                &self.port.to_be_bytes(),
                &self.root_hash,
                &self.network,
            ]
            .concat(),
        ))
    }
}
/// Exact one-shot HTTPS result. No retry or redirect semantics are implied.
pub struct Response {
    /// HTTP status, not Solana transaction execution status.
    pub status: u16,
    /// Raw authenticated RPC body; private, bounded and redacted.
    pub body: PrivateBytes,
    /// Trusted clock after complete body delivery.
    pub at: u64,
}
/// Explicit trusted I/O port; synthetic implementations remain offline evidence.
pub trait Transport {
    /// Exactly one exchange; no automatic retries or error-body logging.
    fn post(&mut self, body: PrivateBytes) -> Result<Response, Error>;
}
/// Actual fixed-route vsock/TLS connection, with no public TCP alternate path.
pub struct Https {
    endpoint: Endpoint,
    trust: Trust,
    clock: Arc<dyn Clock>,
}
impl Https {
    /// Load consumed public trust/private path; no socket is opened here.
    pub fn new(endpoint: Endpoint, clock: Arc<dyn Clock>) -> Result<Self, Error> {
        endpoint.validate()?;
        let trust = Trust::from_der(&endpoint.root, endpoint.root_hash)?;
        Ok(Self {
            endpoint,
            trust,
            clock,
        })
    }
    /// Actual endpoint commitment for release binding, not caller-provided hash.
    pub fn commitment(&self) -> Result<[u8; 32], Error> {
        self.endpoint.commitment()
    }
}
impl Transport for Https {
    fn post(&mut self, body: PrivateBytes) -> Result<Response, Error> {
        if body.as_bytes().is_empty() || body.as_bytes().len() > MAX_RPC_BODY {
            return Err(Error);
        }
        let start = Instant::now();
        let now = self.clock.now()?;
        let mut wire=Zeroizing::new(format!("POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nAccept: application/json\r\nAccept-Encoding: identity\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",self.endpoint.path,self.endpoint.host,body.as_bytes().len()).into_bytes());
        wire.extend_from_slice(body.as_bytes());
        let socket = VsockStream::connect(VsockTarget::new(3, self.endpoint.port)?)?;
        let result = exchange_bounded(
            socket,
            &self.trust,
            &self.endpoint.host,
            now,
            &wire,
            start,
            MAX_RPC_BODY,
        )?;
        let at = self.clock.now()?;
        if at < now || at - now > 10_000 || start.elapsed() > DEADLINE {
            return Err(Error);
        }
        Ok(Response {
            status: result.status,
            body: result.body,
            at,
        })
    }
}
/// Bounded per-boot RPC call budget. It is not the venue's durable credit policy
/// or a global AWS spending cap; the approved run bounds restart counts separately.
pub struct Client<T: Transport> {
    network: [u8; 32],
    transport: T,
    remaining: u32,
    next: u64,
    last_at: u64,
    genesis_at: Option<u64>,
}
impl<T: Transport> Client<T> {
    /// No default origin/identity/budget. Constructing a client grants no funding
    /// mandate; send requires a persisted one-use ChainDelivery.
    pub fn new(network: [u8; 32], transport: T, maximum_calls: u32) -> Result<Self, Error> {
        if network == [0; 32] || maximum_calls == 0 || maximum_calls > 100_000 {
            return Err(Error);
        }
        Ok(Self {
            network,
            transport,
            remaining: maximum_calls,
            next: 1,
            last_at: 0,
            genesis_at: None,
        })
    }
    fn call(
        &mut self,
        method: &str,
        params: Value,
        at: u64,
    ) -> Result<(Value, PrivateBytes, u64), Error> {
        if self.remaining == 0 || at == 0 || at < self.last_at {
            return Err(Error);
        }
        let id = self.next;
        self.next = self.next.checked_add(1).ok_or(Error)?;
        self.remaining -= 1;
        // Even failed/unknown exchanges consume this boot's request budget.
        self.last_at = at;
        let body = PrivateBytes::new(
            serde_json::to_vec(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
                .map_err(|_| Error)?,
        )
        .map_err(|_| Error)?;
        let reply = self.transport.post(body)?;
        if reply.status != 200 || reply.at < at || reply.at - at > 10_000 || reply.at < self.last_at
        {
            return Err(Error);
        }
        self.last_at = reply.at;
        let value = envelope(reply.body.as_bytes(), id)?;
        Ok((value, reply.body, reply.at))
    }
    /// Recheck exact configured genesis over authenticated RPC. No URL-to-network
    /// inference and no fallback to a different provider/network.
    pub fn genesis(&mut self, at: u64) -> Result<u64, Error> {
        self.genesis_at = None;
        let (v, _, at) = self.call("getGenesisHash", json!([]), at)?;
        if chain::parse_address(v.as_str().ok_or(Error)?).map_err(|_| Error)? != self.network {
            return Err(Error);
        }
        self.genesis_at = Some(at);
        Ok(at)
    }
    /// Fresh finalized slot for pre-sign executable/account checks. Distinct
    /// from the confirmed blockhash slot and last-valid block height.
    pub fn slot(&mut self, at: u64) -> Result<(u64, u64), Error> {
        let at = self.genesis(at)?;
        let (slot, _, at) = self.call("getSlot", json!([{"commitment":"finalized"}]), at)?;
        Ok((positive(&slot)?, at))
    }
    /// Fresh native blockhash/height context. A block height is never a slot.
    pub fn context(&mut self, at: u64) -> Result<Context, Error> {
        let at = self.genesis(at)?;
        let (v, _, received) = self.call(
            "getLatestBlockhash",
            json!([{"commitment":"confirmed"}]),
            at,
        )?;
        let slot = positive(&v["context"]["slot"])?;
        let blockhash = chain::parse_address(v["value"]["blockhash"].as_str().ok_or(Error)?)
            .map_err(|_| Error)?;
        let last_valid_height = positive(&v["value"]["lastValidBlockHeight"])?;
        let (height, _, _) = self.call(
            "getBlockHeight",
            json!([{"commitment":"confirmed"}]),
            received,
        )?;
        let height = positive(&height)?;
        if blockhash == [0; 32] || height > last_valid_height {
            return Err(Error);
        }
        Ok(Context {
            network: self.network,
            blockhash,
            slot,
            height,
            last_valid_height,
            at: received,
        })
    }
    /// Simulate/fee-quote the SAME unsigned message without replacing its
    /// blockhash. It cannot sign/send a request or establish a finalized effect.
    pub fn simulate(&mut self, p: &Prepared<'_>, at: u64) -> Result<Simulation, Error> {
        let c = p.context();
        let limits = p.limits();
        if c.network != self.network || at < c.at || at - c.at > limits.maximum_age_ms {
            return Err(Error);
        }
        let at = if self.genesis_at.is_none() {
            self.genesis(at)?
        } else {
            at
        };
        let (fee, _, fee_at) = self.call(
            "getFeeForMessage",
            json!([STANDARD.encode(p.message()),
            {"commitment":"confirmed","minContextSlot":c.slot}]),
            at,
        )?;
        let fee_slot = positive(&fee["context"]["slot"])?;
        let fee_lamports = positive(&fee["value"])?;
        if fee_slot < c.slot || fee_lamports > limits.maximum_fee_lamports {
            return Err(Error);
        }
        let unsigned = p.simulation_wire().map_err(|_| Error)?;
        let (sim, _, sim_at) = self.call(
            "simulateTransaction",
            json!([STANDARD.encode(unsigned.as_bytes()),
            {"encoding":"base64","commitment":"confirmed","sigVerify":false,
             "replaceRecentBlockhash":false,"minContextSlot":c.slot,"innerInstructions":true}]),
            fee_at,
        )?;
        let slot = positive(&sim["context"]["slot"])?;
        if slot < c.slot
            || !sim["value"].is_object()
            || sim["value"].get("err") != Some(&Value::Null)
            || sim["value"]
                .get("replacementBlockhash")
                .is_some_and(|v| !v.is_null())
            || sim_at - c.at > limits.maximum_age_ms
        {
            return Err(Error);
        }
        Ok(Simulation {
            network: self.network,
            message: p.message_hash(),
            slot: slot.max(fee_slot),
            at: fee_at,
            fee_lamports,
            succeeded: true,
        })
    }
    /// Submit one ORIGINAL persisted capability, maxRetries=0, no blockhash
    /// refresh. Any error stays Unknown; even an exact ACK is not final settlement.
    pub fn submit(&mut self, delivery: ChainDelivery, minimum_slot: u64, at: u64) -> Reply {
        if delivery.network() != self.network
            || at >= delivery.expires_at()
            || minimum_slot > delivery.expires_at_slot()
        {
            return Reply::Unknown;
        }
        let at = if self.genesis_at.is_none() {
            match self.genesis(at) {
                Ok(at) => at,
                Err(_) => return Reply::Unknown,
            }
        } else {
            at
        };
        if at >= delivery.expires_at() {
            return Reply::Unknown;
        }
        let wire = delivery.into_wire();
        if minimum_slot == 0 || wire.as_bytes().len() < 65 {
            return Reply::Unknown;
        }
        let expected: [u8; 64] = match wire.as_bytes()[1..65].try_into() {
            Ok(x) => x,
            Err(_) => return Reply::Unknown,
        };
        match self.call("sendTransaction",json!([STANDARD.encode(wire.as_bytes()),
            {"encoding":"base64","skipPreflight":false,"preflightCommitment":"confirmed","maxRetries":0,"minContextSlot":minimum_slot}]),at) {
            Ok((v,raw,received_at)) if v.as_str().and_then(|s|chain::parse_signature(s).ok())==Some(expected)=>
                Reply::Response{status:200,body:raw,received_at,retry_after_ms:None},
            _=>Reply::Unknown,
        }
    }
    /// Original signature's finalized transaction only. Missing/old/pruned or
    /// malformed data is not a no-effect certificate and must keep holds.
    pub fn finalized(&mut self, signature: [u8; 64], at: u64) -> Result<Option<Finalized>, Error> {
        if signature == [0; 64] {
            return Err(Error);
        }
        let at = self.genesis(at)?;
        let text = chain::signature(signature);
        let (statuses, _, at) = self.call(
            "getSignatureStatuses",
            json!([[text],{"searchTransactionHistory":true}]),
            at,
        )?;
        let values = statuses["value"].as_array().ok_or(Error)?;
        if values.len() != 1 {
            return Err(Error);
        }
        let s = &values[0];
        if s.is_null() || s["confirmationStatus"] != "finalized" {
            return Ok(None);
        }
        if s.get("confirmations") != Some(&Value::Null) || s.get("err").is_none() {
            return Err(Error);
        }
        let slot = positive(&s["slot"])?;
        if positive(&statuses["context"]["slot"])? < slot {
            return Err(Error);
        }
        let (tx, raw, at) = self.call(
            "getTransaction",
            json!([chain::signature(signature),
            {"encoding":"base64","commitment":"finalized","maxSupportedTransactionVersion":0}]),
            at,
        )?;
        if tx.is_null() {
            return Ok(None);
        }
        if positive(&tx["slot"])? != slot
            || !tx["meta"].is_object()
            || tx["meta"].get("err") != s.get("err")
            || tx.get("version").is_some_and(|v| v != "legacy")
        {
            return Err(Error);
        }
        let encoded = tx["transaction"].as_array().ok_or(Error)?;
        if encoded.len() != 2 || encoded[1] != "base64" {
            return Err(Error);
        }
        let wire = STANDARD
            .decode(encoded[0].as_str().ok_or(Error)?)
            .map_err(|_| Error)?;
        if wire.len() < 65
            || wire.len() > chain::MAX_WIRE
            || wire[0] != 1
            || wire[1..65] != signature
        {
            return Err(Error);
        }
        let fee = tx["meta"]["fee"].as_u64().ok_or(Error)?;
        Ok(Some(Finalized {
            network: self.network,
            slot,
            wire: PrivateBytes::new(wire).map_err(|_| Error)?,
            meta: tx["meta"].clone(),
            raw,
            at,
            fee_lamports: fee,
        }))
    }
    /// Read exactly a bounded ordered list of accounts at finalized commitment.
    /// A current account read is not historical ownership/finality by itself.
    pub fn accounts(
        &mut self,
        keys: &[[u8; 32]],
        minimum_slot: u64,
        at: u64,
    ) -> Result<Accounts, Error> {
        self.account_request(keys, None, minimum_slot, at)
    }
    /// Bounded finalized data slice for hashing large deployed programs. No
    /// decompression or unbounded program response enters the private journal.
    pub(crate) fn accounts_slice(
        &mut self,
        keys: &[[u8; 32]],
        offset: u32,
        length: u32,
        minimum_slot: u64,
        at: u64,
    ) -> Result<Accounts, Error> {
        if length == 0 || length > 65_536 || offset.checked_add(length).is_none() {
            return Err(Error);
        }
        self.account_request(keys, Some((offset, length)), minimum_slot, at)
    }
    fn account_request(
        &mut self,
        keys: &[[u8; 32]],
        slice: Option<(u32, u32)>,
        minimum_slot: u64,
        at: u64,
    ) -> Result<Accounts, Error> {
        if keys.is_empty()
            || keys.len() > 32
            || minimum_slot == 0
            || keys.iter().collect::<BTreeSet<_>>().len() != keys.len()
        {
            return Err(Error);
        }
        let at = if self.genesis_at.is_none() {
            self.genesis(at)?
        } else {
            at
        };
        let mut options =
            json!({"encoding":"base64","commitment":"finalized","minContextSlot":minimum_slot});
        if let Some((offset, length)) = slice {
            options["dataSlice"] = json!({"offset":offset,"length":length});
        }
        let (v, raw, at) = self.call(
            "getMultipleAccounts",
            json!([
                keys.iter().map(|k| chain::address(*k)).collect::<Vec<_>>(),
                options
            ]),
            at,
        )?;
        let slot = positive(&v["context"]["slot"])?;
        let values = v["value"].as_array().ok_or(Error)?;
        if slot < minimum_slot || values.len() != keys.len() {
            return Err(Error);
        }
        let mut out = Vec::new();
        for (key, v) in keys.iter().zip(values) {
            if v.is_null() {
                out.push(None);
                continue;
            }
            let owner =
                chain::parse_address(v["owner"].as_str().ok_or(Error)?).map_err(|_| Error)?;
            let executable = v["executable"].as_bool().ok_or(Error)?;
            let data = v["data"].as_array().ok_or(Error)?;
            if data.len() != 2 || data[1] != "base64" {
                return Err(Error);
            }
            let bytes = STANDARD
                .decode(data[0].as_str().ok_or(Error)?)
                .map_err(|_| Error)?;
            if bytes.len() > MAX_RPC_BODY || slice.is_some_and(|(_, n)| bytes.len() > n as usize) {
                return Err(Error);
            }
            out.push(Some(Account {
                key: *key,
                owner,
                executable,
                lamports: v["lamports"].as_u64().ok_or(Error)?,
                data: PrivateBytes::new(bytes).map_err(|_| Error)?,
            }));
        }
        Ok(Accounts {
            network: self.network,
            slot,
            values: out,
            raw,
            at,
        })
    }
}
/// Authenticated provider's finalized signature observation; effects still need
/// exact wire/program/token/receipt verification before creating ChainReceipt.
pub struct Finalized {
    /// Genesis checked on this configured provider before finalized lookup.
    pub network: [u8; 32],
    /// Finalized native slot.
    pub slot: u64,
    /// Exact returned legacy wire, checked against the originally persisted wire.
    pub wire: PrivateBytes,
    /// Transaction-specific token changes/error/fee data, never customer balance.
    pub meta: Value,
    /// Raw authenticated evidence for encrypted retention.
    pub raw: PrivateBytes,
    /// Trusted receive time, not execution time.
    pub at: u64,
    /// Actual SOL execution fee, including failed transactions.
    pub fee_lamports: u64,
}
/// Authenticated current account metadata; not an implicit validated SPL type.
pub struct Account {
    /// Requested exact public address.
    pub key: [u8; 32],
    /// Actual owning program, not SPL token authority.
    pub owner: [u8; 32],
    /// Executable flag; token/config accounts must not be executable.
    pub executable: bool,
    /// Native lamports, distinct from token amounts.
    pub lamports: u64,
    /// Exact bounded account data.
    pub data: PrivateBytes,
}
/// Ordered account response and common finalized source context.
pub struct Accounts {
    /// Genesis checked on this configured provider before account reads.
    pub network: [u8; 32],
    /// Current read slot >= requested finalized transaction slot.
    pub slot: u64,
    /// Exact ordered account results, None remains missing evidence.
    pub values: Vec<Option<Account>>,
    /// Raw authenticated account response.
    pub raw: PrivateBytes,
    /// Trusted receive time.
    pub at: u64,
}
fn positive(v: &Value) -> Result<u64, Error> {
    v.as_u64().filter(|v| *v > 0).ok_or(Error)
}

// Reject duplicate keys at EVERY nesting level before selecting security fields.
struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Unique;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("bounded duplicate-free JSON")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut m: M) -> Result<Unique, M::Error> {
                let mut out = serde_json::Map::new();
                while let Some((k, v)) = m.next_entry::<String, Unique>()? {
                    if out.len() >= 128 || out.insert(k, v.0).is_some() {
                        return Err(de::Error::custom("ambiguous map"));
                    }
                }
                Ok(Unique(Value::Object(out)))
            }
            fn visit_seq<S: SeqAccess<'de>>(self, mut s: S) -> Result<Unique, S::Error> {
                let mut out = Vec::new();
                while let Some(v) = s.next_element::<Unique>()? {
                    if out.len() >= 512 {
                        return Err(de::Error::custom("large array"));
                    }
                    out.push(v.0);
                }
                Ok(Unique(Value::Array(out)))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Unique, E> {
                Ok(Unique(Value::String(v.into())))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Unique, E> {
                Ok(Unique(Value::String(v)))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Unique, E> {
                Ok(Unique(Value::from(v)))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Unique, E> {
                Ok(Unique(Value::from(v)))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Unique, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| Unique(Value::Number(n)))
                    .ok_or_else(|| de::Error::custom("invalid number"))
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Unique, E> {
                Ok(Unique(Value::Bool(v)))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_none<E: de::Error>(self) -> Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
        }
        d.deserialize_any(V)
    }
}
fn envelope(body: &[u8], id: u64) -> Result<Value, Error> {
    if body.is_empty() || body.len() > MAX_RPC_BODY {
        return Err(Error);
    }
    let v = serde_json::from_slice::<Unique>(body).map_err(|_| Error)?.0;
    let m = v.as_object().ok_or(Error)?;
    if m.len() != 3
        || m.get("jsonrpc") != Some(&json!("2.0"))
        || m.get("id").and_then(Value::as_u64) != Some(id)
        || !m.contains_key("result")
        || m.contains_key("error")
    {
        return Err(Error);
    }
    Ok(m.get("result").ok_or(Error)?.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    struct Fake {
        results: VecDeque<Result<Value, Error>>,
        requests: Vec<Value>,
        at: u64,
    }
    impl Transport for Fake {
        fn post(&mut self, body: PrivateBytes) -> Result<Response, Error> {
            let request: Value = serde_json::from_slice(body.as_bytes()).unwrap();
            self.requests.push(request.clone());
            let result = self.results.pop_front().unwrap()?;
            Ok(Response {
                status: 200,
                body: PrivateBytes::new(
                    serde_json::to_vec(
                        &json!({"jsonrpc":"2.0","id":request["id"],"result":result}),
                    )
                    .unwrap(),
                )
                .unwrap(),
                at: self.at,
            })
        }
    }
    fn client(results: Vec<Value>, calls: u32) -> Client<Fake> {
        Client::new(
            [1; 32],
            Fake {
                results: results.into_iter().map(Ok).collect(),
                requests: vec![],
                at: 102,
            },
            calls,
        )
        .unwrap()
    }
    fn genesis() -> Value {
        json!(chain::address([1; 32]))
    }
    fn vector() -> Value {
        serde_json::from_str::<Value>(include_str!(
            "../../pacifica/tests/fixtures/funding-codec-public.json"
        ))
        .unwrap()["vectors"][0]
            .clone()
    }
    fn wire() -> Vec<u8> {
        vector()["wire"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u8)
            .collect()
    }
    fn sig() -> [u8; 64] {
        wire()[1..65].try_into().unwrap()
    }
    fn status() -> Value {
        json!({"context":{"slot":201},"value":[{"slot":200,"confirmations":null,"err":null,"confirmationStatus":"finalized"}]})
    }
    fn transaction() -> Value {
        json!({"slot":200,"version":"legacy","transaction":[STANDARD.encode(wire()),"base64"],"meta":{"err":null,"fee":5000}})
    }
    #[test]
    fn json_rpc_envelope_rejects_ambiguous_security_fields_at_all_depths() {
        assert_eq!(
            envelope(br#"{"jsonrpc":"2.0","id":1,"result":null}"#, 1).unwrap(),
            Value::Null
        );
        for b in [
            br#"{"jsonrpc":"2.0","id":1,"id":1,"result":1}"#.as_slice(),
            br#"{"jsonrpc":"2.0","id":1,"result":{"slot":1,"slot":2}}"#,
            br#"{"jsonrpc":"2.0","id":1,"result":[{"err":null,"err":false}]}"#,
            br#"{"jsonrpc":"2.0","id":1,"result":1,"error":null}"#,
            br#"{"jsonrpc":"2.0","id":1,"error":null}"#,
            br#"{"jsonrpc":"2.0","id":2,"result":1}"#,
            br#"{"jsonrpc":"2.0","id":1.0,"result":1}"#,
            br#"{"jsonrpc":"2.0","id":1,"result":1}{}"#,
        ] {
            assert!(envelope(b, 1).is_err());
        }
        assert!(envelope(&vec![b' '; MAX_RPC_BODY + 1], 1).is_err());
    }
    #[test]
    fn exact_genesis_blockhash_slot_and_height_are_independently_checked() {
        let hash = json!({"context":{"slot":120},"value":{"blockhash":chain::address([51;32]),"lastValidBlockHeight":400}});
        let mut c = client(vec![genesis(), hash.clone(), json!(200)], 3);
        let context = c.context(100).unwrap();
        assert_eq!(context.slot, 120);
        assert_eq!(context.height, 200);
        assert_eq!(context.last_valid_height, 400);
        assert_eq!(context.at, 102);
        assert_eq!(
            c.transport
                .requests
                .iter()
                .map(|r| r["method"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["getGenesisHash", "getLatestBlockhash", "getBlockHeight"]
        );
        let mut c = client(vec![json!(chain::address([2; 32]))], 2);
        assert!(c.context(100).is_err());
        assert_eq!(c.transport.requests.len(), 1);
        let mut c = client(vec![genesis(), hash, json!(401)], 3);
        assert!(c.context(100).is_err());
    }
    #[test]
    fn unknown_http_and_transport_outcomes_consume_the_per_boot_budget_without_retry() {
        let mut c = client(vec![], 1);
        c.transport.results.push_back(Err(Error));
        assert!(c.genesis(100).is_err());
        assert_eq!(c.transport.requests.len(), 1);
        assert!(c.genesis(102).is_err());
        assert_eq!(c.transport.requests.len(), 1);
        let mut c = client(vec![genesis(), genesis()], 2);
        assert!(c.genesis(100).is_ok());
        assert!(c.genesis(101).is_err());
        assert_eq!(c.transport.requests.len(), 1);
        assert!(Client::new([0; 32], c.transport, 1).is_err());
    }
    #[test]
    fn nonfinal_missing_or_pruned_signatures_are_not_no_effect_receipts() {
        for value in [
            Value::Null,
            json!({"slot":200,"confirmationStatus":"confirmed"}),
        ] {
            let mut c = client(
                vec![genesis(), json!({"context":{"slot":201},"value":[value]})],
                3,
            );
            assert!(c.finalized(sig(), 100).unwrap().is_none());
            assert_eq!(c.transport.requests.len(), 2);
        }
        let mut c = client(vec![genesis(), status(), Value::Null], 3);
        assert!(c.finalized(sig(), 100).unwrap().is_none());
        assert_eq!(
            c.transport.requests[1]["params"][1]["searchTransactionHistory"],
            true
        );
        assert_eq!(
            c.transport.requests[2]["params"][1]["commitment"],
            "finalized"
        );
    }
    #[test]
    fn finalized_lookup_requires_original_wire_signature_slot_error_and_legacy_version() {
        let mut c = client(vec![genesis(), status(), transaction()], 3);
        let tx = c.finalized(sig(), 100).unwrap().unwrap();
        assert_eq!(tx.wire.as_bytes(), wire());
        assert_eq!(tx.slot, 200);
        assert_eq!(tx.fee_lamports, 5000);
        for (field, value) in [
            ("slot", json!(202)),
            ("version", json!(0)),
            (
                "transaction",
                json!([STANDARD.encode(vec![0; 70]), "base64"]),
            ),
        ] {
            let mut tx = transaction();
            tx[field] = value;
            let mut c = client(vec![genesis(), status(), tx], 3);
            assert!(c.finalized(sig(), 100).is_err());
        }
        let mut tx = transaction();
        tx["meta"]["err"] = json!({"InstructionError":[0,"Custom"]});
        let mut c = client(vec![genesis(), status(), tx], 3);
        assert!(c.finalized(sig(), 100).is_err());
        let mut st = status();
        st["value"][0]["confirmations"] = json!(0);
        let mut c = client(vec![genesis(), st], 3);
        assert!(c.finalized(sig(), 100).is_err());
    }
    #[test]
    fn account_reads_bind_key_order_count_context_and_exact_base64_owner() {
        let account = json!({"owner":chain::address([31;32]),"executable":false,"lamports":1000,"data":[STANDARD.encode([1,2,3]),"base64"]});
        let mut c = client(
            vec![
                genesis(),
                json!({"context":{"slot":200},"value":[account.clone(),null]}),
            ],
            2,
        );
        let accounts = c.accounts(&[[11; 32], [12; 32]], 199, 100).unwrap();
        assert_eq!(accounts.values[0].as_ref().unwrap().key, [11; 32]);
        assert!(accounts.values[1].is_none());
        assert_eq!(accounts.values[0].as_ref().unwrap().owner, [31; 32]);
        assert_eq!(
            c.transport.requests[1]["params"][0],
            json!([chain::address([11; 32]), chain::address([12; 32])])
        );
        let mut c = client(vec![], 2);
        assert!(c.accounts(&[[11; 32], [11; 32]], 199, 100).is_err());
        assert!(c.transport.requests.is_empty());
        for (slot, values) in [
            (198, json!([account.clone()])),
            (200, json!([account.clone(), null])),
        ] {
            let mut c = client(
                vec![genesis(), json!({"context":{"slot":slot},"value":values})],
                2,
            );
            assert!(c.accounts(&[[11; 32]], 199, 100).is_err());
        }
        let mut invalid = account;
        invalid["data"][1] = json!("base64+zstd");
        let mut c = client(
            vec![genesis(), json!({"context":{"slot":200},"value":[invalid]})],
            2,
        );
        assert!(c.accounts(&[[11; 32]], 199, 100).is_err());
    }
}
