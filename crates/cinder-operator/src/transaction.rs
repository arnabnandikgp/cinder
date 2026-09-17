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
    sign_instructions(rpc, signer, &[ix])
}

pub(crate) fn sign_native(
    rpc: &mut Rpc,
    signer: &Keypair,
    ix: Instruction,
) -> Result<SignedTransaction> {
    sign_native_instructions(rpc, signer, vec![ix])
}

pub(crate) fn sign_native_instructions(
    rpc: &mut Rpc,
    signer: &Keypair,
    mut instructions: Vec<Instruction>,
) -> Result<SignedTransaction> {
    // Native matching and hot/cold account traversal can exceed the per-ix
    // default. No priority fee or network-dependent fee guess is introduced.
    let mut data = vec![2]; // ComputeBudgetInstruction::SetComputeUnitLimit
    data.extend_from_slice(&1_400_000u32.to_le_bytes());
    let budget = anchor_ix(
        bytes("ComputeBudget111111111111111111111111111111")?,
        vec![],
        data,
    );
    instructions.insert(0, budget);
    sign_instructions(rpc, signer, &instructions)
}

fn sign_instructions(
    rpc: &mut Rpc,
    signer: &Keypair,
    instructions: &[Instruction],
) -> Result<SignedTransaction> {
    let latest = rpc.call(
        "getLatestBlockhash",
        json!([{"commitment":"confirmed"}]),
        signer,
    )?;
    let hash = string(&latest["value"]["blockhash"])?
        .parse()
        .map_err(|_| RuntimeError::Decode)?;
    let payer = Pubkey::new_from_array(signer.pubkey().to_bytes());
    let message = Message::new_with_blockhash(instructions, Some(&payer), &hash);
    // Solana 3 exposes signer Address and message Pubkey through different
    // major versions. Sign the canonical message bytes, not a lossy conversion.
    let sig = signer.sign_message(&message.serialize());
    let signature = sig.as_ref().try_into().map_err(|_| RuntimeError::Decode)?;
    let tx = Transaction {
        signatures: vec![sig],
        message,
    };
    let serialized = bincode::serialize(&tx).map_err(|_| RuntimeError::Decode)?;
    if serialized.len() > 1232 || tx.message.header.num_required_signatures != 1 {
        return Err(RuntimeError::Unsupported);
    }
    Ok(SignedTransaction {
        encoded: STANDARD.encode(serialized),
        signature,
        expiry: number(&latest["value"]["lastValidBlockHeight"])?,
    })
}

pub(crate) fn simulate(
    rpc: &mut Rpc,
    signer: &Keypair,
    tx: &SignedTransaction,
    min_slot: u64,
) -> Result<()> {
    let result=rpc.call("simulateTransaction",json!([tx.encoded,{"encoding":"base64","sigVerify":true,"commitment":"confirmed","minContextSlot":min_slot}]),signer)?;
    if result["value"].get("err") != Some(&Value::Null)
        || number(&result["context"]["slot"])? < min_slot
    {
        return Err(RuntimeError::Rpc);
    }
    Ok(())
}
pub(crate) fn send(rpc: &mut Rpc, signer: &Keypair, tx: &SignedTransaction) -> Result<[u8; 64]> {
    let returned=rpc.call("sendTransaction",json!([tx.encoded,{"encoding":"base64","skipPreflight":false,"preflightCommitment":"confirmed","maxRetries":0}]),signer)?;
    let returned = signature(string(&returned)?)?;
    if returned != tx.signature {
        return Err(RuntimeError::Identity);
    }
    Ok(returned)
}

pub(crate) fn send_ioc(
    rpc: &mut Rpc,
    signer: &Keypair,
    tx: &SignedTransaction,
) -> Result<crate::VenueSubmitResult> {
    let response = rpc.call_envelope("sendTransaction", json!([tx.encoded,{"encoding":"base64","skipPreflight":false,"preflightCommitment":"confirmed","maxRetries":0}]), signer)?;
    decode_ioc_send(&response, &tx.signature)
}
fn decode_ioc_send(response: &Value, expected: &[u8; 64]) -> Result<crate::VenueSubmitResult> {
    if !response["error"].is_null() {
        // Solana's SendTransactionPreflightFailure proves this one submission
        // was not broadcast. HTTP/rate-limit/node/transport errors, missing
        // simulation errors and contradictory results remain ambiguous.
        if response.get("result").is_none()
            && response["error"]["code"].as_i64() == Some(-32002)
            && response["error"]["data"]
                .get("err")
                .is_some_and(|e| !e.is_null())
        {
            return Ok(crate::VenueSubmitResult::PreflightRejected);
        }
        return Err(RuntimeError::Rpc);
    }
    let returned = signature(string(&response["result"])?)?;
    if returned != *expected {
        return Err(RuntimeError::Identity);
    }
    Ok(crate::VenueSubmitResult::Accepted(returned))
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

#[cfg(test)]
pub(crate) fn test_receipt(ix: &Instruction, sig: &[u8; 64], succeeded: bool) -> Value {
    let mut keys = vec![ix
        .accounts
        .iter()
        .find(|a| a.is_signer)
        .unwrap()
        .pubkey
        .to_bytes()];
    for meta in &ix.accounts {
        if !keys.contains(&meta.pubkey.to_bytes()) {
            keys.push(meta.pubkey.to_bytes());
        }
    }
    if !keys.contains(&ix.program_id.to_bytes()) {
        keys.push(ix.program_id.to_bytes());
    }
    let index = |key: &[u8; 32]| keys.iter().position(|k| k == key).unwrap();
    json!({"slot":100,"transaction":{"signatures":[bs58::encode(sig).into_string()],"message":{"header":{"numRequiredSignatures":1},"accountKeys":keys.iter().map(crate::runtime::text).collect::<Vec<_>>(),"instructions":[{"programIdIndex":index(&ix.program_id.to_bytes()),"accounts":ix.accounts.iter().map(|a|index(&a.pubkey.to_bytes())).collect::<Vec<_>>(),"data":bs58::encode(&ix.data).into_string()}]}},"meta":{"err":if succeeded {Value::Null} else {json!({"InstructionError":[0,"InvalidAccountData"]})}}})
}

#[cfg(test)]
mod send_tests {
    use super::*;
    #[test]
    fn only_an_explicit_preflight_error_proves_no_broadcast() {
        let sig = [8; 64];
        let rejected = json!({"error":{"code":-32002,"data":{"err":{"InstructionError":[1,"InvalidAccountData"]}}}});
        assert!(matches!(
            decode_ioc_send(&rejected, &sig),
            Ok(crate::VenueSubmitResult::PreflightRejected)
        ));
        for value in [
            json!({"error":{"code":429}}),
            json!({"error":{"code":-32005}}),
            json!({"error":{"code":-32002}}),
            json!({"error":{"code":-32002,"data":{"err":null}}}),
            json!({"result":null,"error":{"code":-32002,"data":{"err":"BlockhashNotFound"}}}),
        ] {
            assert!(decode_ioc_send(&value, &sig).is_err());
        }
        let accepted = json!({"result":bs58::encode(sig).into_string()});
        assert!(
            matches!(decode_ioc_send(&accepted,&sig), Ok(crate::VenueSubmitResult::Accepted(s)) if s==sig)
        );
        assert!(decode_ioc_send(&accepted, &[9; 64]).is_err());
    }
}
