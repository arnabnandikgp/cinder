//! Bounded, redacted RPC transport. Credentials never enter journal records.
use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::{blocking::Client, redirect::Policy, Url};
use serde_json::{json, Value};
use solana_keypair::Keypair;
use solana_signer::Signer;
use std::{
    fs::OpenOptions,
    io::Read,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeError {
    Configuration,
    SecretFile,
    Transport,
    Authentication,
    Rpc,
    Decode,
    Identity,
    Incomplete,
    Stale,
    Unsupported,
    Journal,
}
impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "operator runtime: {self:?}")
    }
}
impl std::error::Error for RuntimeError {}
pub type Result<T> = std::result::Result<T, RuntimeError>;
pub fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| u64::try_from(d.as_millis()).ok())
        .unwrap_or(0)
}

pub fn load_signer(path: &Path) -> Result<Keypair> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file = options.open(path).map_err(|_| RuntimeError::SecretFile)?;
    let meta = file.metadata().map_err(|_| RuntimeError::SecretFile)?;
    if !meta.is_file() || meta.len() > 4096 {
        return Err(RuntimeError::SecretFile);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if meta.uid() != unsafe { libc::geteuid() } || meta.mode() & 0o077 != 0 || meta.nlink() != 1
        {
            return Err(RuntimeError::SecretFile);
        }
    }
    let bytes: Vec<u8> = serde_json::from_reader(file).map_err(|_| RuntimeError::SecretFile)?;
    Keypair::try_from(bytes.as_slice()).map_err(|_| RuntimeError::SecretFile)
}

pub(crate) struct Rpc {
    client: Client,
    url: Url,
    auth: Option<(String, u64)>,
    qfs: bool,
}
impl Rpc {
    pub fn new(endpoint: &str, qfs: bool) -> Result<Self> {
        let url = Url::parse(endpoint).map_err(|_| RuntimeError::Configuration)?;
        let loopback = matches!(
            url.host_str(),
            Some("localhost" | "127.0.0.1" | "[::1]" | "::1")
        );
        if !(url.scheme() == "https" || url.scheme() == "http" && loopback)
            || !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
            || url.query_pairs().any(|(k, _)| k == "token")
        {
            return Err(RuntimeError::Configuration);
        }
        let client = Client::builder()
            .redirect(Policy::none())
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|_| RuntimeError::Transport)?;
        Ok(Self {
            client,
            url,
            auth: None,
            qfs,
        })
    }
    fn decode(response: reqwest::blocking::Response) -> Result<Value> {
        if !response.status().is_success() {
            return Err(RuntimeError::Rpc);
        }
        let mut bytes = Vec::new();
        response
            .take(8 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| RuntimeError::Transport)?;
        if bytes.len() > 8 * 1024 * 1024 {
            return Err(RuntimeError::Decode);
        }
        serde_json::from_slice(&bytes).map_err(|_| RuntimeError::Decode)
    }
    pub fn authenticate(&mut self, signer: &Keypair) -> Result<()> {
        if !self.qfs {
            return Ok(());
        }
        if self
            .auth
            .as_ref()
            .is_some_and(|(_, expiry)| *expiry > unix_ms().saturating_add(30_000))
        {
            return Ok(());
        }
        self.auth = None;
        let pubkey = signer.pubkey().to_string();
        let mut challenge_url = self.url.clone();
        challenge_url.set_path("/auth/challenge");
        challenge_url
            .query_pairs_mut()
            .append_pair("pubkey", &pubkey);
        let challenge = Self::decode(
            self.client
                .get(challenge_url)
                .send()
                .map_err(|_| RuntimeError::Authentication)?,
        )?;
        let challenge = string(&challenge["challenge"])?;
        if challenge.len() > 4096 {
            return Err(RuntimeError::Authentication);
        }
        let signature = signer.sign_message(challenge.as_bytes()).to_string();
        let mut login_url = self.url.clone();
        login_url.set_path("/auth/login");
        let login = Self::decode(
            self.client
                .post(login_url)
                .json(&json!({"pubkey":pubkey,"challenge":challenge,"signature":signature}))
                .send()
                .map_err(|_| RuntimeError::Authentication)?,
        )?;
        let token = string(&login["token"])?;
        if token.is_empty() || token.len() > 16384 {
            return Err(RuntimeError::Authentication);
        }
        let expiry = login["expiresAt"]
            .as_u64()
            .unwrap_or_else(|| unix_ms().saturating_add(3_600_000));
        if expiry <= unix_ms().saturating_add(30_000) {
            return Err(RuntimeError::Authentication);
        }
        self.auth = Some((token.to_owned(), expiry));
        Ok(())
    }
    pub fn call(&mut self, method: &str, params: Value, signer: &Keypair) -> Result<Value> {
        self.authenticate(signer)?;
        let mut url = self.url.clone();
        if let Some((token, _)) = &self.auth {
            url.query_pairs_mut().append_pair("token", token);
        }
        // Never automatically resend a signed transaction on an ambiguous response.
        let response = self
            .client
            .post(url)
            .json(&json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}))
            .send()
            .map_err(|_| RuntimeError::Transport)?;
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            self.auth = None;
            return Err(RuntimeError::Authentication);
        }
        let value = Self::decode(response)?;
        if !value["error"].is_null() {
            return Err(RuntimeError::Rpc);
        }
        value.get("result").cloned().ok_or(RuntimeError::Decode)
    }
    pub fn accounts(&mut self, keys: &[String], signer: &Keypair) -> Result<(u64, Vec<Value>)> {
        if keys.is_empty() || keys.len() > 100 {
            return Err(RuntimeError::Incomplete);
        }
        let result = self.call(
            "getMultipleAccounts",
            json!([keys,{"encoding":"base64","commitment":"confirmed"}]),
            signer,
        )?;
        let rows = array(&result["value"])?.clone();
        if rows.len() != keys.len() || rows.iter().any(Value::is_null) {
            return Err(RuntimeError::Incomplete);
        }
        Ok((number(&result["context"]["slot"])?, rows))
    }
    pub fn scan(
        &mut self,
        owner: &str,
        discriminator: &[u8],
        signer: &Keypair,
    ) -> Result<Vec<Value>> {
        let result = self.call("getProgramAccounts", json!([owner,{"encoding":"base64","commitment":"confirmed","filters":[{"memcmp":{"offset":0,"bytes":bs58::encode(discriminator).into_string()}}]}]), signer)?;
        Ok(array(&result)?.clone())
    }
    pub fn transaction(
        &mut self,
        signature: &str,
        signer: &Keypair,
        finalized: bool,
    ) -> Result<Value> {
        self.call("getTransaction", json!([signature,{"encoding":"json","commitment":if finalized {"finalized"} else {"confirmed"},"maxSupportedTransactionVersion":0}]), signer)
    }
    pub fn history(&mut self, address: &str, signer: &Keypair) -> Result<Vec<Value>> {
        let mut rows = Vec::new();
        let mut before = None;
        loop {
            let mut cfg = json!({"limit":1000,"commitment":"confirmed"});
            if let Some(cursor) = &before {
                cfg["before"] = json!(cursor);
            }
            let page = self.call("getSignaturesForAddress", json!([address, cfg]), signer)?;
            let page = array(&page)?;
            if page.is_empty() {
                return Ok(rows);
            }
            let cursor = string(&page.last().ok_or(RuntimeError::Decode)?["signature"])?.to_owned();
            if before.as_ref() == Some(&cursor) || rows.len() + page.len() > 10_000 {
                return Err(RuntimeError::Incomplete);
            }
            rows.extend(page.iter().cloned());
            before = Some(cursor);
        }
    }
}
pub(crate) fn number(v: &Value) -> Result<u64> {
    v.as_u64().ok_or(RuntimeError::Decode)
}
pub(crate) fn string(v: &Value) -> Result<&str> {
    v.as_str().ok_or(RuntimeError::Decode)
}
pub(crate) fn array(v: &Value) -> Result<&Vec<Value>> {
    v.as_array().ok_or(RuntimeError::Decode)
}
pub(crate) fn data(account: &Value, owner: &str) -> Result<Vec<u8>> {
    if string(&account["owner"])? != owner
        || string(&account["data"][1])? != "base64"
        || account["executable"] != false
    {
        return Err(RuntimeError::Identity);
    }
    STANDARD
        .decode(string(&account["data"][0])?)
        .map_err(|_| RuntimeError::Decode)
}

#[cfg(test)]
mod secret_tests {
    use super::*;

    #[test]
    fn key_file_must_be_valid_and_owner_only() {
        use std::fs;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("operator.json");
        fs::write(&path, b"[]").unwrap();
        assert!(load_signer(&path).is_err());
        let key = Keypair::new();
        fs::write(
            &path,
            serde_json::to_vec(&key.to_bytes().as_slice()).unwrap(),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
            assert!(load_signer(&path).is_err());
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        }
        assert_eq!(load_signer(&path).unwrap().pubkey(), key.pubkey());
    }

    #[test]
    #[cfg(unix)]
    fn linked_secret_files_are_rejected() {
        use std::{
            fs,
            os::unix::fs::{symlink, PermissionsExt},
        };
        let dir = tempfile::tempdir().unwrap();
        let key = Keypair::new();
        let path = dir.path().join("operator.json");
        fs::write(
            &path,
            serde_json::to_vec(&key.to_bytes().as_slice()).unwrap(),
        )
        .unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        let link = dir.path().join("symlink");
        symlink(&path, &link).unwrap();
        assert!(load_signer(&link).is_err());
        let hard = dir.path().join("hardlink");
        fs::hard_link(&path, &hard).unwrap();
        assert!(load_signer(&path).is_err());
        assert!(load_signer(&hard).is_err());
    }
}
