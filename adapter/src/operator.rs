//! Operator QFS/TEE token. Do not store user tokens.

use crate::{AdapterError, PubkeyBytes};

/// Challenge signer used by `getAuthToken`.
pub trait TeeAuth {
    fn get_auth_token(
        &self,
        url: &str,
        pubkey: &PubkeyBytes,
        sign: &dyn Fn(&[u8]) -> [u8; 64],
    ) -> Result<String, AdapterError>;
}

/// In-memory operator credential. One token, not a user map.
#[derive(Clone, Default)]
pub struct OperatorAuth {
    token: Option<String>,
    qfs_url: String,
}

impl std::fmt::Debug for OperatorAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OperatorAuth")
            .field("token", &self.token.as_ref().map(|_| "<redacted>"))
            .field("qfs_url", &self.qfs_url)
            .finish()
    }
}

impl OperatorAuth {
    pub fn new(qfs_url: impl Into<String>) -> Self {
        Self {
            token: None,
            qfs_url: qfs_url.into(),
        }
    }

    pub fn authenticate<A: TeeAuth>(
        &mut self,
        auth: &A,
        pubkey: &PubkeyBytes,
        sign: &dyn Fn(&[u8]) -> [u8; 64],
    ) -> Result<(), AdapterError> {
        let token = auth.get_auth_token(&self.qfs_url, pubkey, sign)?;
        self.token = Some(token);
        Ok(())
    }

    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    pub fn qfs_url(&self) -> &str {
        &self.qfs_url
    }
}

/// Test/local stand-in. Real path is TS `getAuthToken` against QFS :6699.
#[derive(Clone, Debug, Default)]
pub struct MockTeeAuth {
    pub token: String,
}

impl TeeAuth for MockTeeAuth {
    fn get_auth_token(
        &self,
        _url: &str,
        _pubkey: &PubkeyBytes,
        _sign: &dyn Fn(&[u8]) -> [u8; 64],
    ) -> Result<String, AdapterError> {
        if self.token.is_empty() {
            return Err(AdapterError::Auth("empty operator token".into()));
        }
        Ok(self.token.clone())
    }
}
