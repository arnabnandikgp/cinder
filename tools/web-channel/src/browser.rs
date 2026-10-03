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
}
