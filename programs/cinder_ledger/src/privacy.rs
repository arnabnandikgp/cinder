//! L1 delegation and ER EphemeralPermission setup. Permissions are ER-local.
//! `delegate_user` lives here (ledger-owned PDAs), not on `cinder_vault`.

use super::*;
use anchor_lang::system_program::{transfer, Transfer};
use ephemeral_rollups_sdk::access_control::{
    instructions::CreateEphemeralPermissionCpi,
    structs::{
        EphemeralMembersArgs, EphemeralPermission, Member, PERMISSION_SEED, AUTHORITY_FLAG,
        ACCOUNT_SIGNATURES_FLAG, TX_BALANCES_FLAG, TX_LOGS_FLAG, TX_MESSAGE_FLAG,
    },
};
use ephemeral_rollups_sdk::anchor::delegate;
use ephemeral_rollups_sdk::consts::{EPHEMERAL_VAULT_ID, MAGIC_PROGRAM_ID, PERMISSION_PROGRAM_ID};
use ephemeral_rollups_sdk::cpi::DelegateConfig;

const VIEW_FLAGS: u8 =
    TX_BALANCES_FLAG | TX_LOGS_FLAG | TX_MESSAGE_FLAG | ACCOUNT_SIGNATURES_FLAG;
const ADAPTER_FLAGS: u8 = AUTHORITY_FLAG | VIEW_FLAGS;

pub fn fund_permission_rent<'info>(
    from: AccountInfo<'info>,
    to: AccountInfo<'info>,
    system_program: AccountInfo<'info>,
    members: usize,
) -> Result<()> {
    let lamports = ephemeral_rollups_sdk::ephemeral_accounts::rent(
        EphemeralPermission::size_of(members) as u32,
    );
    transfer(
        CpiContext::new(
            system_program.key(),
            Transfer { from, to },
        ),
        lamports,
    )
}

fn permission_exists(permission: &AccountInfo) -> bool {
    permission.owner == &PERMISSION_PROGRAM_ID && !permission.data_is_empty()
}

pub fn delegate_user_handler(ctx: Context<DelegateUser>) -> Result<()> {
    let cfg = load_vault_config(&ctx.accounts.config)?;
    require_keys_eq!(cfg.adapter, ctx.accounts.adapter.key(), LedgerError::Unauthorized);
    let validator = ctx
        .accounts
        .validator
        .as_ref()
        .ok_or(LedgerError::MissingValidator)?;
    require_keys_eq!(validator.key(), cfg.er_validator, LedgerError::BadValidator);

    if ctx.accounts.user_ledger.owner == &ephemeral_rollups_sdk::id() {
        msg!("user ledger already delegated");
        return Ok(());
    }
    ctx.accounts.delegate_user_ledger(
        &ctx.accounts.adapter,
        &[cc::SEED_USER, ctx.accounts.user.key().as_ref()],
        DelegateConfig {
            validator: Some(validator.key()),
            ..Default::default()
        },
    )?;
    Ok(())
}

pub fn delegate_book_handler(ctx: Context<DelegateBook>) -> Result<()> {
    let cfg = load_vault_config(&ctx.accounts.config)?;
    require_keys_eq!(cfg.adapter, ctx.accounts.adapter.key(), LedgerError::Unauthorized);
    let validator = ctx
        .accounts
        .validator
        .as_ref()
        .ok_or(LedgerError::MissingValidator)?;
    require_keys_eq!(validator.key(), cfg.er_validator, LedgerError::BadValidator);

    if ctx.accounts.book.owner == &ephemeral_rollups_sdk::id() {
        msg!("book already delegated");
        return Ok(());
    }
    ctx.accounts.delegate_book(
        &ctx.accounts.adapter,
        &[cc::SEED_BOOK],
        DelegateConfig {
            validator: Some(validator.key()),
            ..Default::default()
        },
    )?;
    Ok(())
}

pub fn delegate_fees_handler(ctx: Context<DelegateFees>) -> Result<()> {
    let cfg = load_vault_config(&ctx.accounts.config)?;
    require_keys_eq!(cfg.adapter, ctx.accounts.adapter.key(), LedgerError::Unauthorized);
    let validator = ctx
        .accounts
        .validator
        .as_ref()
        .ok_or(LedgerError::MissingValidator)?;
    require_keys_eq!(validator.key(), cfg.er_validator, LedgerError::BadValidator);

    if ctx.accounts.fee_accrual.owner == &ephemeral_rollups_sdk::id() {
        msg!("fees already delegated");
        return Ok(());
    }
    ctx.accounts.delegate_fee_accrual(
        &ctx.accounts.adapter,
        &[cc::SEED_FEES],
        DelegateConfig {
            validator: Some(validator.key()),
            ..Default::default()
        },
    )?;
    Ok(())
}

pub fn init_user_permission_handler(ctx: Context<UserPermission>) -> Result<()> {
    let cfg = load_vault_config(&ctx.accounts.config)?;
    require_keys_eq!(cfg.adapter, ctx.accounts.adapter.key(), LedgerError::Unauthorized);
    if permission_exists(&ctx.accounts.permission) {
        msg!("user permission already exists");
        return Ok(());
    }
    let signers: &[&[u8]] = &[
        cc::SEED_USER,
        ctx.accounts.user_ledger.user.as_ref(),
        &[ctx.accounts.user_ledger.bump],
    ];
    let members = vec![
        Member {
            flags: VIEW_FLAGS,
            pubkey: ctx.accounts.user_ledger.user,
        },
        Member {
            flags: ADAPTER_FLAGS,
            pubkey: cfg.adapter,
        },
    ];
    CreateEphemeralPermissionCpi {
        payer: ctx.accounts.user_ledger.to_account_info(),
        permissioned_account: ctx.accounts.user_ledger.to_account_info(),
        permission: ctx.accounts.permission.to_account_info(),
        vault: ctx.accounts.ephemeral_vault.to_account_info(),
        magic_program: ctx.accounts.magic_program.to_account_info(),
        permission_program: ctx.accounts.permission_program.to_account_info(),
        args: EphemeralMembersArgs {
            is_private: true,
            members,
        },
    }
    .invoke_signed(&[signers])?;
    Ok(())
}

pub fn init_book_permission_handler(ctx: Context<BookPermission>) -> Result<()> {
    let cfg = load_vault_config(&ctx.accounts.config)?;
    require_keys_eq!(cfg.adapter, ctx.accounts.adapter.key(), LedgerError::Unauthorized);
    if permission_exists(&ctx.accounts.permission) {
        msg!("book permission already exists");
        return Ok(());
    }
    let signers: &[&[u8]] = &[cc::SEED_BOOK, &[ctx.accounts.book.bump]];
    let members = vec![Member {
        flags: ADAPTER_FLAGS,
        pubkey: cfg.adapter,
    }];
    CreateEphemeralPermissionCpi {
        payer: ctx.accounts.book.to_account_info(),
        permissioned_account: ctx.accounts.book.to_account_info(),
        permission: ctx.accounts.permission.to_account_info(),
        vault: ctx.accounts.ephemeral_vault.to_account_info(),
        magic_program: ctx.accounts.magic_program.to_account_info(),
        permission_program: ctx.accounts.permission_program.to_account_info(),
        args: EphemeralMembersArgs {
            is_private: true,
            members,
        },
    }
    .invoke_signed(&[signers])?;
    Ok(())
}

pub fn init_fees_permission_handler(ctx: Context<FeesPermission>) -> Result<()> {
    let cfg = load_vault_config(&ctx.accounts.config)?;
    require_keys_eq!(cfg.adapter, ctx.accounts.adapter.key(), LedgerError::Unauthorized);
    if permission_exists(&ctx.accounts.permission) {
        msg!("fees permission already exists");
        return Ok(());
    }
    let signers: &[&[u8]] = &[cc::SEED_FEES, &[ctx.accounts.fee_accrual.bump]];
    let members = vec![Member {
        flags: ADAPTER_FLAGS,
        pubkey: cfg.adapter,
    }];
    CreateEphemeralPermissionCpi {
        payer: ctx.accounts.fee_accrual.to_account_info(),
        permissioned_account: ctx.accounts.fee_accrual.to_account_info(),
        permission: ctx.accounts.permission.to_account_info(),
        vault: ctx.accounts.ephemeral_vault.to_account_info(),
        magic_program: ctx.accounts.magic_program.to_account_info(),
        permission_program: ctx.accounts.permission_program.to_account_info(),
        args: EphemeralMembersArgs {
            is_private: true,
            members,
        },
    }
    .invoke_signed(&[signers])?;
    Ok(())
}

#[delegate]
#[derive(Accounts)]
pub struct DelegateUser<'info> {
    #[account(mut)]
    pub adapter: Signer<'info>,
    pub user: Signer<'info>,
    /// CHECK: vault Config
    #[account(
        seeds = [cc::SEED_CONFIG],
        bump,
        seeds::program = VAULT_PROGRAM_ID,
        owner = VAULT_PROGRAM_ID
    )]
    pub config: UncheckedAccount<'info>,
    /// CHECK: UserLedger PDA; `del` constraint from the delegate macro.
    #[account(mut, del, seeds = [cc::SEED_USER, user.key().as_ref()], bump)]
    pub user_ledger: UncheckedAccount<'info>,
    /// CHECK: must equal Config.er_validator (local ER id).
    pub validator: Option<UncheckedAccount<'info>>,
}

#[delegate]
#[derive(Accounts)]
pub struct DelegateBook<'info> {
    #[account(mut)]
    pub adapter: Signer<'info>,
    /// CHECK: vault Config
    #[account(
        seeds = [cc::SEED_CONFIG],
        bump,
        seeds::program = VAULT_PROGRAM_ID,
        owner = VAULT_PROGRAM_ID
    )]
    pub config: UncheckedAccount<'info>,
    /// CHECK: Book PDA
    #[account(mut, del, seeds = [cc::SEED_BOOK], bump)]
    pub book: UncheckedAccount<'info>,
    /// CHECK: must equal Config.er_validator
    pub validator: Option<UncheckedAccount<'info>>,
}

#[delegate]
#[derive(Accounts)]
pub struct DelegateFees<'info> {
    #[account(mut)]
    pub adapter: Signer<'info>,
    /// CHECK: vault Config
    #[account(
        seeds = [cc::SEED_CONFIG],
        bump,
        seeds::program = VAULT_PROGRAM_ID,
        owner = VAULT_PROGRAM_ID
    )]
    pub config: UncheckedAccount<'info>,
    /// CHECK: FeeAccrual PDA
    #[account(mut, del, seeds = [cc::SEED_FEES], bump)]
    pub fee_accrual: UncheckedAccount<'info>,
    /// CHECK: must equal Config.er_validator
    pub validator: Option<UncheckedAccount<'info>>,
}

#[derive(Accounts)]
pub struct UserPermission<'info> {
    pub adapter: Signer<'info>,
    /// CHECK: vault Config (adapter pubkey + ACL source)
    #[account(
        seeds = [cc::SEED_CONFIG],
        bump,
        seeds::program = VAULT_PROGRAM_ID,
        owner = VAULT_PROGRAM_ID
    )]
    pub config: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [cc::SEED_USER, user_ledger.user.as_ref()],
        bump = user_ledger.bump
    )]
    pub user_ledger: Box<Account<'info, UserLedger>>,
    /// CHECK: Permission program PDA for this ledger.
    #[account(
        mut,
        seeds = [PERMISSION_SEED, user_ledger.key().as_ref()],
        bump,
        seeds::program = PERMISSION_PROGRAM_ID
    )]
    pub permission: UncheckedAccount<'info>,
    /// CHECK: Permission Program
    #[account(address = PERMISSION_PROGRAM_ID)]
    pub permission_program: UncheckedAccount<'info>,
    /// CHECK: ER rent vault
    #[account(mut, address = EPHEMERAL_VAULT_ID)]
    pub ephemeral_vault: UncheckedAccount<'info>,
    /// CHECK: Magic Program
    #[account(address = MAGIC_PROGRAM_ID)]
    pub magic_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct BookPermission<'info> {
    pub adapter: Signer<'info>,
    /// CHECK: vault Config
    #[account(
        seeds = [cc::SEED_CONFIG],
        bump,
        seeds::program = VAULT_PROGRAM_ID,
        owner = VAULT_PROGRAM_ID
    )]
    pub config: UncheckedAccount<'info>,
    #[account(mut, seeds = [cc::SEED_BOOK], bump = book.bump)]
    pub book: Box<Account<'info, Book>>,
    /// CHECK: Permission program PDA for Book.
    #[account(
        mut,
        seeds = [PERMISSION_SEED, book.key().as_ref()],
        bump,
        seeds::program = PERMISSION_PROGRAM_ID
    )]
    pub permission: UncheckedAccount<'info>,
    /// CHECK: Permission Program
    #[account(address = PERMISSION_PROGRAM_ID)]
    pub permission_program: UncheckedAccount<'info>,
    /// CHECK: ER rent vault
    #[account(mut, address = EPHEMERAL_VAULT_ID)]
    pub ephemeral_vault: UncheckedAccount<'info>,
    /// CHECK: Magic Program
    #[account(address = MAGIC_PROGRAM_ID)]
    pub magic_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct FeesPermission<'info> {
    pub adapter: Signer<'info>,
    /// CHECK: vault Config
    #[account(
        seeds = [cc::SEED_CONFIG],
        bump,
        seeds::program = VAULT_PROGRAM_ID,
        owner = VAULT_PROGRAM_ID
    )]
    pub config: UncheckedAccount<'info>,
    #[account(mut, seeds = [cc::SEED_FEES], bump = fee_accrual.bump)]
    pub fee_accrual: Box<Account<'info, FeeAccrual>>,
    /// CHECK: Permission program PDA for FeeAccrual.
    #[account(
        mut,
        seeds = [PERMISSION_SEED, fee_accrual.key().as_ref()],
        bump,
        seeds::program = PERMISSION_PROGRAM_ID
    )]
    pub permission: UncheckedAccount<'info>,
    /// CHECK: Permission Program
    #[account(address = PERMISSION_PROGRAM_ID)]
    pub permission_program: UncheckedAccount<'info>,
    /// CHECK: ER rent vault
    #[account(mut, address = EPHEMERAL_VAULT_ID)]
    pub ephemeral_vault: UncheckedAccount<'info>,
    /// CHECK: Magic Program
    #[account(address = MAGIC_PROGRAM_ID)]
    pub magic_program: UncheckedAccount<'info>,
}
