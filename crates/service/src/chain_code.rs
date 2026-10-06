//! Stream and hash approved upgradeable code under authenticated RPC. Bounded
//! chunks, exact headers before/after and pinned bytes; not a consensus proof.
use crate::{
    Error,
    chain_receipt::{self, Deployment},
    chain_rpc::{Accounts, Client, Transport},
};
use cinder_journal::model::PrivateBytes;
use openssl::sha::{Sha256, sha256};
use serde_json::{Value, json};

/// Hard allocation/code bound, not an automatic allowance for any deployed code.
pub const MAX_CODE: u32 = 10_485_760;
const CHUNK: u32 = 65_536;
/// Constructible only after authenticated stream verification. No signing or
/// delivery capability; governed upgrade authority remains a trust assumption.
pub struct Verified {
    deployment: Deployment,
    network: [u8; 32],
    slot: u64,
    at: u64,
    evidence: Value,
}
impl Verified {
    pub(crate) fn matches(
        &self,
        d: &Deployment,
        network: [u8; 32],
        slot: u64,
        transaction_at: u64,
        accounts_at: u64,
    ) -> Result<(), Error> {
        if &self.deployment != d
            || self.network != network
            || self.slot < slot
            || self.at < transaction_at
            || self.at > accounts_at
        {
            return Err(Error);
        }
        Ok(())
    }
    pub(crate) fn evidence(&self) -> &Value {
        &self.evidence
    }
    /// Trusted receive time for a following exact effect-account request.
    pub fn at(&self) -> u64 {
        self.at
    }
}
fn headers(accounts: &Accounts) -> Result<Vec<Vec<u8>>, Error> {
    accounts
        .values
        .iter()
        .map(|a| a.as_ref().map(|a| a.data.as_bytes().to_vec()).ok_or(Error))
        .collect()
}
/// Verify full expected ELF plus zero allocation tail, without accepting a
/// multi-megabyte JSON body or retaining raw code in financial receipts. Every
/// chunk's route/offset/slot/time/hash is retained; the expected ELF is a separately
/// reviewed artifact. TLS/provider and upgrade governance, not hashes alone,
/// authenticate the chain state. No reconnect/retry is performed on any error.
pub fn verify<T: Transport>(
    client: &mut Client<T>,
    d: &Deployment,
    network: [u8; 32],
    minimum_slot: u64,
    at: u64,
) -> Result<Verified, Error> {
    d.validate()?;
    let first = client.accounts_slice(&[d.program, d.data], 0, 45, minimum_slot, at)?;
    if first.network != network {
        return Err(Error);
    }
    chain_receipt::executable_headers(&first, d, minimum_slot)?;
    let expected = headers(&first)?;
    let data = first.values[1].as_ref().ok_or(Error)?;
    let owner = data.owner;
    let mut at = first.at;
    let mut slot = first.slot;
    let mut offset = 0u32;
    let mut hash = Sha256::new();
    let mut chunks = Vec::new();
    loop {
        if offset > MAX_CODE {
            return Err(Error);
        }
        let n = CHUNK.min(
            MAX_CODE
                .checked_sub(offset)
                .ok_or(Error)?
                .checked_add(1)
                .ok_or(Error)?,
        );
        let a = client.accounts_slice(&[d.data], 45 + offset, n, slot, at)?;
        let value = a.values.first().and_then(Option::as_ref).ok_or(Error)?;
        let bytes = value.data.as_bytes();
        if a.network != network
            || value.key != d.data
            || value.owner != owner
            || value.executable
            || value.lamports == 0
            || a.at < at
        {
            return Err(Error);
        }
        let remaining = d.length.saturating_sub(offset) as usize;
        let used = remaining.min(bytes.len());
        if offset == 0 && bytes.get(..4) != Some(b"\x7fELF") {
            return Err(Error);
        }
        hash.update(&bytes[..used]);
        if bytes[used..].iter().any(|b| *b != 0) {
            return Err(Error);
        }
        chunks.push(json!({"offset":45+offset,"length":bytes.len(),"slot":a.slot,"at":a.at,"sha256":sha256(bytes)}));
        at = a.at;
        slot = a.slot;
        offset = offset
            .checked_add(u32::try_from(bytes.len()).map_err(|_| Error)?)
            .ok_or(Error)?;
        if bytes.len() < n as usize {
            break;
        }
    }
    if offset < d.length || offset > MAX_CODE || hash.finish() != d.hash {
        return Err(Error);
    }
    let last = client.accounts_slice(&[d.program, d.data], 0, 45, slot, at)?;
    chain_receipt::executable_headers(&last, d, minimum_slot)?;
    if last.network != network || headers(&last)? != expected || last.at < at {
        return Err(Error);
    }
    // This bounded evidence must fit one encrypted journal record.
    let evidence = json!({"schema":"cinder-streamed-code-v1","network":network,"program":d.program,"data":d.data,
        "length":d.length,"hash":d.hash,"headers":expected,"chunks":chunks,"slot":last.slot,"at":last.at});
    PrivateBytes::new(serde_json::to_vec(&evidence).map_err(|_| Error)?).map_err(|_| Error)?;
    Ok(Verified {
        deployment: d.clone(),
        network,
        slot: last.slot,
        at: last.at,
        evidence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain_rpc::Response;
    use base64::{Engine, engine::general_purpose::STANDARD};
    use std::sync::{Arc, Mutex};

    struct Fake {
        deployment: Deployment,
        bytes: Vec<u8>,
        requests: Arc<Mutex<Vec<Value>>>,
        fault: u8,
        headers: u8,
    }
    impl Transport for Fake {
        fn post(&mut self, body: PrivateBytes) -> Result<Response, Error> {
            let r: Value = serde_json::from_slice(body.as_bytes()).unwrap();
            self.requests.lock().unwrap().push(r.clone());
            let result = if r["method"] == "getGenesisHash" {
                json!(cinder_pacifica::funding::chain::address([1; 32]))
            } else {
                assert_eq!(r["method"], "getMultipleAccounts");
                assert_eq!(r["params"][1]["commitment"], "finalized");
                let offset = r["params"][1]["dataSlice"]["offset"].as_u64().unwrap() as usize;
                let length = r["params"][1]["dataSlice"]["length"].as_u64().unwrap() as usize;
                assert!(length <= 65_536);
                let mut owner = "BPFLoaderUpgradeab1e11111111111111111111111".to_owned();
                if self.fault == 3 && offset >= 45 {
                    owner = cinder_pacifica::funding::chain::address([9; 32]);
                }
                let mut values = vec![];
                for key in r["params"][0].as_array().unwrap() {
                    let program = key
                        == &json!(cinder_pacifica::funding::chain::address(
                            self.deployment.program
                        ));
                    let mut data = if program {
                        [2u32.to_le_bytes().as_slice(), &self.deployment.data].concat()
                    } else {
                        self.bytes.clone()
                    };
                    if offset == 0 && !program {
                        self.headers += 1;
                        if self.fault == 2 && self.headers > 1 {
                            data[13] ^= 1;
                        }
                    }
                    let slice = data.get(offset..).unwrap_or_default();
                    let n = length.min(slice.len());
                    values.push(json!({"owner":owner,"executable":program,"lamports":1000,"data":[STANDARD.encode(&slice[..n]),"base64"]}));
                }
                json!({"context":{"slot":if self.fault==4 && offset>=45 {199}else{200}},"value":values})
            };
            Ok(Response {
                status: 200,
                at: 100,
                body: PrivateBytes::new(
                    serde_json::to_vec(&json!({"jsonrpc":"2.0","id":r["id"],"result":result}))
                        .unwrap(),
                )
                .unwrap(),
            })
        }
    }
    fn case(
        length: usize,
        tail: usize,
        fault: u8,
        budget: u32,
    ) -> (Client<Fake>, Deployment, Arc<Mutex<Vec<Value>>>) {
        let mut elf = vec![7; length];
        elf[..4].copy_from_slice(b"\x7fELF");
        let d = Deployment {
            program: [20; 32],
            data: [60; 32],
            length: length as u32,
            hash: sha256(&elf),
            authority: Some([61; 32]),
        };
        let mut bytes = vec![0; 45];
        bytes[..4].copy_from_slice(&3u32.to_le_bytes());
        bytes[4..12].copy_from_slice(&150u64.to_le_bytes());
        bytes[12] = 1;
        bytes[13..45].copy_from_slice(&[61; 32]);
        bytes.extend(elf);
        bytes.resize(bytes.len() + tail, 0);
        if fault == 1 {
            bytes[45 + length - 1] ^= 1;
        }
        if fault == 5 {
            *bytes.last_mut().unwrap() = 1;
        }
        let requests = Arc::new(Mutex::new(vec![]));
        (
            Client::new(
                [1; 32],
                Fake {
                    deployment: d.clone(),
                    bytes,
                    requests: requests.clone(),
                    fault,
                    headers: 0,
                },
                budget,
            )
            .unwrap(),
            d,
            requests,
        )
    }
    #[test]
    fn large_actual_sized_code_and_zero_allocation_tail_are_streamed_in_bounded_chunks() {
        for (length, tail) in [(64, 0), (65_536, 65_536), (522_200, 522_200)] {
            let (mut c, d, requests) = case(length, tail, 0, 100);
            let verified = verify(&mut c, &d, [1; 32], 200, 100).unwrap();
            verified.matches(&d, [1; 32], 200, 100, 100).unwrap();
            assert!(verified.matches(&d, [2; 32], 200, 100, 100).is_err());
            assert!(verified.matches(&d, [1; 32], 201, 100, 100).is_err());
            assert!(verified.matches(&d, [1; 32], 200, 101, 100).is_err());
            assert!(serde_json::to_vec(verified.evidence()).unwrap().len() < 10_000);
            let requests = requests.lock().unwrap();
            assert_eq!(
                requests
                    .iter()
                    .filter(|r| r["params"][1]["dataSlice"]["offset"] == 0)
                    .count(),
                2
            );
            assert!(
                requests
                    .iter()
                    .filter(|r| r["method"] == "getMultipleAccounts")
                    .count()
                    > 2
            );
        }
    }
    #[test]
    fn changed_code_governance_owner_context_or_nonzero_tail_never_produces_a_proof() {
        for fault in 1..=5 {
            let (mut c, d, _) = case(80_000, 100, fault, 100);
            assert!(
                verify(&mut c, &d, [1; 32], 200, 100).is_err(),
                "fault {fault}"
            );
        }
    }
    #[test]
    fn stream_exhaustion_cannot_retry_or_skip_unread_chunks() {
        let (mut c, d, requests) = case(522_200, 0, 0, 3);
        assert!(verify(&mut c, &d, [1; 32], 200, 100).is_err());
        assert_eq!(requests.lock().unwrap().len(), 3);
        assert!(verify(&mut c, &d, [1; 32], 200, 100).is_err());
        assert_eq!(requests.lock().unwrap().len(), 3);
    }
}
