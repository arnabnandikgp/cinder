//! Atomic fences around an independent Phoenix IOC. Native fee changes or
//! adverse collateral changes cannot turn a successful simulation into an
//! unbounded charge: a failing trailing fence rolls the entire transaction back.
use super::*;
use anchor_lang::solana_program::program::{get_return_data, invoke};
use anchor_lang::InstructionData;
use phoenix_rise_accounts::{global_config::GlobalConfig, orderbook::Orderbook};
use phoenix_rise_ix::{
    constants::*,
    hawkeye::{self, HawkeyeReturnData, HawkeyeTraderViewAccounts},
};
use solana_instructions_sysvar::{load_current_index_checked, load_instruction_at_checked};
use solana_sha256_hasher::{hash, hashv};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PhoenixExecutionGuard {
    pub snapshot_hash: [u8; 32],
    pub ioc_data_hash: [u8; 32],
    pub observed_slot: u64,
    pub oldest_observed_ms: u64,
    pub taker_fee_counter: u64,
    pub max_fee_usdc: u64,
    pub gti_count: u8,
}

#[derive(Accounts)]
pub struct GuardPhoenixExecution<'info> {
    pub adapter: Signer<'info>,
    #[account(seeds = [cc::SEED_CONFIG], bump = config.bump_config,
        has_one = adapter @ VaultError::Unauthorized,
        has_one = phoenix_trader @ VaultError::BadPhoenixAccount)]
    pub config: Box<Account<'info, Config>>,
    /// CHECK: pinned deployment and executable below.
    pub phoenix_program: UncheckedAccount<'info>,
    /// CHECK: native ownership/layout and deployment below.
    pub phoenix_global_config: UncheckedAccount<'info>,
    /// CHECK: native ownership and GlobalConfig binding below.
    pub phoenix_asset_map: UncheckedAccount<'info>,
    /// CHECK: Config binding, native ownership, snapshot and Hawkeye below.
    pub phoenix_trader: UncheckedAccount<'info>,
    /// CHECK: native orderbook header and IOC account binding below.
    pub orderbook: UncheckedAccount<'info>,
    /// CHECK: official Hawkeye executable below.
    pub hawkeye_program: UncheckedAccount<'info>,
    /// CHECK: pinned instructions sysvar; used to bind the adjacent native IOC.
    #[account(address = Pubkey::new_from_array(solana_instructions_sysvar::ID.to_bytes()))]
    pub instructions: UncheckedAccount<'info>,
}

fn failed() -> anchor_lang::error::Error {
    error!(VaultError::ExecutionGuardFailed)
}

/// Streaming hash: no copy of the large native asset map or active buffers.
fn snapshot(accounts: &[AccountInfo]) -> Result<[u8; 32]> {
    let mut digest = hash(b"cinder:phoenix-snapshot:v1").to_bytes();
    for account in accounts {
        require!(!account.executable, VaultError::BadPhoenixAccount);
        let data = account.try_borrow_data()?;
        digest = hashv(&[
            &digest,
            account.key.as_ref(),
            account.owner.as_ref(),
            hash(&data).as_ref(),
        ])
        .to_bytes();
    }
    Ok(digest)
}

#[inline(never)]
fn validate_native(
    a: &GuardPhoenixExecution,
    remaining: &[AccountInfo],
    split: usize,
) -> Result<()> {
    let pinned = [
        PROD_PHOENIX_INSTRUCTION_ADDRESSES,
        BETA_PHOENIX_INSTRUCTION_ADDRESSES,
    ]
    .iter()
    .any(|p| {
        p.program_id.to_bytes() == a.phoenix_program.key().to_bytes()
            && p.global_configuration.to_bytes() == a.phoenix_global_config.key().to_bytes()
    });
    require!(
        pinned && a.phoenix_program.executable && a.hawkeye_program.executable,
        VaultError::BadPhoenixAccount
    );
    require!(
        a.hawkeye_program.key().to_bytes() == hawkeye::HAWKEYE_PROGRAM_ID.to_bytes(),
        VaultError::BadPhoenixAccount
    );
    require!(
        remaining.len() >= 5 && remaining.len() <= 67 && split > 0 && split < remaining.len() - 3,
        VaultError::BadPhoenixAccount
    );
    for account in [
        &a.phoenix_global_config,
        &a.phoenix_asset_map,
        &a.phoenix_trader,
        &a.orderbook,
    ] {
        require_keys_eq!(
            *account.owner,
            a.phoenix_program.key(),
            VaultError::BadPhoenixAccount
        );
    }
    for (i, account) in remaining.iter().enumerate() {
        require!(!account.is_signer, VaultError::BadPhoenixAccount);
        if i >= 3 {
            require_keys_eq!(
                *account.owner,
                a.phoenix_program.key(),
                VaultError::BadPhoenixAccount
            );
        }
        require!(
            !remaining[..i].iter().any(|p| p.key == account.key),
            VaultError::BadPhoenixAccount
        );
    }
    require_keys_eq!(
        *remaining[0].key,
        a.config.vault_usdc_ata,
        VaultError::BadPhoenixAccount
    );
    require_keys_eq!(
        *remaining[1].key,
        a.config.usdc_mint,
        VaultError::BadPhoenixAccount
    );
    let data = a.phoenix_global_config.try_borrow_data()?;
    let global = GlobalConfig::try_from_account_bytes(&data).map_err(|_| failed())?;
    require!(
        global.account_key() == a.phoenix_global_config.key().to_bytes()
            && global.perp_asset_map_key() == a.phoenix_asset_map.key().to_bytes()
            && global.quote_decimals() == 6
            && global.canonical_token_mint_key() == remaining[2].key.to_bytes()
            && global.global_trader_index_header_key() == remaining[3].key.to_bytes()
            && global.active_trader_buffer_header_key() == remaining[3 + split].key.to_bytes(),
        VaultError::BadPhoenixAccount
    );
    Ok(())
}

pub fn guard_execution<'info>(
    ctx: Context<'info, GuardPhoenixExecution<'info>>,
    guard: PhoenixExecutionGuard,
    after: bool,
) -> Result<()> {
    let a = &ctx.accounts;
    let split = usize::from(guard.gti_count);
    validate_native(a, ctx.remaining_accounts, split)?;
    let clock = Clock::get()?;
    let now_ms = u64::try_from(clock.unix_timestamp)
        .map_err(|_| failed())?
        .checked_mul(1000)
        .ok_or_else(failed)?;
    require!(
        clock.slot >= guard.observed_slot
            && now_ms
                .checked_sub(guard.oldest_observed_ms)
                .is_some_and(|age| age <= cc::MARK_STALE_MS)
            && !cc::entries_blocked(a.config.paused & !cc::OPERATOR_DOWN),
        VaultError::ExecutionGuardFailed
    );
    let index = usize::from(load_current_index_checked(&a.instructions)?);
    let adjacent = if after {
        index.checked_sub(1)
    } else {
        index.checked_add(1)
    }
    .ok_or_else(failed)?;
    let ioc = load_instruction_at_checked(adjacent, &a.instructions)?;
    let peer_index = if after {
        index.checked_sub(2)
    } else {
        index.checked_add(2)
    }
    .ok_or_else(failed)?;
    let peer = load_instruction_at_checked(peer_index, &a.instructions)?;
    let expected_peer = crate::instruction::GuardPhoenixExecution {
        guard: guard.clone(),
        after: !after,
    }
    .data();
    require!(
        peer.program_id == crate::ID
            && peer.data == expected_peer
            && peer.accounts.len() == 9 + ctx.remaining_accounts.len()
            && peer.accounts.iter().map(|m| m.pubkey).eq([
                a.adapter.key(),
                a.config.key(),
                a.phoenix_program.key(),
                a.phoenix_global_config.key(),
                a.phoenix_asset_map.key(),
                a.phoenix_trader.key(),
                a.orderbook.key(),
                a.hawkeye_program.key(),
                a.instructions.key()
            ]
            .into_iter()
            .chain(ctx.remaining_accounts.iter().map(|r| *r.key)))
            && ioc.program_id == a.phoenix_program.key()
            && hash(&ioc.data).to_bytes() == guard.ioc_data_hash
            && ioc
                .accounts
                .iter()
                .any(|m| m.pubkey == a.orderbook.key() && m.is_writable)
            && ioc
                .accounts
                .iter()
                .any(|m| m.pubkey == a.phoenix_trader.key() && m.is_writable)
            && ioc
                .accounts
                .iter()
                .any(|m| m.pubkey == a.adapter.key() && m.is_signer),
        VaultError::ExecutionGuardFailed
    );
    let counter = {
        let data = a.orderbook.try_borrow_data()?;
        Orderbook::try_from_account_bytes(&data)
            .map_err(|_| failed())?
            .header()
            .total_taker_quote_lot_fees
            .as_inner()
    };
    if !after {
        require!(
            counter == guard.taker_fee_counter,
            VaultError::ExecutionGuardFailed
        );
        let mut accounts = vec![
            a.config.to_account_info(),
            a.phoenix_global_config.to_account_info(),
            a.phoenix_asset_map.to_account_info(),
            a.phoenix_trader.to_account_info(),
        ];
        accounts.extend_from_slice(ctx.remaining_accounts);
        require!(
            snapshot(&accounts)? == guard.snapshot_hash,
            VaultError::ExecutionGuardFailed
        );
    } else {
        require!(
            counter
                .checked_sub(guard.taker_fee_counter)
                .is_some_and(|fee| fee <= guard.max_fee_usdc),
            VaultError::ExecutionGuardFailed
        );
        check_post_collateral(a, ctx.remaining_accounts, split)?;
    }
    Ok(())
}

#[inline(never)]
fn check_post_collateral<'info>(
    a: &GuardPhoenixExecution<'info>,
    remaining: &[AccountInfo<'info>],
    split: usize,
) -> Result<()> {
    let key = |p: Pubkey| solana_pubkey::Pubkey::new_from_array(p.to_bytes());
    let accounts = HawkeyeTraderViewAccounts {
        phoenix_program_id: key(a.phoenix_program.key()),
        global_config: key(a.phoenix_global_config.key()),
        perp_asset_map: key(a.phoenix_asset_map.key()),
        trader: key(a.phoenix_trader.key()),
        global_trader_index: remaining[3..3 + split]
            .iter()
            .map(|i| key(*i.key))
            .collect(),
        active_trader_buffer: remaining[3 + split..].iter().map(|i| key(*i.key)).collect(),
    };
    let ix = crate::phoenix::instruction(hawkeye::create_hawkeye_view_margin_ix(accounts));
    let mut infos = vec![
        a.phoenix_program.to_account_info(),
        a.phoenix_global_config.to_account_info(),
        a.phoenix_asset_map.to_account_info(),
        a.phoenix_trader.to_account_info(),
        a.hawkeye_program.to_account_info(),
    ];
    infos.extend_from_slice(&remaining[3..]);
    invoke(&ix, &infos)?;
    let (program, bytes) = get_return_data().ok_or_else(failed)?;
    require!(
        program == a.hawkeye_program.key(),
        VaultError::ExecutionGuardFailed
    );
    let margin = match hawkeye::decode_hawkeye_return_data(&bytes).map_err(|_| failed())? {
        HawkeyeReturnData::Margin(m) if m.version == hawkeye::HAWKEYE_RETURN_VERSION => m,
        _ => return Err(failed()),
    };
    let required = u128::from(margin.initial_margin_quote_lots)
        .checked_mul(u128::from(cc::BPS_DENOM) + u128::from(cc::BUFFER_MIN_BPS))
        .ok_or_else(failed)?
        .div_ceil(u128::from(cc::BPS_DENOM))
        .max(u128::from(cc::BUFFER_FLOOR_USDC));
    require!(
        margin.risk_tier == 0
            && margin.is_liquidatable == 0
            && margin.effective_collateral_quote_lots
                >= i64::try_from(margin.initial_margin_quote_lots).map_err(|_| failed())?
            && margin.collateral_quote_lots >= i64::try_from(required).map_err(|_| failed())?,
        VaultError::ExecutionGuardFailed
    );
    Ok(())
}
