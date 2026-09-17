//! Exact compiled-message joins. Redacted/status-only receipts are not proof.
use crate::rpc::{array, number, string, Result, Rpc, RuntimeError};
use anchor_lang::{AnchorDeserialize, Discriminator};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::Transaction;

pub(crate) fn key(text: &str) -> Result<Pubkey> {
    text.parse().map_err(|_| RuntimeError::Configuration)
}
pub(crate) fn bytes(text: &str) -> Result<[u8; 32]> {
    Ok(key(text)?.to_bytes())
}
pub(crate) fn signature(text: &str) -> Result<[u8; 64]> {
    bs58::decode(text)
        .into_vec()
        .map_err(|_| RuntimeError::Decode)?
        .try_into()
        .map_err(|_| RuntimeError::Decode)
}
pub(crate) fn anchor_ix(
    program: [u8; 32],
    accounts: Vec<([u8; 32], bool, bool)>,
    data: Vec<u8>,
) -> Instruction {
    Instruction {
        program_id: Pubkey::new_from_array(program),
        accounts: accounts
            .into_iter()
            .map(|(k, s, w)| AccountMeta {
                pubkey: Pubkey::new_from_array(k),
                is_signer: s,
                is_writable: w,
            })
            .collect(),
        data,
    }
}
pub(crate) fn rise_ix(ix: phoenix_rise_ix::types::Instruction) -> Instruction {
    anchor_ix(
        ix.program_id.to_bytes(),
        ix.accounts
            .into_iter()
            .map(|a| (a.pubkey.to_bytes(), a.is_signer, a.is_writable))
            .collect(),
        ix.data,
    )
}
pub(crate) struct SignedTransaction {
    pub encoded: String,
    pub signature: [u8; 64],
    pub expiry: u64,
}
pub(crate) fn sign(rpc: &mut Rpc, signer: &Keypair, ix: Instruction) -> Result<SignedTransaction> {
    let latest = rpc.call(
        "getLatestBlockhash",
        json!([{"commitment":"confirmed"}]),
        signer,
    )?;
    let hash = string(&latest["value"]["blockhash"])?
        .parse()
        .map_err(|_| RuntimeError::Decode)?;
    let payer = Pubkey::new_from_array(signer.pubkey().to_bytes());
    let message = Message::new_with_blockhash(&[ix], Some(&payer), &hash);
    // Solana 3 exposes signer Address and message Pubkey through different
    // major versions. Sign the canonical message bytes, not a lossy conversion.
    let sig = signer.sign_message(&message.serialize());
    let signature = sig.as_ref().try_into().map_err(|_| RuntimeError::Decode)?;
    let tx = Transaction {
        signatures: vec![sig],
        message,
    };
    Ok(SignedTransaction {
        encoded: STANDARD.encode(bincode::serialize(&tx).map_err(|_| RuntimeError::Decode)?),
        signature,
        expiry: number(&latest["value"]["lastValidBlockHeight"])?,
    })
}
pub(crate) fn send(rpc: &mut Rpc, signer: &Keypair, tx: &SignedTransaction) -> Result<[u8; 64]> {
    let returned=rpc.call("sendTransaction",json!([tx.encoded,{"encoding":"base64","skipPreflight":false,"preflightCommitment":"confirmed","maxRetries":0}]),signer)?;
    let returned = signature(string(&returned)?)?;
    if returned != tx.signature {
        return Err(RuntimeError::Identity);
    }
    Ok(returned)
}

pub(crate) struct Receipt {
    pub keys: Vec<[u8; 32]>,
    pub signers: usize,
    pub instructions: Vec<Value>,
    pub succeeded: bool,
    pub slot: u64,
}
impl Receipt {
    pub fn decode(value: &Value, expected_sig: &[u8; 64]) -> Result<Self> {
        if signature(string(&value["transaction"]["signatures"][0])?)? != *expected_sig {
            return Err(RuntimeError::Identity);
        }
        let msg = &value["transaction"]["message"];
        let mut keys = array(&msg["accountKeys"])?
            .iter()
            .map(|v| bytes(string(v)?))
            .collect::<Result<Vec<_>>>()?;
        let signers = usize::try_from(number(&msg["header"]["numRequiredSignatures"])?)
            .map_err(|_| RuntimeError::Decode)?;
        if signers == 0
            || signers > keys.len()
            || array(&value["transaction"]["signatures"])?.len() != signers
        {
            return Err(RuntimeError::Decode);
        }
        for field in ["writable", "readonly"] {
            if let Some(rows) = value["meta"]["loadedAddresses"][field].as_array() {
                keys.extend(
                    rows.iter()
                        .map(|v| bytes(string(v)?))
                        .collect::<Result<Vec<_>>>()?,
                );
            }
        }
        if value["meta"].is_null() || value["meta"].get("err").is_none() {
            return Err(RuntimeError::Decode);
        }
        Ok(Self {
            keys,
            signers,
            instructions: array(&msg["instructions"])?.clone(),
            succeeded: value["meta"]["err"].is_null(),
            slot: number(&value["slot"])?,
        })
    }
    pub fn program(&self, ix: &Value) -> Result<[u8; 32]> {
        self.index(&ix["programIdIndex"])
    }
    fn index(&self, v: &Value) -> Result<[u8; 32]> {
        self.keys
            .get(usize::try_from(number(v)?).map_err(|_| RuntimeError::Decode)?)
            .copied()
            .ok_or(RuntimeError::Decode)
    }
    pub fn account(&self, ix: &Value, n: usize) -> Result<[u8; 32]> {
        self.index(ix["accounts"].get(n).ok_or(RuntimeError::Decode)?)
    }
    pub fn requires_signature(&self, key: &[u8; 32]) -> bool {
        self.keys[..self.signers].contains(key)
    }
}
pub(crate) fn instruction_data(ix: &Value) -> Result<Vec<u8>> {
    bs58::decode(string(&ix["data"])?)
        .into_vec()
        .map_err(|_| RuntimeError::Decode)
}
pub(crate) fn decode_ix<T: AnchorDeserialize + Discriminator>(raw: &[u8]) -> Result<Option<T>> {
    if !raw.starts_with(T::DISCRIMINATOR) {
        return Ok(None);
    }
    let mut args = &raw[T::DISCRIMINATOR.len()..];
    let decoded = T::deserialize(&mut args).map_err(|_| RuntimeError::Decode)?;
    if !args.is_empty() {
        return Err(RuntimeError::Decode);
    }
    Ok(Some(decoded))
}
