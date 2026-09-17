use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use cinder_common as cc;
use ephemeral_rollups_sdk::pda::ephemeral_balance_pda_from_payer;

mod migration;
pub use migration::*;

/// Magic Action default escrow index (`ActionArgs::new`).
const ACTION_ESCROW_INDEX: u8 = 255;

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
        root.schema_version = cc::ACCOUNT_SCHEMA_VERSION;
        root.epoch = 0;
        root.root = [0u8; 32];
        root.user_count = 0;
        root.total_free = 0;
        root.total_reserved = 0;
        root.total_bad_debt = 0;
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

    /// Operator ownership only; all administrative/economic halt bits survive.
    pub fn set_operator_down(ctx: Context<AdapterConfig>, down: bool) -> Result<()> {
        if down {
            ctx.accounts.config.paused |= cc::OPERATOR_DOWN;
        } else {
            ctx.accounts.config.paused &= !cc::OPERATOR_DOWN;
        }
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

    /// One-shot conversion of the legacy ReserveRoot to the current layout.
    pub fn migrate_reserve_root(ctx: Context<MigrateReserveRoot>) -> Result<()> {
        migration::migrate_reserve_root_handler(ctx)
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

    /// Stand-in era: adapter pays the user ATA after ER `request_withdraw`.
    pub fn user_withdraw_l1(ctx: Context<UserWithdrawL1>, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::ZeroAmount);
        require_keys_eq!(
            ctx.accounts.config.adapter,
            ctx.accounts.adapter.key(),
            VaultError::Unauthorized
        );
        require_keys_eq!(
            ctx.accounts.user_usdc_ata.mint,
            ctx.accounts.config.usdc_mint,
            VaultError::BadMint
        );
        require_keys_eq!(
            ctx.accounts.user_usdc_ata.owner,
            ctx.accounts.user.key(),
            VaultError::BadDestOwner
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
                    to: ctx.accounts.user_usdc_ata.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
                &[seeds],
            ),
            amount,
        )?;
        Ok(())
    }

    pub fn write_reserve_root(
        ctx: Context<WriteReserveRoot>,
        root: [u8; 32],
        user_count: u32,
        total_free: u64,
        total_reserved: u64,
        book_hash: [u8; 32],
    ) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.config.adapter,
            ctx.accounts.adapter.key(),
            VaultError::Unauthorized
        );
        let rr = &mut ctx.accounts.reserve_root;
        require!(
            rr.schema_version == cc::ACCOUNT_SCHEMA_VERSION,
            VaultError::UnsupportedAccountSchema
        );
        rr.epoch = rr.epoch.checked_add(1).ok_or(VaultError::Overflow)?;
        rr.root = root;
        rr.user_count = user_count;
        rr.total_free = total_free;
        rr.total_reserved = total_reserved;
        rr.book_hash = book_hash;
        rr.committed_at_base_slot = Clock::get()?.slot;
        Ok(())
    }

    /// Config.vault_authority is the vault-authority PDA.
    pub fn retire_stand_in(ctx: Context<RetireStandIn>) -> Result<()> {
        ctx.accounts.config.vault_authority = ctx.accounts.vault_authority.key();
        Ok(())
    }

    /// PDA-signed pull from an account owned by the vault-authority PDA.
    pub fn pull_collateral_pda(ctx: Context<PullCollateralPda>, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::ZeroAmount);
        require_keys_eq!(
            ctx.accounts.config.adapter,
            ctx.accounts.adapter.key(),
            VaultError::Unauthorized
        );
        require_keys_eq!(
            ctx.accounts.config.vault_authority,
            ctx.accounts.vault_authority.key(),
            VaultError::StandInNotRetired
        );
        require_keys_eq!(
            ctx.accounts.source_usdc_ata.owner,
            ctx.accounts.vault_authority.key(),
            VaultError::BadTransitOwner
        );
        require_keys_eq!(
            ctx.accounts.source_usdc_ata.mint,
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
                    from: ctx.accounts.source_usdc_ata.to_account_info(),
                    to: ctx.accounts.vault_usdc_ata.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
                &[seeds],
            ),
            amount,
        )?;
        Ok(())
    }

    /// Magic Action settle. `escrow_auth` bound to vault-authority PDA;
    /// `escrow` is `ephemeral_balance_pda_from_payer(escrow_auth, 255)`.
    pub fn settle_user_withdraw(ctx: Context<SettleUserWithdraw>, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::ZeroAmount);
        require_keys_eq!(
            ctx.accounts.config.vault_authority,
            ctx.accounts.vault_authority.key(),
            VaultError::StandInNotRetired
        );
        require_keys_eq!(
            ctx.accounts.escrow_auth.key(),
            ctx.accounts.vault_authority.key(),
            VaultError::BadEscrowAuth
        );
        let expected = ephemeral_balance_pda_from_payer(
            &ctx.accounts.escrow_auth.key(),
            ACTION_ESCROW_INDEX,
        );
        require_keys_eq!(ctx.accounts.escrow.key(), expected, VaultError::BadEscrow);
        require_keys_eq!(
            ctx.accounts.user_usdc_ata.mint,
            ctx.accounts.config.usdc_mint,
            VaultError::BadMint
        );
        require_keys_eq!(
            ctx.accounts.user_usdc_ata.owner,
            ctx.accounts.user.key(),
            VaultError::BadDestOwner
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
                    to: ctx.accounts.user_usdc_ata.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
                &[seeds],
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
    /// CHECK: PDA signer; this account has no data and acts as ATA authority.
    #[account(seeds = [cc::SEED_VAULT_AUTHORITY], bump)]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(
        init,
        payer = admin,
        space = ReserveRoot::ACCOUNT_SPACE,
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
pub struct AdapterConfig<'info> {
    pub adapter: Signer<'info>,
    #[account(mut, seeds = [cc::SEED_CONFIG], bump = config.bump_config,
        has_one = adapter @ VaultError::Unauthorized)]
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

#[derive(Accounts)]
pub struct UserWithdrawL1<'info> {
    pub adapter: Signer<'info>,
    /// CHECK: recipient; must own `user_usdc_ata`.
    pub user: UncheckedAccount<'info>,
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
    pub user_usdc_ata: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct WriteReserveRoot<'info> {
    pub adapter: Signer<'info>,
    #[account(
        seeds = [cc::SEED_CONFIG],
        bump = config.bump_config
    )]
    pub config: Account<'info, Config>,
    #[account(
        mut,
        seeds = [cc::SEED_RESERVE],
        bump
    )]
    pub reserve_root: Account<'info, ReserveRoot>,
}

#[derive(Accounts)]
pub struct RetireStandIn<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [cc::SEED_CONFIG],
        bump = config.bump_config,
        has_one = admin @ VaultError::Unauthorized
    )]
    pub config: Account<'info, Config>,
    /// CHECK: vault-authority PDA; becomes Config.vault_authority.
    #[account(
        seeds = [cc::SEED_VAULT_AUTHORITY],
        bump = config.bump_vault_authority
    )]
    pub vault_authority: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct PullCollateralPda<'info> {
    pub adapter: Signer<'info>,
    #[account(
        seeds = [cc::SEED_CONFIG],
        bump = config.bump_config
    )]
    pub config: Account<'info, Config>,
    /// CHECK: vault-authority PDA signer.
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
    pub source_usdc_ata: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct SettleUserWithdraw<'info> {
    pub adapter: Signer<'info>,
    /// CHECK: recipient; must own `user_usdc_ata`.
    pub user: UncheckedAccount<'info>,
    #[account(
        seeds = [cc::SEED_CONFIG],
        bump = config.bump_config
    )]
    pub config: Account<'info, Config>,
    /// CHECK: vault-authority PDA; escrow_auth must match.
    #[account(
        seeds = [cc::SEED_VAULT_AUTHORITY],
        bump = config.bump_vault_authority
    )]
    pub vault_authority: UncheckedAccount<'info>,
    /// CHECK: bound to vault-authority PDA (Magic Action escrow_auth).
    pub escrow_auth: UncheckedAccount<'info>,
    /// CHECK: `ephemeral_balance_pda_from_payer(escrow_auth, 255)`.
    pub escrow: UncheckedAccount<'info>,
    #[account(
        mut,
        address = config.vault_usdc_ata
    )]
    pub vault_usdc_ata: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user_usdc_ata: Account<'info, TokenAccount>,
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
    pub schema_version: u8,
    pub epoch: u64,
    pub root: [u8; 32],
    pub user_count: u32,
    pub total_free: u64,
    pub total_reserved: u64,
    pub total_bad_debt: u64,
    pub book_hash: [u8; 32],
    pub committed_at_base_slot: u64,
}

impl ReserveRoot {
    pub const ACCOUNT_SPACE: usize = 8 + Self::INIT_SPACE;
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
    #[msg("user ATA owner mismatch")]
    BadDestOwner,
    #[msg("overflow")]
    Overflow,
    #[msg("stand-in authority is not retired")]
    StandInNotRetired,
    #[msg("escrow_auth must be the vault-authority PDA")]
    BadEscrowAuth,
    #[msg("escrow PDA mismatch")]
    BadEscrow,
    #[msg("account has already been migrated")]
    AccountAlreadyMigrated,
    #[msg("unsupported account schema")]
    UnsupportedAccountSchema,
    #[msg("invalid legacy account data")]
    InvalidMigrationData,
}
