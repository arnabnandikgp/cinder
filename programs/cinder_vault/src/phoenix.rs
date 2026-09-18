//! Pinned, PDA-authorized funding only. No caller-supplied CPI data or program.
use super::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
};
use phoenix_rise_accounts::{global_config::GlobalConfig, trader::TraderHeader};
use phoenix_rise_ix::{constants::*, types::Instruction as RiseInstruction};

pub const SEED_PHOENIX_FUNDING: &[u8] = b"phoenix-funding";

fn anchor_key(key: [u8; 32]) -> Pubkey {
    Pubkey::new_from_array(key)
}
fn rise_key(key: Pubkey) -> solana_pubkey::Pubkey {
    solana_pubkey::Pubkey::new_from_array(key.to_bytes())
}

fn deployment(program: Pubkey, global: Pubkey, log: Pubkey) -> Result<()> {
    let matches = [
        PROD_PHOENIX_INSTRUCTION_ADDRESSES,
        BETA_PHOENIX_INSTRUCTION_ADDRESSES,
    ]
    .iter()
    .any(|a| {
        program.to_bytes() == a.program_id.to_bytes()
            && global.to_bytes() == a.global_configuration.to_bytes()
            && log.to_bytes() == a.log_authority.to_bytes()
    });
    require!(matches, VaultError::BadPhoenixAccount);
    Ok(())
}

fn trader_identity(info: &AccountInfo, program: Pubkey, authority: Pubkey) -> Result<()> {
    require_keys_eq!(*info.owner, program, VaultError::BadPhoenixAccount);
    let data = info.try_borrow_data()?;
    let header = TraderHeader::try_from_account_bytes(&data)
        .map_err(|_| error!(VaultError::BadPhoenixState))?;
    require!(
        header.key == info.key.to_bytes()
            && header.authority == authority.to_bytes()
            && header.trader_subaccount_index == 0,
        VaultError::BadPhoenixAccount
    );
    Ok(())
}

pub(crate) fn instruction(ix: RiseInstruction) -> Instruction {
    Instruction {
        program_id: anchor_key(ix.program_id.to_bytes()),
        data: ix.data,
        accounts: ix
            .accounts
            .into_iter()
            .map(|a| AccountMeta {
                pubkey: anchor_key(a.pubkey.to_bytes()),
                is_signer: a.is_signer,
                is_writable: a.is_writable,
            })
            .collect(),
    }
}

fn invoke<'info>(ix: Instruction, accounts: &[AccountInfo<'info>], bump: u8) -> Result<()> {
    invoke_signed(&ix, accounts, &[&[cc::SEED_VAULT_AUTHORITY, &[bump]]]).map_err(Into::into)
}

pub fn delegate(ctx: Context<DelegatePhoenixTrader>) -> Result<()> {
    let a = &ctx.accounts;
    deployment(
        a.phoenix_program.key(),
        a.phoenix_global_config.key(),
        a.phoenix_log_authority.key(),
    )?;
    require_keys_eq!(
        a.config.vault_authority,
        a.vault_authority.key(),
        VaultError::StandInNotRetired
    );
    trader_identity(
        &a.phoenix_trader,
        a.phoenix_program.key(),
        a.vault_authority.key(),
    )?;
    let params = phoenix_rise_ix::delegate_trader::DelegateTraderParams::builder()
        .trader_wallet(rise_key(a.vault_authority.key()))
        .trader_account(rise_key(a.phoenix_trader.key()))
        .new_position_authority(rise_key(a.adapter.key()))
        .build()
        .map_err(|_| error!(VaultError::BadPhoenixState))?;
    let mut ix = instruction(
        phoenix_rise_ix::delegate_trader::create_delegate_trader_ix(params)
            .map_err(|_| error!(VaultError::BadPhoenixState))?,
    );
    ix.program_id = a.phoenix_program.key();
    for (index, key) in [
        a.phoenix_program.key(),
        a.phoenix_log_authority.key(),
        a.phoenix_global_config.key(),
    ]
    .into_iter()
    .enumerate()
    {
        ix.accounts[index].pubkey = key;
    }
    invoke(
        ix,
        &[
            a.phoenix_program.to_account_info(),
            a.phoenix_log_authority.to_account_info(),
            a.phoenix_global_config.to_account_info(),
            a.vault_authority.to_account_info(),
            a.phoenix_trader.to_account_info(),
            a.adapter.to_account_info(),
        ],
        a.config.bump_vault_authority,
    )
}

// Keep the 1,104-byte SDK configuration copy outside the CPI frame.
#[inline(never)]
fn validate_global(a: &FundPhoenix, remaining: &[AccountInfo], split: usize) -> Result<()> {
    let data = a.phoenix_global_config.try_borrow_data()?;
    let global = GlobalConfig::try_from_account_bytes(&data)
        .map_err(|_| error!(VaultError::BadPhoenixState))?;
    require!(
        global.account_key() == a.phoenix_global_config.key().to_bytes()
            && global.quote_decimals() == 6
            && global.canonical_token_mint_key() == a.phoenix_quote_mint.key().to_bytes()
            && global.global_vault_key() == a.phoenix_global_vault.key().to_bytes(),
        VaultError::BadPhoenixAccount
    );
    require!(
        split > 0 && split < remaining.len() && remaining.len() <= 64,
        VaultError::BadPhoenixAccount
    );
    require!(
        remaining[0].key.to_bytes() == global.global_trader_index_header_key()
            && remaining[split].key.to_bytes() == global.active_trader_buffer_header_key(),
        VaultError::BadPhoenixAccount
    );
    Ok(())
}

pub fn fund<'info>(
    ctx: Context<'info, FundPhoenix<'info>>,
    funding_id: [u8; 32],
    amount: u64,
    gti_count: u8,
) -> Result<()> {
    let a = &ctx.accounts;
    require!(
        amount > 0 && funding_id != [0; 32],
        VaultError::BadFundingIntent
    );
    require_keys_eq!(
        a.config.vault_authority,
        a.vault_authority.key(),
        VaultError::StandInNotRetired
    );
    deployment(
        a.phoenix_program.key(),
        a.phoenix_global_config.key(),
        a.phoenix_log_authority.key(),
    )?;
    trader_identity(
        &a.phoenix_trader,
        a.phoenix_program.key(),
        a.vault_authority.key(),
    )?;
    let split = usize::from(gti_count);
    let remaining = ctx.remaining_accounts;
    validate_global(a, remaining, split)?;
    require_keys_neq!(
        a.usdc_mint.key(),
        a.phoenix_quote_mint.key(),
        VaultError::BadMint
    );
    for account in [&a.vault_usdc_ata, &a.vault_phoenix_ata] {
        require!(
            account.delegate.is_none() && account.close_authority.is_none(),
            VaultError::BadPhoenixAccount
        );
    }
    let mut seen = std::collections::BTreeSet::new();
    for info in remaining {
        require!(
            info.is_writable
                && !info.is_signer
                && *info.owner == a.phoenix_program.key()
                && seen.insert(*info.key),
            VaultError::BadPhoenixAccount
        );
        require!(
            ![
                a.phoenix_trader.key(),
                a.phoenix_global_config.key(),
                a.phoenix_global_vault.key(),
                a.vault_phoenix_ata.key(),
                a.vault_usdc_ata.key()
            ]
            .contains(info.key),
            VaultError::BadPhoenixAccount
        );
    }
    let program = a.phoenix_program.key();
    let ember_state =
        Pubkey::find_program_address(&[program.as_ref(), b"state"], &a.ember_program.key()).0;
    let ember_vault =
        Pubkey::find_program_address(&[program.as_ref(), b"vault"], &a.ember_program.key()).0;
    require_keys_eq!(
        a.ember_state.key(),
        ember_state,
        VaultError::BadPhoenixAccount
    );
    require_keys_eq!(
        a.ember_vault.key(),
        ember_vault,
        VaultError::BadPhoenixAccount
    );
    let source_before = a.vault_usdc_ata.amount;
    let transit_before = a.vault_phoenix_ata.amount;
    let global_before = a.phoenix_global_vault.amount;
    let params = phoenix_rise_ix::ember_deposit::EmberDepositParams::builder()
        .trader(rise_key(a.vault_authority.key()))
        .usdc_mint(rise_key(a.usdc_mint.key()))
        .canonical_mint(rise_key(a.phoenix_quote_mint.key()))
        .trader_usdc_account(rise_key(a.vault_usdc_ata.key()))
        .trader_phoenix_account(rise_key(a.vault_phoenix_ata.key()))
        .amount(amount)
        .build()
        .map_err(|_| error!(VaultError::BadFundingIntent))?;
    let mut ember = instruction(
        phoenix_rise_ix::ember_deposit::create_ember_deposit_ix(params)
            .map_err(|_| error!(VaultError::BadPhoenixState))?,
    );
    ember.accounts[1].pubkey = ember_state;
    ember.accounts[6].pubkey = ember_vault;
    let infos = [
        a.vault_authority.to_account_info(),
        a.ember_state.to_account_info(),
        a.usdc_mint.to_account_info(),
        a.phoenix_quote_mint.to_account_info(),
        a.vault_usdc_ata.to_account_info(),
        a.vault_phoenix_ata.to_account_info(),
        a.ember_vault.to_account_info(),
        a.token_program.to_account_info(),
        a.ember_program.to_account_info(),
    ];
    invoke(ember, &infos, a.config.bump_vault_authority)?;
    let params = phoenix_rise_ix::deposit_funds::DepositFundsParams::builder()
        .trader(rise_key(a.vault_authority.key()))
        .trader_account(rise_key(a.phoenix_trader.key()))
        .canonical_mint(rise_key(a.phoenix_quote_mint.key()))
        .global_vault(rise_key(a.phoenix_global_vault.key()))
        .trader_token_account(rise_key(a.vault_phoenix_ata.key()))
        .global_trader_index(
            remaining[..split]
                .iter()
                .map(|i| rise_key(*i.key))
                .collect(),
        )
        .active_trader_buffer(
            remaining[split..]
                .iter()
                .map(|i| rise_key(*i.key))
                .collect(),
        )
        .amount(amount)
        .build()
        .map_err(|_| error!(VaultError::BadFundingIntent))?;
    let mut native = instruction(
        phoenix_rise_ix::deposit_funds::create_deposit_funds_ix(params)
            .map_err(|_| error!(VaultError::BadPhoenixState))?,
    );
    native.program_id = program;
    for (index, key) in [
        program,
        a.phoenix_log_authority.key(),
        a.phoenix_global_config.key(),
    ]
    .into_iter()
    .enumerate()
    {
        native.accounts[index].pubkey = key;
    }
    let mut infos = vec![
        a.phoenix_program.to_account_info(),
        a.phoenix_log_authority.to_account_info(),
        a.phoenix_global_config.to_account_info(),
        a.vault_authority.to_account_info(),
        a.vault_phoenix_ata.to_account_info(),
        a.phoenix_trader.to_account_info(),
        a.phoenix_global_vault.to_account_info(),
        a.token_program.to_account_info(),
    ];
    infos.extend_from_slice(remaining);
    invoke(native, &infos, a.config.bump_vault_authority)?;
    ctx.accounts.vault_usdc_ata.reload()?;
    ctx.accounts.vault_phoenix_ata.reload()?;
    ctx.accounts.phoenix_global_vault.reload()?;
    require!(
        source_before.checked_sub(amount) == Some(ctx.accounts.vault_usdc_ata.amount)
            && transit_before == ctx.accounts.vault_phoenix_ata.amount
            && global_before.checked_add(amount) == Some(ctx.accounts.phoenix_global_vault.amount),
        VaultError::BadPhoenixState
    );
    let receipt = &mut ctx.accounts.funding_receipt;
    receipt.schema_version = cc::ACCOUNT_SCHEMA_VERSION;
    receipt.funding_id = funding_id;
    receipt.amount = amount;
    receipt.phoenix_trader = ctx.accounts.phoenix_trader.key();
    receipt.phoenix_program = program;
    receipt.funded_at_slot = Clock::get()?.slot;
    receipt.bump = ctx.bumps.funding_receipt;
    Ok(())
}

#[derive(Accounts)]
pub struct DelegatePhoenixTrader<'info> {
    pub adapter: Signer<'info>,
    #[account(seeds=[cc::SEED_CONFIG],bump=config.bump_config,has_one=adapter @ VaultError::Unauthorized)]
    pub config: Account<'info, Config>,
    /// CHECK: canonical vault PDA, used only to sign pinned Phoenix CPI.
    #[account(seeds=[cc::SEED_VAULT_AUTHORITY],bump=config.bump_vault_authority)]
    pub vault_authority: UncheckedAccount<'info>,
    /// CHECK: executable deployment checked together with global/log pins.
    #[account(executable)]
    pub phoenix_program: UncheckedAccount<'info>,
    /// CHECK: canonical log authority checked against the deployment.
    pub phoenix_log_authority: UncheckedAccount<'info>,
    /// CHECK: fixed deployment configuration owned by Phoenix.
    #[account(owner=phoenix_program.key())]
    pub phoenix_global_config: UncheckedAccount<'info>,
    /// CHECK: fixed pooled cross trader; header authority/identity checked.
    #[account(mut,address=config.phoenix_trader,owner=phoenix_program.key())]
    pub phoenix_trader: UncheckedAccount<'info>,
}

#[derive(Accounts)]
#[instruction(funding_id:[u8;32])]
pub struct FundPhoenix<'info> {
    #[account(mut)]
    pub adapter: Signer<'info>,
    #[account(seeds=[cc::SEED_CONFIG],bump=config.bump_config,has_one=adapter @ VaultError::Unauthorized)]
    pub config: Box<Account<'info, Config>>,
    /// CHECK: canonical vault PDA signer, never an operator-held keypair.
    #[account(seeds=[cc::SEED_VAULT_AUTHORITY],bump=config.bump_vault_authority)]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(init,payer=adapter,space=8+PhoenixFundingReceipt::INIT_SPACE,seeds=[SEED_PHOENIX_FUNDING,&funding_id],bump)]
    pub funding_receipt: Box<Account<'info, PhoenixFundingReceipt>>,
    #[account(address=config.usdc_mint,constraint=usdc_mint.decimals==6 @ VaultError::BadMint)]
    pub usdc_mint: Box<Account<'info, Mint>>,
    #[account(mut,address=config.vault_usdc_ata,token::mint=usdc_mint,token::authority=vault_authority)]
    pub vault_usdc_ata: Box<Account<'info, TokenAccount>>,
    #[account(mut,constraint=phoenix_quote_mint.decimals==6 @ VaultError::BadMint)]
    pub phoenix_quote_mint: Box<Account<'info, Mint>>,
    #[account(mut,associated_token::mint=phoenix_quote_mint,associated_token::authority=vault_authority)]
    pub vault_phoenix_ata: Box<Account<'info, TokenAccount>>,
    /// CHECK: pinned executable Ember deployment.
    #[account(executable,address=anchor_key(EMBER_PROGRAM_ID.to_bytes()))]
    pub ember_program: UncheckedAccount<'info>,
    /// CHECK: deployment-specific Ember state PDA verified before CPI.
    #[account(owner=ember_program.key())]
    pub ember_state: UncheckedAccount<'info>,
    #[account(mut,token::mint=usdc_mint)]
    pub ember_vault: Box<Account<'info, TokenAccount>>,
    /// CHECK: executable native deployment checked with global/log pins.
    #[account(executable)]
    pub phoenix_program: UncheckedAccount<'info>,
    /// CHECK: native deployment log PDA checked before CPI.
    pub phoenix_log_authority: UncheckedAccount<'info>,
    /// CHECK: fixed native deployment config; decoder and key bindings checked.
    #[account(mut,owner=phoenix_program.key())]
    pub phoenix_global_config: UncheckedAccount<'info>,
    /// CHECK: pooled cross trader owned by the vault PDA; decoded before CPI.
    #[account(mut,address=config.phoenix_trader,owner=phoenix_program.key())]
    pub phoenix_trader: UncheckedAccount<'info>,
    #[account(mut,token::mint=phoenix_quote_mint)]
    pub phoenix_global_vault: Box<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[account]
#[derive(InitSpace)]
pub struct PhoenixFundingReceipt {
    pub schema_version: u8,
    pub funding_id: [u8; 32],
    pub amount: u64,
    pub phoenix_trader: Pubkey,
    pub phoenix_program: Pubkey,
    pub funded_at_slot: u64,
    pub bump: u8,
}

#[cfg(test)]
mod tests {
    use super::*;
    use phoenix_rise_accounts::discriminants::PhoenixAccount;

    #[test]
    fn deployment_requires_one_complete_pinned_tuple() {
        for addresses in [
            PROD_PHOENIX_INSTRUCTION_ADDRESSES,
            BETA_PHOENIX_INSTRUCTION_ADDRESSES,
        ] {
            assert!(deployment(
                anchor_key(addresses.program_id.to_bytes()),
                anchor_key(addresses.global_configuration.to_bytes()),
                anchor_key(addresses.log_authority.to_bytes())
            )
            .is_ok());
            assert!(deployment(
                Pubkey::new_unique(),
                anchor_key(addresses.global_configuration.to_bytes()),
                anchor_key(addresses.log_authority.to_bytes())
            )
            .is_err());
        }
        assert!(deployment(
            anchor_key(PROD_PHOENIX_PROGRAM_ID.to_bytes()),
            anchor_key(BETA_PHOENIX_GLOBAL_CONFIGURATION.to_bytes()),
            anchor_key(PROD_PHOENIX_LOG_AUTHORITY.to_bytes())
        )
        .is_err());
    }

    #[test]
    fn pooled_trader_requires_native_owner_vault_authority_and_cross_subaccount() {
        let program = anchor_key(PROD_PHOENIX_PROGRAM_ID.to_bytes());
        let authority = Pubkey::new_unique();
        let key = Pubkey::new_unique();
        let mut data = vec![0u8; 224];
        data[..8].copy_from_slice(&PhoenixAccount::Trader.discriminant());
        let key_offset = core::mem::offset_of!(TraderHeader, key);
        let authority_offset = core::mem::offset_of!(TraderHeader, authority);
        data[key_offset..key_offset + 32].copy_from_slice(key.as_ref());
        data[authority_offset..authority_offset + 32].copy_from_slice(authority.as_ref());
        let mut lamports = 0;
        {
            let info =
                AccountInfo::new(&key, false, true, &mut lamports, &mut data, &program, false);
            assert!(trader_identity(&info, program, authority).is_ok());
            assert!(trader_identity(&info, Pubkey::new_unique(), authority).is_err());
            assert!(trader_identity(&info, program, Pubkey::new_unique()).is_err());
        }
        data[core::mem::offset_of!(TraderHeader, trader_subaccount_index)] = 1;
        let info = AccountInfo::new(&key, false, true, &mut lamports, &mut data, &program, false);
        assert!(trader_identity(&info, program, authority).is_err());
    }
}
