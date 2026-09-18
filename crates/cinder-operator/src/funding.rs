//! Unsigned atomic funding construction and canonical receipt authentication.
//! Finality, freshness, signing, outbox persistence and Book synchronization
//! remain coordinator obligations; these helpers never send transactions.
use crate::rpc::{Result, RuntimeError};
use crate::transaction::{anchor_ix, bytes};
use crate::{FundingIntent, RuntimeConfig};
use anchor_lang::{AccountDeserialize, InstructionData, Space, ToAccountMetas};
use phoenix_rise_accounts::global_config::GlobalConfig;
use phoenix_rise_ix::constants::*;
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;
use std::collections::BTreeSet;

const TOKEN: Pubkey = solana_pubkey::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const ASSOCIATED_TOKEN: Pubkey =
    solana_pubkey::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

fn anchor(key: Pubkey) -> anchor_lang::prelude::Pubkey {
    anchor_lang::prelude::Pubkey::new_from_array(key.to_bytes())
}

fn vault_program() -> Pubkey {
    Pubkey::new_from_array(cinder_vault::ID.to_bytes())
}
fn vault_authority() -> Pubkey {
    Pubkey::find_program_address(&[cinder_common::SEED_VAULT_AUTHORITY], &vault_program()).0
}
fn ata(mint: Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[vault_authority().as_ref(), TOKEN.as_ref(), mint.as_ref()],
        &ASSOCIATED_TOKEN,
    )
    .0
}
fn receipt_address(id: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[cinder_vault::SEED_PHOENIX_FUNDING, id], &vault_program())
}

/// Exact native and Cinder custody binding, decoded from owner-checked global
/// metadata and the L1 vault configuration. No caller-supplied CPI instruction.
pub struct PhoenixFundingAccounts {
    operator: [u8; 32],
    phoenix_program: Pubkey,
    global: Pubkey,
    log: Pubkey,
    trader: Pubkey,
    usdc_mint: Pubkey,
    quote_mint: Pubkey,
    global_vault: Pubkey,
    trader_index: Vec<Pubkey>,
    buffer: Vec<Pubkey>,
}

impl PhoenixFundingAccounts {
    /// The caller must authenticate the vault Config owner/address and obtain
    /// both configurations coherently. This proves identity, not freshness.
    pub fn from_global(
        config: &RuntimeConfig,
        vault: &cinder_vault::Config,
        address: [u8; 32],
        owner: [u8; 32],
        data: &[u8],
    ) -> Result<Self> {
        config.validate()?;
        let program = Pubkey::new_from_array(bytes(&config.phoenix_program)?);
        let global_key = Pubkey::new_from_array(bytes(&config.phoenix_global_config)?);
        let quote = Pubkey::new_from_array(bytes(&config.phoenix_quote_mint)?);
        let trader = Pubkey::new_from_array(bytes(&config.phoenix_trader)?);
        let usdc = Pubkey::new_from_array(vault.usdc_mint.to_bytes());
        if address != global_key.to_bytes()
            || owner != program.to_bytes()
            || vault.vault_authority.to_bytes() != vault_authority().to_bytes()
            || vault.phoenix_trader.to_bytes() != trader.to_bytes()
            || vault.vault_usdc_ata.to_bytes() != ata(usdc).to_bytes()
            || usdc == quote
        {
            return Err(RuntimeError::Identity);
        }
        let global =
            GlobalConfig::try_from_account_bytes(data).map_err(|_| RuntimeError::Decode)?;
        let trader_index = config
            .global_trader_index
            .iter()
            .map(|s| crate::transaction::key(s))
            .collect::<Result<Vec<_>>>()?;
        let buffer = config
            .active_trader_buffer
            .iter()
            .map(|s| crate::transaction::key(s))
            .collect::<Result<Vec<_>>>()?;
        if global.account_key() != address
            || global.quote_decimals() != 6
            || global.canonical_token_mint_key() != quote.to_bytes()
            || global.global_trader_index_header_key() != trader_index[0].to_bytes()
            || global.active_trader_buffer_header_key() != buffer[0].to_bytes()
        {
            return Err(RuntimeError::Identity);
        }
        let log = if program == PROD_PHOENIX_PROGRAM_ID {
            PROD_PHOENIX_LOG_AUTHORITY
        } else {
            BETA_PHOENIX_LOG_AUTHORITY
        };
        Ok(Self {
            operator: vault.adapter.to_bytes(),
            phoenix_program: program,
            global: global_key,
            log,
            trader,
            usdc_mint: usdc,
            quote_mint: quote,
            global_vault: Pubkey::new_from_array(global.global_vault_key()),
            trader_index,
            buffer,
        })
    }
}

pub fn build_phoenix_funding(
    intent: &FundingIntent,
    operator: [u8; 32],
    accounts: &PhoenixFundingAccounts,
) -> Result<Instruction> {
    if intent.amount == 0
        || operator == [0; 32]
        || operator != accounts.operator
        || intent.phoenix_trader != accounts.trader.to_bytes()
        || intent.phoenix_program != accounts.phoenix_program.to_bytes()
        || *intent
            != FundingIntent::new(
                intent.operation_id,
                intent.amount,
                intent.phoenix_trader,
                intent.phoenix_program,
            )
    {
        return Err(RuntimeError::Identity);
    }
    let program = accounts.phoenix_program;
    let ember_state =
        Pubkey::find_program_address(&[program.as_ref(), b"state"], &EMBER_PROGRAM_ID).0;
    let ember_vault =
        Pubkey::find_program_address(&[program.as_ref(), b"vault"], &EMBER_PROGRAM_ID).0;
    let metas = cinder_vault::accounts::FundPhoenix {
        adapter: anchor(Pubkey::new_from_array(operator)),
        config: anchor(
            Pubkey::find_program_address(&[cinder_common::SEED_CONFIG], &vault_program()).0,
        ),
        vault_authority: anchor(vault_authority()),
        funding_receipt: anchor(receipt_address(&intent.funding_id).0),
        usdc_mint: anchor(accounts.usdc_mint),
        vault_usdc_ata: anchor(ata(accounts.usdc_mint)),
        phoenix_quote_mint: anchor(accounts.quote_mint),
        vault_phoenix_ata: anchor(ata(accounts.quote_mint)),
        ember_program: anchor(EMBER_PROGRAM_ID),
        ember_state: anchor(ember_state),
        ember_vault: anchor(ember_vault),
        phoenix_program: anchor(program),
        phoenix_log_authority: anchor(accounts.log),
        phoenix_global_config: anchor(accounts.global),
        phoenix_trader: anchor(accounts.trader),
        phoenix_global_vault: anchor(accounts.global_vault),
        token_program: anchor(TOKEN),
        system_program: anchor(Pubkey::default()),
    }
    .to_account_metas(None);
    let mut keys = metas
        .into_iter()
        .map(|m| (m.pubkey.to_bytes(), m.is_signer, m.is_writable))
        .collect::<Vec<_>>();
    let mut distinct = keys.iter().map(|k| k.0).collect::<BTreeSet<_>>();
    if distinct.len() != keys.len() {
        return Err(RuntimeError::Identity);
    }
    for key in accounts.trader_index.iter().chain(&accounts.buffer) {
        if !distinct.insert(key.to_bytes()) {
            return Err(RuntimeError::Identity);
        }
        keys.push((key.to_bytes(), false, true));
    }
    Ok(anchor_ix(
        vault_program().to_bytes(),
        keys,
        cinder_vault::instruction::FundPhoenix {
            funding_id: intent.funding_id,
            amount: intent.amount,
            global_trader_index_count: u8::try_from(accounts.trader_index.len())
                .map_err(|_| RuntimeError::Decode)?,
        }
        .data(),
    ))
}

/// Authenticate the account fetched with finalized commitment. Absence, an
/// expired transaction or a failed lookup never proves the funding failed.
pub fn decode_phoenix_funding_receipt(
    address: [u8; 32],
    owner: [u8; 32],
    data: &[u8],
    intent: &FundingIntent,
) -> Result<cinder_vault::PhoenixFundingReceipt> {
    let (expected, bump) = receipt_address(&intent.funding_id);
    if address != expected.to_bytes() || owner != vault_program().to_bytes() {
        return Err(RuntimeError::Identity);
    }
    if data.len() != 8 + cinder_vault::PhoenixFundingReceipt::INIT_SPACE {
        return Err(RuntimeError::Decode);
    }
    let receipt = cinder_vault::PhoenixFundingReceipt::try_deserialize(&mut &data[..])
        .map_err(|_| RuntimeError::Decode)?;
    if receipt.schema_version != cinder_common::ACCOUNT_SCHEMA_VERSION
        || receipt.bump != bump
        || receipt.funding_id != intent.funding_id
        || receipt.amount != intent.amount
        || receipt.phoenix_trader.to_bytes() != intent.phoenix_trader
        || receipt.phoenix_program.to_bytes() != intent.phoenix_program
        || receipt.funded_at_slot == 0
    {
        return Err(RuntimeError::Identity);
    }
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::AccountSerialize;

    #[test]
    fn account_binding_decodes_native_global_and_rejects_mismatched_custody() {
        use anchor_lang::AnchorDeserialize;
        use phoenix_rise_accounts::discriminants::PhoenixAccount;
        let (accounts, _) = fixture();
        let mut value: serde_json::Value =
            serde_json::from_str(include_str!("../../../docs/operator-config.example.json"))
                .unwrap();
        value["phoenix_trader"] = accounts.trader.to_string().into();
        value["phoenix_quote_mint"] = accounts.quote_mint.to_string().into();
        let config: RuntimeConfig = serde_json::from_value(value).unwrap();
        let mut vault = cinder_vault::Config::deserialize(
            &mut &vec![0u8; cinder_vault::Config::INIT_SPACE][..],
        )
        .unwrap();
        vault.vault_authority = anchor(vault_authority());
        vault.phoenix_trader = anchor(accounts.trader);
        vault.usdc_mint = anchor(accounts.usdc_mint);
        vault.vault_usdc_ata = anchor(ata(accounts.usdc_mint));
        vault.adapter = anchor(Pubkey::new_from_array(accounts.operator));
        let mut data = vec![0u8; 1104];
        data[..8].copy_from_slice(&PhoenixAccount::GlobalConfiguration.discriminant());
        for (offset, key) in [
            (8, accounts.global.to_bytes()),
            (296, accounts.quote_mint.to_bytes()),
            (328, accounts.global_vault.to_bytes()),
            (392, bytes(&config.global_trader_index[0]).unwrap()),
            (424, bytes(&config.active_trader_buffer[0]).unwrap()),
        ] {
            data[offset..offset + 32].copy_from_slice(&key);
        }
        data[505] = 6;
        let address = accounts.global.to_bytes();
        let owner = accounts.phoenix_program.to_bytes();
        let decoded =
            PhoenixFundingAccounts::from_global(&config, &vault, address, owner, &data).unwrap();
        assert_eq!(decoded.global_vault, accounts.global_vault);
        assert!(
            PhoenixFundingAccounts::from_global(&config, &vault, address, [15; 32], &data).is_err()
        );
        vault.vault_authority = anchor(Pubkey::new_from_array([16; 32]));
        assert!(
            PhoenixFundingAccounts::from_global(&config, &vault, address, owner, &data).is_err()
        );
        vault.vault_authority = anchor(vault_authority());
        data[505] = 9;
        assert!(
            PhoenixFundingAccounts::from_global(&config, &vault, address, owner, &data).is_err()
        );
        data[505] = 6;
        data[296..328].fill(17);
        assert!(
            PhoenixFundingAccounts::from_global(&config, &vault, address, owner, &data).is_err()
        );
    }
    fn fixture() -> (PhoenixFundingAccounts, FundingIntent) {
        let accounts = PhoenixFundingAccounts {
            operator: [1; 32],
            phoenix_program: PROD_PHOENIX_PROGRAM_ID,
            global: PROD_PHOENIX_GLOBAL_CONFIGURATION,
            log: PROD_PHOENIX_LOG_AUTHORITY,
            trader: Pubkey::new_from_array([2; 32]),
            usdc_mint: Pubkey::new_from_array([3; 32]),
            quote_mint: Pubkey::new_from_array([4; 32]),
            global_vault: Pubkey::new_from_array([5; 32]),
            trader_index: vec![
                Pubkey::new_from_array([6; 32]),
                Pubkey::new_from_array([7; 32]),
            ],
            buffer: vec![
                Pubkey::new_from_array([8; 32]),
                Pubkey::new_from_array([9; 32]),
            ],
        };
        let intent = FundingIntent::new(
            [10; 32],
            50_000_000,
            accounts.trader.to_bytes(),
            accounts.phoenix_program.to_bytes(),
        );
        (accounts, intent)
    }

    #[test]
    fn funding_packet_uses_vault_pda_and_preserves_all_spill_accounts() {
        let (accounts, intent) = fixture();
        let ix = build_phoenix_funding(&intent, [1; 32], &accounts).unwrap();
        assert_eq!(ix.program_id, vault_program());
        assert_eq!(
            ix.accounts
                .iter()
                .filter(|m| m.is_signer)
                .map(|m| m.pubkey)
                .collect::<Vec<_>>(),
            vec![Pubkey::new_from_array([1; 32])]
        );
        assert_eq!(ix.accounts[2].pubkey, vault_authority());
        assert_eq!(ix.accounts[3].pubkey, receipt_address(&intent.funding_id).0);
        assert_eq!(ix.accounts[5].pubkey, ata(accounts.usdc_mint));
        assert_eq!(ix.accounts[7].pubkey, ata(accounts.quote_mint));
        assert_eq!(
            ix.accounts[18..]
                .iter()
                .map(|m| m.pubkey)
                .collect::<Vec<_>>(),
            accounts
                .trader_index
                .iter()
                .chain(&accounts.buffer)
                .copied()
                .collect::<Vec<_>>()
        );
        assert_eq!(ix.data[8..40], intent.funding_id);
        assert_eq!(
            u64::from_le_bytes(ix.data[40..48].try_into().unwrap()),
            intent.amount
        );
        assert_eq!(ix.data[48], 2);
        assert!(
            !ix.accounts
                .iter()
                .any(|m| m.pubkey.to_bytes() == intent.operation_id),
            "private journal identity stays off chain"
        );
    }

    #[test]
    fn funding_packet_rejects_wrong_operator_identity_and_aliases() {
        let (mut accounts, mut intent) = fixture();
        assert!(build_phoenix_funding(&intent, [11; 32], &accounts).is_err());
        intent.amount += 1;
        assert!(build_phoenix_funding(&intent, [1; 32], &accounts).is_err());
        intent.amount -= 1;
        accounts.buffer.push(accounts.trader_index[0]);
        assert!(build_phoenix_funding(&intent, [1; 32], &accounts).is_err());
    }

    #[test]
    fn receipt_authentication_checks_owner_pda_bump_and_immutable_facts() {
        let (_, intent) = fixture();
        let (address, bump) = receipt_address(&intent.funding_id);
        let mut receipt = cinder_vault::PhoenixFundingReceipt {
            schema_version: cinder_common::ACCOUNT_SCHEMA_VERSION,
            funding_id: intent.funding_id,
            amount: intent.amount,
            phoenix_trader: anchor(Pubkey::new_from_array(intent.phoenix_trader)),
            phoenix_program: anchor(Pubkey::new_from_array(intent.phoenix_program)),
            funded_at_slot: 42,
            bump,
        };
        let mut data = Vec::new();
        receipt.try_serialize(&mut data).unwrap();
        assert!(decode_phoenix_funding_receipt(
            address.to_bytes(),
            vault_program().to_bytes(),
            &data,
            &intent
        )
        .is_ok());
        assert!(decode_phoenix_funding_receipt(
            [12; 32],
            vault_program().to_bytes(),
            &data,
            &intent
        )
        .is_err());
        assert!(
            decode_phoenix_funding_receipt(address.to_bytes(), [12; 32], &data, &intent).is_err()
        );
        for variant in 0..4 {
            receipt.amount = intent.amount;
            receipt.bump = bump;
            receipt.funded_at_slot = 42;
            receipt.funding_id = intent.funding_id;
            match variant {
                0 => receipt.amount += 1,
                1 => receipt.bump = bump.wrapping_add(1),
                2 => receipt.funded_at_slot = 0,
                _ => receipt.funding_id = [0; 32],
            }
            data.clear();
            receipt.try_serialize(&mut data).unwrap();
            assert!(decode_phoenix_funding_receipt(
                address.to_bytes(),
                vault_program().to_bytes(),
                &data,
                &intent
            )
            .is_err());
        }
    }
}
