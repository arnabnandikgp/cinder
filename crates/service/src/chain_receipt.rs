//! Validate original wire + finalized RPC + deployed executable + classic SPL
//! metadata + permanent P15 receipt. No balance setter or invented venue credit.
use crate::{
    Error,
    chain_rpc::{Account, Accounts, Finalized},
};
use cinder_journal::model::PrivateBytes;
use cinder_kernel::identity::AttemptKey;
use cinder_pacifica::funding::{
    ChainReceipt, Counters, Rail, Route,
    chain::{self, Target},
};
use openssl::sha::sha256;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const TOKEN: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const LOADER: &str = "BPFLoaderUpgradeab1e11111111111111111111111";
/// Explicit qualified upgradeable deployment; never inferred from an IDL name.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Deployment {
    /// Expected executable program identity.
    pub program: [u8; 32],
    /// Expected ProgramData pointer.
    pub data: [u8; 32],
    /// Exact ELF byte count, excluding loader metadata and zero allocation tail.
    pub length: u32,
    /// Reviewed ELF hash, not caller-provided account hash as a trust substitute.
    pub hash: [u8; 32],
    /// Explicit current upgrade authority; None requires immutable deployment.
    pub authority: Option<[u8; 32]>,
}
impl Deployment {
    /// Validate source bounds; construction does not qualify a native capability.
    pub fn validate(&self) -> Result<(), Error> {
        if self.program == [0; 32]
            || self.data == [0; 32]
            || self.program == self.data
            || self.hash == [0; 32]
            || self.length < 64
            || self.length > crate::chain_code::MAX_CODE
            || self.authority == Some([0; 32])
        {
            return Err(Error);
        }
        Ok(())
    }
}
/// Ordered deduplicated accounts needed for an exact receipt check. This does
/// not request native history or assert that a chain deposit reached the venue.
pub fn keys(target: &Target, deployment: &Deployment) -> Result<Vec<[u8; 32]>, Error> {
    deployment.validate()?;
    if deployment.program != target.program {
        return Err(Error);
    }
    let mut keys = vec![
        target.program,
        deployment.data,
        target.mint,
        target.source,
        target.destination,
    ];
    if let Some(receipt) = target.receipt {
        keys.extend([target.config, receipt]);
    }
    if let Some(customer) = target.customer_counter {
        keys.push(customer);
    }
    let mut seen = std::collections::BTreeSet::new();
    if keys.iter().any(|k| !seen.insert(*k)) {
        return Err(Error);
    }
    Ok(keys)
}
/// Small receipt accounts only; deployed code is independently streamed and
/// verified by chain_code, rather than embedded in every account response.
pub fn effect_keys(target: &Target, deployment: &Deployment) -> Result<Vec<[u8; 32]>, Error> {
    let mut keys = keys(target, deployment)?;
    keys.drain(..2);
    Ok(keys)
}
pub(crate) fn get(accounts: &Accounts, key: [u8; 32]) -> Result<&Account, Error> {
    let mut matches = accounts
        .values
        .iter()
        .filter_map(Option::as_ref)
        .filter(|a| a.key == key);
    let a = matches.next().ok_or(Error)?;
    if matches.next().is_some() {
        return Err(Error);
    }
    Ok(a)
}
fn owned(a: &Account, owner: [u8; 32], length: usize) -> Result<&[u8], Error> {
    if a.owner != owner || a.executable || a.data.as_bytes().len() != length || a.lamports == 0 {
        return Err(Error);
    }
    Ok(a.data.as_bytes())
}
pub(crate) fn word(data: &[u8], at: usize) -> Result<[u8; 32], Error> {
    data.get(at..at + 32)
        .ok_or(Error)?
        .try_into()
        .map_err(|_| Error)
}
pub(crate) fn number(data: &[u8], at: usize) -> Result<u64, Error> {
    Ok(u64::from_le_bytes(
        data.get(at..at + 8)
            .ok_or(Error)?
            .try_into()
            .map_err(|_| Error)?,
    ))
}
pub(crate) fn discriminated<'a>(
    a: &'a Account,
    program: [u8; 32],
    name: &str,
    length: usize,
) -> Result<&'a [u8], Error> {
    let bytes = owned(a, program, length)?;
    if bytes[..8] != sha256(format!("account:{name}").as_bytes())[..8] {
        return Err(Error);
    }
    Ok(bytes)
}
/// Current finalized executable must be the approved binary and must not have
/// been redeployed after the transaction being recognized. RPC authenticity and
/// upgrade governance remain explicit trust assumptions, not a consensus proof.
pub fn executable(accounts: &Accounts, d: &Deployment, execution_slot: u64) -> Result<(), Error> {
    executable_headers(accounts, d, execution_slot)?;
    let b = get(accounts, d.data)?.data.as_bytes();
    let end = 45 + d.length as usize;
    if b.len() < end
        || &b[45..49] != b"\x7fELF"
        || sha256(&b[45..end]) != d.hash
        || b[end..].iter().any(|b| *b != 0)
    {
        return Err(Error);
    }
    Ok(())
}
pub(crate) fn executable_headers(
    accounts: &Accounts,
    d: &Deployment,
    execution_slot: u64,
) -> Result<(), Error> {
    d.validate()?;
    if execution_slot == 0 || accounts.slot < execution_slot {
        return Err(Error);
    }
    let loader = chain::parse_address(LOADER).map_err(|_| Error)?;
    let program = get(accounts, d.program)?;
    let p = program.data.as_bytes();
    if program.owner != loader
        || !program.executable
        || program.lamports == 0
        || p.len() != 36
        || p[..4] != 2u32.to_le_bytes()
        || word(p, 4)? != d.data
    {
        return Err(Error);
    }
    let data = get(accounts, d.data)?;
    let b = data.data.as_bytes();
    if data.owner != loader
        || data.executable
        || data.lamports == 0
        || b.len() < 45
        || b[..4] != 3u32.to_le_bytes()
        || number(b, 4)? > execution_slot
        || number(b, 4)? == 0
    {
        return Err(Error);
    }
    match d.authority {
        Some(authority) if b[12] == 1 && word(b, 13)? == authority => {}
        None if b[12] == 0 => {}
        _ => return Err(Error),
    }
    Ok(())
}
/// Classic SPL token metadata/authority; never interpret Token-2022 or uiAmount.
pub fn token(account: &Account, mint: [u8; 32], authority: [u8; 32]) -> Result<u64, Error> {
    let b = owned(
        account,
        chain::parse_address(TOKEN).map_err(|_| Error)?,
        165,
    )?;
    if word(b, 0)? != mint
        || word(b, 32)? != authority
        || b[108] != 1
        || b[72..76] != [0; 4]
        || b[109..113] != [0; 4]
        || number(b, 121)? != 0
        || b[129..133] != [0; 4]
    {
        return Err(Error);
    }
    number(b, 64)
}
pub(crate) fn mint(account: &Account, t: &Target) -> Result<(), Error> {
    let b = owned(account, chain::parse_address(TOKEN).map_err(|_| Error)?, 82)?;
    if b[44] != t.decimals || b[45] != 1 {
        return Err(Error);
    }
    Ok(())
}
pub(crate) fn config(account: &Account, t: &Target) -> Result<(), Error> {
    let b = discriminated(account, t.program, "VaultConfig", 333)?;
    // Receipt recognition records a past effect, not permission for a new call.
    // P15's one-way freeze increments the epoch once; later recovery modes must
    // not erase an original finalized receipt. `counters` retains strict current
    // epoch/mode checks before exposing a new action.
    if b[8] != 1
        || b[11] != t.decimals
        || word(b, 12)? != t.domain
        || word(b, 44)? != t.pool
        || word(b, 76)? != t.mint
        || word(b, 140)? != t.funds
        || word(b, 204)? != t.broker
        || word(b, 236)? != t.broker_tokens
        || number(b, 268)?.checked_sub(t.epoch).is_none_or(|d| d > 1)
        || b[276] > 4
    {
        return Err(Error);
    }
    Ok(())
}
fn movement(account: &Account, t: &Target) -> Result<(), Error> {
    let b = discriminated(account, t.program, "MovementReceipt", 161)?;
    let (kind, owner, sequence) = match t.rail {
        Rail::Release => (1, t.funds, t.sequence.checked_add(1).ok_or(Error)?),
        Rail::Return => (2, t.broker, 0),
        Rail::Payout => (3, t.customer, t.sequence.checked_add(1).ok_or(Error)?),
        _ => return Err(Error),
    };
    if word(b, 8)? != t.config
        || word(b, 40)? != t.operation
        || number(b, 72)? != t.epoch
        || b[80] != kind
        || word(b, 81)? != owner
        || word(b, 113)? != t.destination
        || number(b, 145)? != t.amount
        || number(b, 153)? != sequence
    {
        return Err(Error);
    }
    Ok(())
}
fn customer(account: &Account, t: &Target) -> Result<(), Error> {
    let b = discriminated(account, t.program, "CustomerCounter", 97)?;
    if word(b, 8)? != t.config
        || word(b, 40)? != t.customer
        || number(b, 81)? < t.paid.checked_add(t.amount).ok_or(Error)?
        || number(b, 89)? < t.sequence.checked_add(1).ok_or(Error)?
    {
        return Err(Error);
    }
    Ok(())
}
/// Decode already-finalized custody preconditions before exposing a new action.
/// A stale counter causes the program to reject; it cannot reset lifetime paid.
pub fn counters(
    accounts: &Accounts,
    route: &Route,
    rail: Rail,
    wallet: [u8; 32],
    tokens: [u8; 32],
    epoch: u64,
    expires_at_slot: u64,
) -> Result<Counters, Error> {
    let config_key = chain::pda(
        route.program,
        &[b"cinder_vault", &route.domain, &route.pool, &route.mint],
    )
    .map_err(|_| Error)?;
    if config_key != route.config
        || chain::pda(route.program, &[b"tokens", &route.config]).map_err(|_| Error)? != route.vault
        || expires_at_slot == 0
    {
        return Err(Error);
    }
    let b = discriminated(
        get(accounts, route.config)?,
        route.program,
        "VaultConfig",
        333,
    )?;
    if b[8] != 1
        || b[11] != route.decimals
        || word(b, 12)? != route.domain
        || word(b, 44)? != route.pool
        || word(b, 76)? != route.mint
        || word(b, 140)? != route.funds
        || word(b, 204)? != route.broker
        || word(b, 236)? != route.broker_tokens
        || number(b, 268)? != epoch
        || (rail == Rail::Return && !matches!(b[276], 0 | 1))
        || (rail != Rail::Return && b[276] != 0)
    {
        return Err(Error);
    }
    let (paid, sequence, recipient_tokens) = if rail == Rail::Payout {
        let key =
            chain::pda(route.program, &[b"customer", &route.config, &wallet]).map_err(|_| Error)?;
        let c = discriminated(get(accounts, key)?, route.program, "CustomerCounter", 97)?;
        if word(c, 8)? != route.config || word(c, 40)? != wallet {
            return Err(Error);
        }
        token(get(accounts, tokens)?, route.mint, wallet)?;
        (number(c, 81)?, number(c, 89)?, tokens)
    } else {
        (
            0,
            if rail == Rail::Release {
                number(b, 301)?
            } else {
                0
            },
            [0; 32],
        )
    };
    Ok(Counters {
        paid,
        sequence,
        recipient_tokens,
        expires_at_slot,
    })
}
fn atoms(v: &Value) -> Result<u64, Error> {
    let s = v.as_str().ok_or(Error)?;
    if s.is_empty()
        || s.len() > 20
        || s.len() > 1 && s.starts_with('0')
        || !s.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(Error);
    }
    s.parse().map_err(|_| Error)
}
pub(crate) fn balance(
    rows: &Value,
    t: &Target,
    key: [u8; 32],
    owner: [u8; 32],
) -> Result<u64, Error> {
    let rows = rows.as_array().ok_or(Error)?;
    if rows.len() > 32 {
        return Err(Error);
    }
    let index = t.keys.iter().position(|k| *k == key).ok_or(Error)?;
    let mut found = None;
    let mut indices = std::collections::BTreeSet::new();
    for row in rows {
        let i = row["accountIndex"].as_u64().ok_or(Error)?;
        if i >= t.keys.len() as u64 || !indices.insert(i) {
            return Err(Error);
        }
        if i != index as u64 {
            continue;
        }
        if row["mint"] != chain::address(t.mint)
            || row["owner"] != chain::address(owner)
            || row["uiTokenAmount"]["decimals"].as_u64() != Some(u64::from(t.decimals))
        {
            return Err(Error);
        }
        found = Some(atoms(&row["uiTokenAmount"]["amount"])?);
    }
    found.ok_or(Error)
}
/// Recognize exact finalized chain effects. Both inputs must come from this
/// release's enclave-authenticated RPC, never an HTTP caller or parent callback.
/// Native deposit reports only chain debit; it DOES NOT construct Credit.
pub fn recognize(
    contract: &[u8],
    attempt: AttemptKey,
    original_wire: &[u8],
    finalized: Finalized,
    accounts: Accounts,
    deployment: &Deployment,
    maximum_fee_lamports: u64,
) -> Result<ChainReceipt, Error> {
    recognize_inner(
        contract,
        attempt,
        original_wire,
        finalized,
        accounts,
        deployment,
        maximum_fee_lamports,
        None,
    )
}
/// Actual bounded-RPC path: approved code evidence is obtained independently
/// through streamed authenticated reads; no host-provided hash is accepted.
#[allow(clippy::too_many_arguments)]
pub fn recognize_verified(
    contract: &[u8],
    attempt: AttemptKey,
    original_wire: &[u8],
    finalized: Finalized,
    accounts: Accounts,
    deployment: &Deployment,
    maximum_fee_lamports: u64,
    code: crate::chain_code::Verified,
) -> Result<ChainReceipt, Error> {
    recognize_inner(
        contract,
        attempt,
        original_wire,
        finalized,
        accounts,
        deployment,
        maximum_fee_lamports,
        Some(code),
    )
}
#[allow(clippy::too_many_arguments)]
fn recognize_inner(
    contract: &[u8],
    attempt: AttemptKey,
    original_wire: &[u8],
    finalized: Finalized,
    accounts: Accounts,
    deployment: &Deployment,
    maximum_fee_lamports: u64,
    code: Option<crate::chain_code::Verified>,
) -> Result<ChainReceipt, Error> {
    let t = chain::inspect(contract, attempt, original_wire).map_err(|_| Error)?;
    if finalized.network != t.network
        || accounts.network != t.network
        || finalized.wire.as_bytes() != original_wire
        || finalized.slot == 0
        || accounts.slot < finalized.slot
        || accounts.at < finalized.at
        || maximum_fee_lamports == 0
        || finalized.fee_lamports > maximum_fee_lamports
    {
        return Err(Error);
    }
    let err = finalized.meta.get("err").ok_or(Error)?;
    let succeeded = err.is_null();
    if !succeeded
        && !(err.as_str().is_some_and(|s| !s.is_empty())
            || err.as_object().is_some_and(|m| m.len() == 1))
    {
        return Err(Error);
    }
    if succeeded {
        if let Some(code) = &code {
            code.matches(
                deployment,
                t.network,
                finalized.slot,
                finalized.at,
                accounts.at,
            )?;
        } else {
            executable(&accounts, deployment, finalized.slot)?;
        }
        mint(get(&accounts, t.mint)?, &t)?;
        token(get(&accounts, t.source)?, t.mint, t.source_owner)?;
        token(get(&accounts, t.destination)?, t.mint, t.destination_owner)?;
        let pre = &finalized.meta["preTokenBalances"];
        let post = &finalized.meta["postTokenBalances"];
        if balance(pre, &t, t.source, t.source_owner)?.checked_sub(balance(
            post,
            &t,
            t.source,
            t.source_owner,
        )?) != Some(t.amount)
            || balance(post, &t, t.destination, t.destination_owner)?.checked_sub(balance(
                pre,
                &t,
                t.destination,
                t.destination_owner,
            )?) != Some(t.amount)
        {
            return Err(Error);
        }
        if let Some(receipt) = t.receipt {
            config(get(&accounts, t.config)?, &t)?;
            movement(get(&accounts, receipt)?, &t)?;
        }
        if let Some(key) = t.customer_counter {
            customer(get(&accounts, key)?, &t)?;
        }
    } else {
        // Failure is atomic but still charged SOL. Reject contradictory token
        // effects; do not reinterpret a missing/unknown transaction as failure.
        let pre = finalized.meta["preTokenBalances"].as_array().ok_or(Error)?;
        let post = finalized.meta["postTokenBalances"]
            .as_array()
            .ok_or(Error)?;
        type Balances = std::collections::BTreeMap<u64, (String, String, u64, u64)>;
        let norm = |rows: &[Value]| -> Result<Balances, Error> {
            let mut out = std::collections::BTreeMap::new();
            for row in rows {
                let i = row["accountIndex"].as_u64().ok_or(Error)?;
                let mint = row["mint"].as_str().ok_or(Error)?.to_owned();
                let owner = row["owner"].as_str().ok_or(Error)?.to_owned();
                let amount = atoms(&row["uiTokenAmount"]["amount"])?;
                let decimals = row["uiTokenAmount"]["decimals"].as_u64().ok_or(Error)?;
                if i >= t.keys.len() as u64
                    || out.insert(i, (mint, owner, decimals, amount)).is_some()
                {
                    return Err(Error);
                }
            }
            Ok(out)
        };
        if norm(pre)? != norm(post)? {
            return Err(Error);
        }
    }
    let raw = PrivateBytes::new(
        serde_json::to_vec(&json!({"schema":"cinder-chain-receipt-v1",
        "transaction":serde_json::from_slice::<Value>(finalized.raw.as_bytes()).map_err(|_|Error)?,
        "accounts":serde_json::from_slice::<Value>(accounts.raw.as_bytes()).map_err(|_|Error)?,
        "program_hash":deployment.hash,"code":code.as_ref().map(|c|c.evidence()),
        "actual_fee_lamports":finalized.fee_lamports}))
        .map_err(|_| Error)?,
    )
    .map_err(|_| Error)?;
    let sequence = if succeeded && matches!(t.rail, Rail::Release | Rail::Payout) {
        Some(t.sequence.checked_add(1).ok_or(Error)?)
    } else {
        None
    };
    Ok(ChainReceipt {
        network: t.network,
        attempt,
        mint: t.mint,
        program: t.program,
        source: t.source,
        destination: t.destination,
        amount: if succeeded { t.amount } else { 0 },
        succeeded,
        signature: t.verified.signature,
        slot: finalized.slot,
        operation: if succeeded && t.receipt.is_some() {
            Some(t.operation)
        } else {
            None
        },
        epoch: if succeeded && t.receipt.is_some() {
            Some(t.epoch)
        } else {
            None
        },
        config: if succeeded && t.receipt.is_some() {
            Some(t.config)
        } else {
            None
        },
        paid: if succeeded && t.rail == Rail::Payout {
            Some(t.paid.checked_add(t.amount).ok_or(Error)?)
        } else {
            None
        },
        sequence,
        raw,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use cinder_kernel::identity::*;
    pub(crate) struct Case {
        pub(crate) contract: Vec<u8>,
        pub(crate) attempt: AttemptKey,
        pub(crate) wire: Vec<u8>,
        pub(crate) tx: Finalized,
        pub(crate) accounts: Accounts,
        pub(crate) deployment: Deployment,
    }
    fn field(b: &mut [u8], at: usize, x: &[u8]) {
        b[at..at + x.len()].copy_from_slice(x);
    }
    fn account(key: [u8; 32], owner: [u8; 32], data: Vec<u8>, executable: bool) -> Account {
        Account {
            key,
            owner,
            executable,
            lamports: 1_000_000,
            data: PrivateBytes::new(data).unwrap(),
        }
    }
    fn raw() -> PrivateBytes {
        PrivateBytes::new(
            br#"{"source":"OFFLINE synthetic trusted port; not actual RPC"}"#.to_vec(),
        )
        .unwrap()
    }
    fn row(t: &Target, key: [u8; 32], owner: [u8; 32], amount: u64) -> Value {
        json!({"accountIndex":t.keys.iter().position(|k|*k==key).unwrap(),"mint":chain::address(t.mint),
            "owner":chain::address(owner),"uiTokenAmount":{"amount":amount.to_string(),"decimals":t.decimals,"uiAmount":0.0}})
    }
    fn fixture(i: usize) -> Case {
        let v = serde_json::from_str::<Value>(include_str!(
            "../../pacifica/tests/fixtures/funding-codec-public.json"
        ))
        .unwrap()["vectors"][i]
            .clone();
        let contract = serde_json::to_vec(&v["contract"]).unwrap();
        let wire = v["wire"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u8)
            .collect::<Vec<_>>();
        let attempt = AttemptKey {
            request: RequestKey {
                domain: Domain {
                    network: NetworkId::new([1; 32]).unwrap(),
                    deployment: DeploymentId::new([2; 32]).unwrap(),
                },
                account: AccountId::new([11; 32]).unwrap(),
                request: RequestId::new([12; 32]).unwrap(),
            },
            attempt: AttemptId::new([13; 32]).unwrap(),
        };
        fixture_from(contract, attempt, wire)
    }
    // Shared synthetic chain effect producer for joined controller tests. It
    // provides no executable capability or production receipt constructor.
    pub(crate) fn fixture_from(contract: Vec<u8>, attempt: AttemptKey, wire: Vec<u8>) -> Case {
        let t = chain::inspect(&contract, attempt, &wire).unwrap();
        fixture_target(t, contract, attempt, wire)
    }
    pub(crate) fn fixture_target(
        t: Target,
        contract: Vec<u8>,
        attempt: AttemptKey,
        wire: Vec<u8>,
    ) -> Case {
        let mut elf = vec![0; 64];
        field(&mut elf, 0, b"\x7fELF");
        let deployment = Deployment {
            program: t.program,
            data: [60; 32],
            length: 64,
            hash: sha256(&elf),
            authority: Some([61; 32]),
        };
        let loader = chain::parse_address(LOADER).unwrap();
        let token_program = chain::parse_address(TOKEN).unwrap();
        let mut p = vec![0; 36];
        field(&mut p, 0, &2u32.to_le_bytes());
        field(&mut p, 4, &deployment.data);
        let mut d = vec![0; 45 + elf.len() + 10];
        field(&mut d, 0, &3u32.to_le_bytes());
        field(&mut d, 4, &150u64.to_le_bytes());
        d[12] = 1;
        field(&mut d, 13, &[61; 32]);
        field(&mut d, 45, &elf);
        let mut values = vec![
            Some(account(t.program, loader, p, true)),
            Some(account(deployment.data, loader, d, false)),
        ];
        let mut m = vec![0; 82];
        m[44] = t.decimals;
        m[45] = 1;
        values.push(Some(account(t.mint, token_program, m, false)));
        for (key, authority) in [
            (t.source, t.source_owner),
            (t.destination, t.destination_owner),
        ] {
            let mut token = vec![0; 165];
            field(&mut token, 0, &t.mint);
            field(&mut token, 32, &authority);
            token[108] = 1;
            values.push(Some(account(key, token_program, token, false)));
        }
        if let Some(receipt) = t.receipt {
            let mut config = vec![0; 333];
            field(&mut config, 0, &sha256(b"account:VaultConfig")[..8]);
            config[8] = 1;
            config[11] = t.decimals;
            for (at, x) in [
                (12, t.domain),
                (44, t.pool),
                (76, t.mint),
                (140, t.funds),
                (204, t.broker),
                (236, t.broker_tokens),
            ] {
                field(&mut config, at, &x);
            }
            field(&mut config, 268, &t.epoch.to_le_bytes());
            values.push(Some(account(t.config, t.program, config, false)));
            let mut r = vec![0; 161];
            field(&mut r, 0, &sha256(b"account:MovementReceipt")[..8]);
            field(&mut r, 8, &t.config);
            field(&mut r, 40, &t.operation);
            field(&mut r, 72, &t.epoch.to_le_bytes());
            let (kind, owner, sequence) = match t.rail {
                Rail::Release => (1, t.funds, t.sequence + 1),
                Rail::Return => (2, t.broker, 0),
                Rail::Payout => (3, t.customer, t.sequence + 1),
                Rail::Deposit => (0, t.customer, 0),
                _ => unreachable!(),
            };
            r[80] = kind;
            field(&mut r, 81, &owner);
            field(&mut r, 113, &t.destination);
            field(&mut r, 145, &t.amount.to_le_bytes());
            field(&mut r, 153, &sequence.to_le_bytes());
            values.push(Some(account(receipt, t.program, r, false)));
        }
        if let Some(counter) = t.customer_counter {
            let mut c = vec![0; 97];
            field(&mut c, 0, &sha256(b"account:CustomerCounter")[..8]);
            field(&mut c, 8, &t.config);
            field(&mut c, 40, &t.customer);
            if t.rail == Rail::Deposit {
                field(&mut c, 73, &t.amount.to_le_bytes());
            } else {
                field(&mut c, 81, &(t.paid + t.amount).to_le_bytes());
                field(&mut c, 89, &(t.sequence + 1).to_le_bytes());
            }
            values.push(Some(account(counter, t.program, c, false)));
        }
        let pre = json!([
            row(&t, t.source, t.source_owner, t.amount + 100),
            row(&t, t.destination, t.destination_owner, 200)
        ]);
        let post = json!([
            row(&t, t.source, t.source_owner, 100),
            row(&t, t.destination, t.destination_owner, t.amount + 200)
        ]);
        let tx = Finalized {
            network: [1; 32],
            slot: 200,
            wire: PrivateBytes::new(wire.clone()).unwrap(),
            meta: json!({"err":null,"preTokenBalances":pre,"postTokenBalances":post}),
            raw: raw(),
            at: 300,
            fee_lamports: 5000,
        };
        Case {
            contract,
            attempt,
            wire,
            tx,
            accounts: Accounts {
                network: [1; 32],
                slot: 201,
                values,
                raw: raw(),
                at: 301,
            },
            deployment,
        }
    }
    fn recognized(c: Case) -> Result<ChainReceipt, Error> {
        recognize(
            &c.contract,
            c.attempt,
            &c.wire,
            c.tx,
            c.accounts,
            &c.deployment,
            6000,
        )
    }
    fn change_account(c: &mut Case, key: [u8; 32], f: impl FnOnce(&mut Account)) {
        f(c.accounts
            .values
            .iter_mut()
            .filter_map(Option::as_mut)
            .find(|a| a.key == key)
            .unwrap());
    }
    #[test]
    fn all_original_rails_recognize_only_finalized_exact_token_and_receipt_effects() {
        for i in 0..8 {
            let c = fixture(i);
            let t = chain::inspect(&c.contract, c.attempt, &c.wire).unwrap();
            assert_eq!(
                keys(&t, &c.deployment).unwrap(),
                c.accounts
                    .values
                    .iter()
                    .map(|a| a.as_ref().unwrap().key)
                    .collect::<Vec<_>>()
            );
            let r = recognized(c).unwrap();
            assert!(r.succeeded);
            assert_eq!(r.amount, t.amount);
            assert_eq!(r.signature, t.verified.signature);
            if t.rail == Rail::Deposit {
                assert_eq!(r.operation, None);
                assert_eq!(r.sequence, None);
            }
            // No venue credit constructed.
            else {
                assert_eq!(r.operation, Some(t.operation));
                assert_eq!(r.epoch, Some(t.epoch));
            }
            if t.rail == Rail::Payout {
                assert_eq!(r.paid, Some(t.paid + t.amount));
            }
        }
    }
    #[test]
    fn wrong_network_wire_clock_source_context_and_actual_fee_fail_closed() {
        for kind in 0..6 {
            let mut c = fixture(0);
            match kind {
                0 => c.tx.network = [2; 32],
                1 => c.accounts.network = [2; 32],
                2 => c.tx.wire = PrivateBytes::new(vec![0; 500]).unwrap(),
                3 => c.accounts.slot = 199,
                4 => c.accounts.at = 299,
                5 => c.tx.fee_lamports = 6001,
                _ => unreachable!(),
            }
            assert!(recognized(c).is_err());
        }
    }
    #[test]
    fn programdata_pointer_binary_owner_authority_padding_and_upgrade_slot_are_checked() {
        for kind in 0..7 {
            let mut c = fixture(0);
            let key = if kind == 0 {
                c.deployment.program
            } else {
                c.deployment.data
            };
            change_account(&mut c, key, |a| {
                let mut b = a.data.as_bytes().to_vec();
                match kind {
                    0 => b[4] ^= 1,
                    1 => a.owner = [31; 32],
                    2 => b[45] ^= 1,
                    3 => b[13] ^= 1,
                    4 => b[45 + 64] = 1,
                    5 => field(&mut b, 4, &201u64.to_le_bytes()),
                    6 => a.executable = true,
                    _ => unreachable!(),
                }
                a.data = PrivateBytes::new(b).unwrap();
            });
            assert!(recognized(c).is_err());
        }
    }
    #[test]
    fn spl_token_type_mint_authority_state_delegate_close_and_decimals_are_checked() {
        for kind in 0..8 {
            let mut c = fixture(0);
            let t = chain::inspect(&c.contract, c.attempt, &c.wire).unwrap();
            let key = if kind == 7 { t.mint } else { t.destination };
            change_account(&mut c, key, |a| {
                let mut b = a.data.as_bytes().to_vec();
                match kind {
                    0 => a.owner = [31; 32],
                    1 => b[0] ^= 1,
                    2 => b[32] ^= 1,
                    3 => b[108] = 2,
                    4 => b[72] = 1,
                    5 => b[129] = 1,
                    6 => b.push(0),
                    7 => b[44] += 1,
                    _ => unreachable!(),
                }
                a.data = PrivateBytes::new(b).unwrap();
            });
            assert!(recognized(c).is_err());
        }
    }
    #[test]
    fn immutable_receipt_domain_amount_sequence_and_customer_counter_cannot_be_substituted() {
        for kind in 0..7 {
            let mut c = fixture(6);
            let t = chain::inspect(&c.contract, c.attempt, &c.wire).unwrap();
            let key = if kind == 6 {
                t.customer_counter.unwrap()
            } else if kind == 5 {
                t.config
            } else {
                t.receipt.unwrap()
            };
            change_account(&mut c, key, |a| {
                let mut b = a.data.as_bytes().to_vec();
                let at = match kind {
                    0 => 0,
                    1 => 8,
                    2 => 40,
                    3 => 145,
                    4 => 153,
                    5 => 12,
                    6 => 81,
                    _ => unreachable!(),
                };
                if kind == 6 {
                    field(&mut b, at, &0u64.to_le_bytes());
                } else {
                    b[at] ^= 1;
                }
                a.data = PrivateBytes::new(b).unwrap();
            });
            assert!(recognized(c).is_err());
        }
    }
    #[test]
    fn completed_original_effects_survive_later_freeze_but_new_actions_stay_fenced() {
        for i in [0, 4, 6] {
            for mode in 1..=4 {
                let mut c = fixture(i);
                let t = chain::inspect(&c.contract, c.attempt, &c.wire).unwrap();
                change_account(&mut c, t.config, |a| {
                    let mut b = a.data.as_bytes().to_vec();
                    field(&mut b, 268, &(t.epoch + 1).to_le_bytes());
                    b[276] = mode;
                    a.data = PrivateBytes::new(b).unwrap();
                });
                let route = Route {
                    domain: t.domain,
                    pool: t.pool,
                    funds: t.funds,
                    beneficiaries: vec![],
                    program: t.program,
                    config: t.config,
                    vault: t.vault,
                    mint: t.mint,
                    broker: t.broker,
                    broker_tokens: t.broker_tokens,
                    venue_program: [25; 32],
                    venue_vault: [26; 32],
                    epoch: t.epoch,
                    decimals: t.decimals,
                    withdrawal: cinder_pacifica::profile::Level::Qualified,
                    chain: cinder_pacifica::profile::Level::Qualified,
                    settings: cinder_pacifica::profile::Level::Qualified,
                    withdrawal_cost: 120,
                    maximum_movement: u64::MAX,
                    maximum_fee: 0,
                    setup_max_age: 1000,
                };
                assert!(
                    counters(
                        &c.accounts,
                        &route,
                        t.rail,
                        t.customer,
                        t.destination,
                        t.epoch,
                        1000
                    )
                    .is_err()
                );
                assert!(config(get(&c.accounts, t.config).unwrap(), &t).is_ok());
                let r = recognized(c).unwrap();
                assert!(r.succeeded);
                assert_eq!(r.epoch, Some(t.epoch));
            }
        }
        let mut c = fixture(0);
        let t = chain::inspect(&c.contract, c.attempt, &c.wire).unwrap();
        change_account(&mut c, t.config, |a| {
            let mut b = a.data.as_bytes().to_vec();
            field(&mut b, 268, &0u64.to_le_bytes());
            a.data = PrivateBytes::new(b).unwrap();
        });
        assert!(recognized(c).is_err());
        for fault in [0, 1] {
            let mut c = fixture(0);
            let t = chain::inspect(&c.contract, c.attempt, &c.wire).unwrap();
            change_account(&mut c, t.config, |a| {
                let mut b = a.data.as_bytes().to_vec();
                if fault == 0 {
                    field(&mut b, 268, &(t.epoch + 2).to_le_bytes());
                } else {
                    b[276] = 5;
                }
                a.data = PrivateBytes::new(b).unwrap();
            });
            assert!(recognized(c).is_err());
        }
    }
    #[test]
    fn transaction_specific_deltas_reject_missing_duplicate_wrong_owner_or_rounded_amounts() {
        for kind in 0..6 {
            let mut c = fixture(0);
            match kind {
                0 => c.tx.meta["postTokenBalances"][0]["uiTokenAmount"]["amount"] = json!("101"),
                1 => c.tx.meta["postTokenBalances"][1]["owner"] = json!(chain::address([31; 32])),
                2 => c.tx.meta["preTokenBalances"] = json!([]),
                3 => {
                    let duplicate = c.tx.meta["postTokenBalances"][0].clone();
                    c.tx.meta["postTokenBalances"]
                        .as_array_mut()
                        .unwrap()
                        .push(duplicate);
                }
                4 => {
                    c.tx.meta["postTokenBalances"][1]["uiTokenAmount"]["amount"] =
                        json!(9007199254741193u64)
                }
                5 => c.tx.meta["preTokenBalances"][0]["uiTokenAmount"]["amount"] = json!("01"),
                _ => unreachable!(),
            }
            assert!(recognized(c).is_err());
        }
    }
    #[test]
    fn finalized_failed_transaction_has_zero_token_effect_but_retains_actual_sol_fee() {
        let mut c = fixture(0);
        c.tx.meta["err"] = json!({"InstructionError":[0,{"Custom":6000}]});
        c.tx.meta["postTokenBalances"] = c.tx.meta["preTokenBalances"].clone();
        let r = recognized(c).unwrap();
        assert!(!r.succeeded);
        assert_eq!(r.amount, 0);
        assert_eq!(r.operation, None);
        assert_eq!(
            serde_json::from_slice::<Value>(r.raw.as_bytes()).unwrap()["actual_fee_lamports"],
            5000
        );
        let mut c = fixture(0);
        c.tx.meta["err"] = json!({"InstructionError":[0,{"Custom":6000}]});
        assert!(recognized(c).is_err());
        let mut c = fixture(0);
        c.tx.meta["err"] = json!(false);
        assert!(recognized(c).is_err());
    }
}
