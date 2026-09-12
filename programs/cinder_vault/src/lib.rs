use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use cinder_common as cc;

declare_id!("9zhBFVgk13gnYT6iVuKPGfQiAvVfr6cYQq2bY2QUzXmg");

#[program]
pub mod cinder_vault {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        adapter: Pubkey,
        vault_authority: Pubkey,
        phoenix_trader: Pubkey,
        er_validator: Pubkey,
    ) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.admin = ctx.accounts.admin.key();
        config.adapter = adapter;
        config.vault_authority = vault_authority;
        config.phoenix_trader = phoenix_trader;
        config.usdc_mint = ctx.accounts.usdc_mint.key();
        config.vault_usdc_ata = ctx.accounts.vault_usdc_ata.key();
        config.er_validator = er_validator;
        config.paused = 0;
        config.user_im_mult_bps = cc::USER_IM_MULT_BPS;
        config.user_mm_mult_bps = cc::USER_MM_MULT_BPS;
        config.max_user_leverage = cc::MAX_USER_LEVERAGE;
        config.buffer_min_bps = cc::BUFFER_MIN_BPS;
        config.buffer_floor_usdc = cc::BUFFER_FLOOR_USDC;
        config.allowlist_len = 0;
        config.allowlist_assets = [0; cc::MAX_ALLOWLIST];
        config.bump_config = ctx.bumps.config;
        config.bump_vault_authority = ctx.bumps.vault_authority;

        let root = &mut ctx.accounts.reserve_root;
        root.epoch = 0;
        root.root = [0u8; 32];
        root.user_count = 0;
        root.total_free = 0;
        root.total_reserved = 0;
        root.book_hash = [0u8; 32];
        root.committed_at_base_slot = 0;
        Ok(())
    }

    pub fn set_adapter(ctx: Context<AdminConfig>, adapter: Pubkey) -> Result<()> {
        ctx.accounts.config.adapter = adapter;
        Ok(())
    }

    pub fn set_halt(ctx: Context<AdminConfig>, flags: u8) -> Result<()> {
        ctx.accounts.config.paused = flags;
        Ok(())
    }

    pub fn set_allowlist(ctx: Context<AdminConfig>, assets: Vec<u16>) -> Result<()> {
        require!(assets.len() <= cc::MAX_ALLOWLIST, VaultError::AllowlistTooLong);
        let config = &mut ctx.accounts.config;
        config.allowlist_len = assets.len() as u8;
        config.allowlist_assets = [0; cc::MAX_ALLOWLIST];
        for (i, asset) in assets.iter().enumerate() {
            config.allowlist_assets[i] = *asset;
        }
        Ok(())
    }

    pub fn escape_withdraw(_ctx: Context<EscapeWithdraw>) -> Result<()> {
        err!(VaultError::Unsupported)
    }

    /// Stand-in era: move USDC vault ATA → stand-in ATA. Adapter then Rise-deposits.
    pub fn post_collateral(ctx: Context<PostCollateral>, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::ZeroAmount);
        require_keys_eq!(
            ctx.accounts.config.adapter,
            ctx.accounts.adapter.key(),
            VaultError::Unauthorized
        );
        require_keys_eq!(
            ctx.accounts.dest_usdc_ata.owner,
            ctx.accounts.config.vault_authority,
            VaultError::BadTransitOwner
        );
        require_keys_eq!(
            ctx.accounts.dest_usdc_ata.mint,
            ctx.accounts.config.usdc_mint,
            VaultError::BadMint
        );
        let seeds: &[&[u8]] = &[
            cc::SEED_VAULT_AUTHORITY,
            &[ctx.accounts.config.bump_vault_authority],
        ];
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: ctx.accounts.vault_usdc_ata.to_account_info(),
                    to: ctx.accounts.dest_usdc_ata.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
                &[seeds],
            ),
            amount,
        )?;
        Ok(())
    }

    /// Stand-in era: after Rise withdraw, move USDC stand-in ATA → vault ATA.
    /// Stand-in (`vault_authority` keypair) must sign; adapter authorizes the ix.
    pub fn pull_collateral(ctx: Context<PullCollateral>, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::ZeroAmount);
        require_keys_eq!(
            ctx.accounts.config.adapter,
            ctx.accounts.adapter.key(),
            VaultError::Unauthorized
        );
        require_keys_eq!(
            ctx.accounts.source_usdc_ata.owner,
            ctx.accounts.config.vault_authority,
            VaultError::BadTransitOwner
        );
        require_keys_eq!(
            ctx.accounts.stand_in.key(),
            ctx.accounts.config.vault_authority,
            VaultError::Unauthorized
        );
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: ctx.accounts.source_usdc_ata.to_account_info(),
                    to: ctx.accounts.vault_usdc_ata.to_account_info(),
                    authority: ctx.accounts.stand_in.to_account_info(),
                },
            ),
            amount,
        )?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = 8 + Config::INIT_SPACE,
        seeds = [cc::SEED_CONFIG],
        bump
    )]
    pub config: Account<'info, Config>,
    /// CHECK: PDA signer for S9; no data account. ATA authority now.
    #[account(seeds = [cc::SEED_VAULT_AUTHORITY], bump)]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(
        init,
        payer = admin,
        space = 8 + ReserveRoot::INIT_SPACE,
        seeds = [cc::SEED_RESERVE],
        bump
    )]
    pub reserve_root: Account<'info, ReserveRoot>,
    pub usdc_mint: Account<'info, Mint>,
    #[account(
        init,
        payer = admin,
        associated_token::mint = usdc_mint,
        associated_token::authority = vault_authority
    )]
    pub vault_usdc_ata: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AdminConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [cc::SEED_CONFIG],
        bump = config.bump_config,
        has_one = admin @ VaultError::Unauthorized
    )]
    pub config: Account<'info, Config>,
}

#[derive(Accounts)]
pub struct EscapeWithdraw<'info> {
    pub user: Signer<'info>,
}

#[derive(Accounts)]
pub struct PostCollateral<'info> {
    pub adapter: Signer<'info>,
    #[account(
        seeds = [cc::SEED_CONFIG],
        bump = config.bump_config
    )]
    pub config: Account<'info, Config>,
    /// CHECK: PDA signer for the vault ATA.
    #[account(
        seeds = [cc::SEED_VAULT_AUTHORITY],
        bump = config.bump_vault_authority
    )]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(
        mut,
        address = config.vault_usdc_ata
    )]
    pub vault_usdc_ata: Account<'info, TokenAccount>,
    #[account(mut)]
    pub dest_usdc_ata: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct PullCollateral<'info> {
    pub adapter: Signer<'info>,
    /// Stand-in Phoenix authority (keypair era). Token owner of `source_usdc_ata`.
    pub stand_in: Signer<'info>,
    #[account(
        seeds = [cc::SEED_CONFIG],
        bump = config.bump_config
    )]
    pub config: Account<'info, Config>,
    #[account(
        mut,
        address = config.vault_usdc_ata
    )]
    pub vault_usdc_ata: Account<'info, TokenAccount>,
    #[account(mut)]
    pub source_usdc_ata: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub admin: Pubkey,
    pub adapter: Pubkey,
    pub vault_authority: Pubkey,
    pub phoenix_trader: Pubkey,
    pub usdc_mint: Pubkey,
    pub vault_usdc_ata: Pubkey,
    pub er_validator: Pubkey,
    pub paused: u8,
    pub user_im_mult_bps: u16,
    pub user_mm_mult_bps: u16,
    pub max_user_leverage: u16,
    pub buffer_min_bps: u16,
    pub buffer_floor_usdc: u64,
    pub allowlist_len: u8,
    pub allowlist_assets: [u16; 32],
    pub bump_config: u8,
    pub bump_vault_authority: u8,
}

#[account]
#[derive(InitSpace)]
pub struct ReserveRoot {
    pub epoch: u64,
    pub root: [u8; 32],
    pub user_count: u32,
    pub total_free: u64,
    pub total_reserved: u64,
    pub book_hash: [u8; 32],
    pub committed_at_base_slot: u64,
}

#[error_code]
pub enum VaultError {
    #[msg("unsupported")]
    Unsupported,
    #[msg("unauthorized")]
    Unauthorized,
    #[msg("allowlist too long")]
    AllowlistTooLong,
    #[msg("zero amount")]
    ZeroAmount,
    #[msg("bad mint")]
    BadMint,
    #[msg("transit ATA owner is not vault_authority")]
    BadTransitOwner,
}
