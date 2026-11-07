//! Thin qualification adapter; all cryptographic/state semantics remain in Rust.
use crate::Endpoint;
use wasm_bindgen::prelude::*;

fn redacted(_: crate::Error) -> JsValue {
    JsValue::from_str("Web channel unavailable")
}
/// Published public-key fixture, NEVER a production session or secret.
#[wasm_bindgen]
pub fn standard_vector() -> bool {
    crate::known_answer::verify()
}
/// Independently test canonical context bytes across native and browser builds.
#[wasm_bindgen]
pub fn web_context(policy: &[u8], fields: &[u8], expires: u64) -> Result<Vec<u8>, JsValue> {
    crate::profile::context(policy, fields, expires).map_err(redacted)
}
/// Public context digest; this does not validate attestation.
#[wasm_bindgen]
pub fn web_user_data(context: &[u8]) -> Vec<u8> {
    crate::profile::user_data(context).to_vec()
}
/// Public prologue digest; callers must independently verify the quote first.
#[wasm_bindgen]
pub fn web_prologue(context: &[u8], quote: &[u8]) -> Result<Vec<u8>, JsValue> {
    crate::profile::prologue(context, quote)
        .map(|b| b.to_vec())
        .map_err(redacted)
}
/// P18-sized binding digest. Not a readiness check; Endpoint.binding owns that.
#[wasm_bindgen]
pub fn web_binding(prologue: &[u8], hash: &[u8]) -> Result<Vec<u8>, JsValue> {
    let prologue = prologue.try_into().map_err(|_| redacted(crate::Error))?;
    let hash = hash.try_into().map_err(|_| redacted(crate::Error))?;
    Ok(crate::profile::binding(prologue, hash).to_vec())
}
/// Browser qualification client, not a production attested financial SDK.
#[wasm_bindgen]
pub struct BrowserEndpoint(Endpoint);
#[wasm_bindgen]
impl BrowserEndpoint {
    /// Trusted SYNTHETIC responder key/context supplied by the local test harness.
    #[wasm_bindgen(constructor)]
    pub fn new(public: &[u8], context: &[u8]) -> Result<BrowserEndpoint, JsValue> {
        Endpoint::client(public, context)
            .map(BrowserEndpoint)
            .map_err(redacted)
    }
    /// Empty first handshake message; no private input is possible.
    pub fn start(&mut self) -> Result<Vec<u8>, JsValue> {
        self.0.start().map_err(redacted)
    }
    /// Advance exactly one standard-handshake/key-confirmation step.
    pub fn advance(&mut self, wire: &[u8]) -> Result<Vec<u8>, JsValue> {
        self.0.advance(wire).map_err(redacted)
    }
    /// True only after both key-confirmation records.
    pub fn ready(&self) -> bool {
        self.0.ready()
    }
    /// Public finalized handshake hash, not a claim of Nitro verification.
    pub fn binding(&self) -> Result<Vec<u8>, JsValue> {
        self.0.binding().map(|b| b.to_vec()).map_err(redacted)
    }
    /// Seal a bounded record; failure closes the underlying state.
    pub fn seal(&mut self, body: &[u8]) -> Result<Vec<u8>, JsValue> {
        self.0.seal(body).map_err(redacted)
    }
    /// Copy authenticated plaintext to JS; Rust scratch is zeroized afterward.
    /// No claim is made that JS/browser copies or opaque Snow keys are erased.
    pub fn open(&mut self, wire: &[u8]) -> Result<Vec<u8>, JsValue> {
        self.0.open(wire).map(|b| b.to_vec()).map_err(redacted)
    }
    /// Encrypt an application request with its session-local correlation.
    pub fn request(&mut self, sequence: u32, body: &[u8]) -> Result<Vec<u8>, JsValue> {
        let clear = crate::records::request(sequence, body).map_err(redacted)?;
        self.0.seal(&clear).map_err(redacted)
    }
    /// Same encrypted application record with authenticated WebSocket mode.
    pub fn socket_request(
        &mut self,
        sequence: u32,
        body: &[u8],
        subscribe: bool,
    ) -> Result<Vec<u8>, JsValue> {
        let clear = crate::records::socket_request(sequence, body, subscribe).map_err(redacted)?;
        self.0.seal(&clear).map_err(redacted)
    }
    /// Return only an authenticated COMPLETE correlated application response.
    pub fn response(&mut self, sequence: u32, batch: &[u8]) -> Result<Vec<u8>, JsValue> {
        crate::records::read_response(&mut self.0, sequence, batch)
            .map(|b| b.to_vec())
            .map_err(redacted)
    }
}
