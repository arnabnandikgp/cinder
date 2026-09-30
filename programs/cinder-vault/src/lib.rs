//! Public custody evidence, not the private financial ledger or a venue authority bridge.
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, TransferChecked};

// Local development identity only. G03/G05 must approve a fresh deployment identity.
declare_id!("FMu4FVg4kWABgsPg8Q5Q8Z3wieTtVgtG2gapCSdGT7RX");

pub const NORMAL: u8 = 0;
pub const FROZEN: u8 = 1;
const SCHEMA: u8 = 1;

#[program]
pub mod cinder_vault {
    use super::*;

    /// Only this executable's actual loader upgrade authority can bootstrap custody.
    pub fn initialize(
        ctx: Context<Initialize>,
        domain: [u8; 32],
        pool: [u8; 32],
        funding_cap: u64,
        payout_cap: u64,
    ) -> Result<()> {
        nonzero(&domain)?;
        nonzero(&pool)?;
        require!(funding_cap > 0 && payout_cap > 0, VaultError::Bound);
        require!(ctx.accounts.mint.decimals <= 18, VaultError::Mint);
        roles(
            ctx.accounts.governance.key(),
            ctx.accounts.funds.key(),
            ctx.accounts.recovery.key(),
            ctx.accounts.broker_tokens.owner,
        )?;
        clean(&ctx.accounts.broker_tokens)?;
        let c = &mut ctx.accounts.config;
        c.schema = SCHEMA;
        c.bump = ctx.bumps.config;
        c.vault_bump = ctx.bumps.vault;
        c.domain = domain;
        c.pool = pool;
        c.mint = ctx.accounts.mint.key();
        c.decimals = ctx.accounts.mint.decimals;
        c.governance = ctx.accounts.governance.key();
        c.funds = ctx.accounts.funds.key();
        c.recovery = ctx.accounts.recovery.key();
        c.broker = ctx.accounts.broker_tokens.owner;
        c.broker_tokens = ctx.accounts.broker_tokens.key();
        c.epoch = 1;
        c.funding_cap = funding_cap;
        c.payout_cap = payout_cap;
        Ok(())
    }

    /// Public wallet attribution only; never stores current trading equity or positions.
    pub fn register_customer(ctx: Context<RegisterCustomer>, domain: [u8; 32]) -> Result<()> {
        ctx.accounts
            .config
            .normal(domain, ctx.accounts.config.epoch)?;
        let c = &mut ctx.accounts.customer;
        c.config = ctx.accounts.config.key();
        c.owner = ctx.accounts.owner.key();
        c.bump = ctx.bumps.customer;
        Ok(())
    }

    /// An immutable receipt is created only in the same transaction as actual token arrival.
    pub fn deposit(ctx: Context<Deposit>, auth: Movement, amount: u64) -> Result<()> {
        ctx.accounts.config.normal(auth.domain, auth.epoch)?;
        auth.valid(amount)?;
        clean(&ctx.accounts.source)?;
        clean(&ctx.accounts.vault)?;
        let total = add(ctx.accounts.config.deposited, amount)?;
        let credited = add(ctx.accounts.customer.deposited, amount)?;
        token::transfer_checked(
            CpiContext::new(
                token::ID,
                TransferChecked {
                    from: ctx.accounts.source.to_account_info(),
                    to: ctx.accounts.vault.to_account_info(),
                    authority: ctx.accounts.owner.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                },
            ),
            amount,
            ctx.accounts.config.decimals,
        )?;
        ctx.accounts.config.deposited = total;
        ctx.accounts.customer.deposited = credited;
        ctx.accounts.receipt.record(
            ctx.accounts.config.key(),
            &auth,
            0,
            (ctx.accounts.owner.key(), ctx.accounts.vault.key()),
            amount,
            0,
        );
        Ok(())
    }

    /// Releases working collateral to one immutable, explicitly allowlisted broker route.
    /// Epoch budget is gross cumulative release; returns do not silently replenish it.
    pub fn release_funding(
        ctx: Context<ReleaseFunding>,
        auth: Movement,
        amount: u64,
        expected_sequence: u64,
    ) -> Result<()> {
        let c = &ctx.accounts.config;
        c.normal(auth.domain, auth.epoch)?;
        auth.valid(amount)?;
        require!(expected_sequence == c.funding_sequence, VaultError::Counter);
        let released = add(c.epoch_released, amount)?;
        require!(released <= c.funding_cap, VaultError::Bound);
        let sequence = add(c.funding_sequence, 1)?;
        clean(&ctx.accounts.broker_tokens)?;
        pay(
            c,
            &ctx.accounts.vault,
            &ctx.accounts.broker_tokens,
            &ctx.accounts.mint,
            amount,
        )?;
        ctx.accounts.config.epoch_released = released;
        ctx.accounts.config.funding_sequence = sequence;
        ctx.accounts.receipt.record(
            ctx.accounts.config.key(),
            &auth,
            1,
            (ctx.accounts.funds.key(), ctx.accounts.broker_tokens.key()),
            amount,
            sequence,
        );
        Ok(())
    }

    /// Returning custody does not credit a customer again. Permitted while frozen too.
    pub fn return_funding(ctx: Context<ReturnFunding>, auth: Movement, amount: u64) -> Result<()> {
        ctx.accounts.config.bound(auth.domain, auth.epoch)?;
        auth.valid(amount)?;
        clean(&ctx.accounts.source)?;
        clean(&ctx.accounts.vault)?;
        let returned = add(ctx.accounts.config.returned, amount)?;
        token::transfer_checked(
            CpiContext::new(
                token::ID,
                TransferChecked {
                    from: ctx.accounts.source.to_account_info(),
                    to: ctx.accounts.vault.to_account_info(),
                    authority: ctx.accounts.broker.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                },
            ),
            amount,
            ctx.accounts.config.decimals,
        )?;
        ctx.accounts.config.returned = returned;
        ctx.accounts.receipt.record(
            ctx.accounts.config.key(),
            &auth,
            2,
            (ctx.accounts.broker.key(), ctx.accounts.vault.key()),
            amount,
            0,
        );
        Ok(())
    }

    /// Funds authority approves a ledger-qualified payout; deposits are NOT its entitlement cap.
    /// Customer/asset paid totals and sequence survive authority rotation and later recovery.
    pub fn normal_payout(
        ctx: Context<NormalPayout>,
        auth: Movement,
        amount: u64,
        expected_paid: u64,
        expected_sequence: u64,
    ) -> Result<()> {
        let c = &ctx.accounts.config;
        c.normal(auth.domain, auth.epoch)?;
        auth.valid(amount)?;
        require!(amount <= c.payout_cap, VaultError::Bound);
        require!(
            expected_paid == ctx.accounts.customer.paid
                && expected_sequence == ctx.accounts.customer.payout_sequence,
            VaultError::Counter
        );
        let paid = add(expected_paid, amount)?;
        let sequence = add(expected_sequence, 1)?;
        let total = add(c.paid, amount)?;
        clean(&ctx.accounts.destination)?;
        pay(
            c,
            &ctx.accounts.vault,
            &ctx.accounts.destination,
            &ctx.accounts.mint,
            amount,
        )?;
        ctx.accounts.customer.paid = paid;
        ctx.accounts.customer.payout_sequence = sequence;
        ctx.accounts.config.paid = total;
        ctx.accounts.receipt.record(
            ctx.accounts.config.key(),
            &auth,
            3,
            (ctx.accounts.customer.owner, ctx.accounts.destination.key()),
            amount,
            sequence,
        );
        Ok(())
    }

    /// Atomic role handoff, signed by the old and new governance. Epoch invalidates old wires.
    /// This changes no historical receipt or paid counter; it cannot revive a frozen pool.
    pub fn rotate(
        ctx: Context<Rotate>,
        domain: [u8; 32],
        expected_epoch: u64,
        funding_cap: u64,
        payout_cap: u64,
    ) -> Result<()> {
        let c = &mut ctx.accounts.config;
        c.normal(domain, expected_epoch)?;
        require!(funding_cap > 0 && payout_cap > 0, VaultError::Bound);
        roles(
            ctx.accounts.new_governance.key(),
            ctx.accounts.new_funds.key(),
            ctx.accounts.new_recovery.key(),
            c.broker,
        )?;
        c.epoch = add(c.epoch, 1)?;
        c.governance = ctx.accounts.new_governance.key();
        c.funds = ctx.accounts.new_funds.key();
        c.recovery = ctx.accounts.new_recovery.key();
        c.funding_cap = funding_cap;
        c.payout_cap = payout_cap;
        c.epoch_released = 0;
        Ok(())
    }

    /// Emergency fence only. No heartbeat, root publication, recovery payout or resume path.
    pub fn freeze(ctx: Context<Freeze>, domain: [u8; 32], expected_epoch: u64) -> Result<()> {
        ctx.accounts.config.normal(domain, expected_epoch)?;
        ctx.accounts.config.mode = FROZEN;
        ctx.accounts.config.epoch = add(expected_epoch, 1)?;
        Ok(())
    }
}

fn add(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b)
        .ok_or_else(|| error!(VaultError::Arithmetic))
}
fn nonzero(id: &[u8; 32]) -> Result<()> {
    require!(*id != [0; 32], VaultError::Domain);
    Ok(())
}
fn roles(governance: Pubkey, funds: Pubkey, recovery: Pubkey, broker: Pubkey) -> Result<()> {
    let keys = [governance, funds, recovery, broker];
    for (i, key) in keys.iter().enumerate() {
        require!(
            key.is_on_curve() && *key != Pubkey::default(),
            VaultError::Authority
        );
        require!(!keys[..i].contains(key), VaultError::RoleAlias);
    }
    Ok(())
}
fn clean(a: &Account<TokenAccount>) -> Result<()> {
    require!(
        a.delegate.is_none() && a.close_authority.is_none(),
        VaultError::TokenAuthority
    );
    Ok(())
}
fn pay<'info>(
    c: &Account<'info, VaultConfig>,
    vault: &Account<'info, TokenAccount>,
    to: &Account<'info, TokenAccount>,
    mint: &Account<'info, Mint>,
    amount: u64,
) -> Result<()> {
    clean(vault)?;
    require_keys_neq!(vault.key(), to.key(), VaultError::Recipient);
    let bump = [c.bump];
    let seeds: &[&[u8]] = &[b"cinder_vault", &c.domain, &c.pool, c.mint.as_ref(), &bump];
    token::transfer_checked(
        CpiContext::new_with_signer(
            token::ID,
            TransferChecked {
                from: vault.to_account_info(),
                to: to.to_account_info(),
                authority: c.to_account_info(),
                mint: mint.to_account_info(),
            },
            &[seeds],
        ),
        amount,
        c.decimals,
    )
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct Movement {
    pub domain: [u8; 32],
    pub epoch: u64,
    pub operation: [u8; 32],
    /// Latest acceptable execution slot, not evidence of external finality.
    pub expires_at_slot: u64,
}
impl Movement {
    fn valid(&self, amount: u64) -> Result<()> {
        nonzero(&self.operation)?;
        require!(amount > 0, VaultError::Bound);
        require!(
            Clock::get()?.slot <= self.expires_at_slot,
            VaultError::Expired
        );
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(domain: [u8; 32], pool: [u8; 32])]
pub struct Initialize<'info> {
    #[account(mut)]
    pub governance: Signer<'info>,
    pub funds: Signer<'info>,
    pub recovery: Signer<'info>,
    #[account(constraint = program.programdata_address()? == Some(program_data.key()) @ VaultError::Authority)]
    pub program: Program<'info, crate::program::CinderVault>,
    #[account(constraint = program_data.upgrade_authority_address == Some(governance.key()) @ VaultError::Authority)]
    pub program_data: Account<'info, ProgramData>,
    pub mint: Box<Account<'info, Mint>>,
    #[account(token::mint = mint, token::token_program = token_program)]
    pub broker_tokens: Box<Account<'info, TokenAccount>>,
    #[account(init, payer = governance, space = 8 + VaultConfig::INIT_SPACE,
        seeds = [b"cinder_vault", domain.as_ref(), pool.as_ref(), mint.key().as_ref()], bump)]
    pub config: Box<Account<'info, VaultConfig>>,
    #[account(init, payer = governance, seeds = [b"tokens", config.key().as_ref()], bump,
        token::mint = mint, token::authority = config, token::token_program = token_program)]
    pub vault: Box<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RegisterCustomer<'info> {
    #[account(seeds = [b"cinder_vault", config.domain.as_ref(), config.pool.as_ref(), config.mint.as_ref()], bump = config.bump)]
    pub config: Box<Account<'info, VaultConfig>>,
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(init, payer = owner, space = 8 + CustomerCounter::INIT_SPACE,
        seeds = [b"customer", config.key().as_ref(), owner.key().as_ref()], bump)]
    pub customer: Box<Account<'info, CustomerCounter>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(auth: Movement)]
pub struct Deposit<'info> {
    #[account(mut, seeds = [b"cinder_vault", config.domain.as_ref(), config.pool.as_ref(), config.mint.as_ref()], bump = config.bump, has_one = mint)]
    pub config: Box<Account<'info, VaultConfig>>,
    #[account(mut, seeds = [b"customer", config.key().as_ref(), owner.key().as_ref()], bump = customer.bump, has_one = config, has_one = owner)]
    pub customer: Box<Account<'info, CustomerCounter>>,
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut, token::mint = mint, token::authority = owner, token::token_program = token_program)]
    pub source: Box<Account<'info, TokenAccount>>,
    #[account(mut, seeds = [b"tokens", config.key().as_ref()], bump = config.vault_bump,
        token::mint = mint, token::authority = config, token::token_program = token_program)]
    pub vault: Box<Account<'info, TokenAccount>>,
    pub mint: Box<Account<'info, Mint>>,
    #[account(init, payer = owner, space = 8 + MovementReceipt::INIT_SPACE,
        seeds = [b"deposit", config.key().as_ref(), owner.key().as_ref(), auth.operation.as_ref()], bump)]
    pub receipt: Box<Account<'info, MovementReceipt>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(auth: Movement)]
pub struct ReleaseFunding<'info> {
    #[account(mut, seeds = [b"cinder_vault", config.domain.as_ref(), config.pool.as_ref(), config.mint.as_ref()], bump = config.bump, has_one = mint, has_one = funds, has_one = broker_tokens)]
    pub config: Box<Account<'info, VaultConfig>>,
    #[account(mut)]
    pub funds: Signer<'info>,
    #[account(mut, token::mint = mint, constraint = broker_tokens.owner == config.broker @ VaultError::Authority, token::token_program = token_program)]
    pub broker_tokens: Box<Account<'info, TokenAccount>>,
    #[account(mut, seeds = [b"tokens", config.key().as_ref()], bump = config.vault_bump,
        token::mint = mint, token::authority = config, token::token_program = token_program)]
    pub vault: Box<Account<'info, TokenAccount>>,
    pub mint: Box<Account<'info, Mint>>,
    #[account(init, payer = funds, space = 8 + MovementReceipt::INIT_SPACE,
        seeds = [b"receipt", config.key().as_ref(), auth.operation.as_ref()], bump)]
    pub receipt: Box<Account<'info, MovementReceipt>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(auth: Movement)]
pub struct ReturnFunding<'info> {
    #[account(mut, seeds = [b"cinder_vault", config.domain.as_ref(), config.pool.as_ref(), config.mint.as_ref()], bump = config.bump, has_one = mint, has_one = broker)]
    pub config: Box<Account<'info, VaultConfig>>,
    #[account(mut)]
    pub broker: Signer<'info>,
    #[account(mut, address = config.broker_tokens, token::mint = mint, token::authority = broker, token::token_program = token_program)]
    pub source: Box<Account<'info, TokenAccount>>,
    #[account(mut, seeds = [b"tokens", config.key().as_ref()], bump = config.vault_bump,
        token::mint = mint, token::authority = config, token::token_program = token_program)]
    pub vault: Box<Account<'info, TokenAccount>>,
    pub mint: Box<Account<'info, Mint>>,
    #[account(init, payer = broker, space = 8 + MovementReceipt::INIT_SPACE,
        seeds = [b"receipt", config.key().as_ref(), auth.operation.as_ref()], bump)]
    pub receipt: Box<Account<'info, MovementReceipt>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(auth: Movement)]
pub struct NormalPayout<'info> {
    #[account(mut, seeds = [b"cinder_vault", config.domain.as_ref(), config.pool.as_ref(), config.mint.as_ref()], bump = config.bump, has_one = mint, has_one = funds)]
    pub config: Box<Account<'info, VaultConfig>>,
    #[account(mut)]
    pub funds: Signer<'info>,
    /// CHECK: Bound to the initialized customer's wallet; receives no authority privilege.
    #[account(address = customer.owner)]
    pub owner: UncheckedAccount<'info>,
    #[account(mut, seeds = [b"customer", config.key().as_ref(), owner.key().as_ref()], bump = customer.bump, has_one = config, has_one = owner)]
    pub customer: Box<Account<'info, CustomerCounter>>,
    #[account(mut, token::mint = mint, token::authority = owner, token::token_program = token_program)]
    pub destination: Box<Account<'info, TokenAccount>>,
    #[account(mut, seeds = [b"tokens", config.key().as_ref()], bump = config.vault_bump,
        token::mint = mint, token::authority = config, token::token_program = token_program)]
    pub vault: Box<Account<'info, TokenAccount>>,
    pub mint: Box<Account<'info, Mint>>,
    #[account(init, payer = funds, space = 8 + MovementReceipt::INIT_SPACE,
        seeds = [b"receipt", config.key().as_ref(), auth.operation.as_ref()], bump)]
    pub receipt: Box<Account<'info, MovementReceipt>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Rotate<'info> {
    #[account(mut, seeds = [b"cinder_vault", config.domain.as_ref(), config.pool.as_ref(), config.mint.as_ref()], bump = config.bump, has_one = governance)]
    pub config: Box<Account<'info, VaultConfig>>,
    pub governance: Signer<'info>,
    pub new_governance: Signer<'info>,
    pub new_funds: Signer<'info>,
    pub new_recovery: Signer<'info>,
}

#[derive(Accounts)]
pub struct Freeze<'info> {
    #[account(mut, seeds = [b"cinder_vault", config.domain.as_ref(), config.pool.as_ref(), config.mint.as_ref()], bump = config.bump, has_one = recovery)]
    pub config: Box<Account<'info, VaultConfig>>,
    pub recovery: Signer<'info>,
}

#[account]
#[derive(InitSpace)]
pub struct VaultConfig {
    pub schema: u8,
    pub bump: u8,
    pub vault_bump: u8,
    pub decimals: u8,
    pub domain: [u8; 32],
    pub pool: [u8; 32],
    pub mint: Pubkey,
    pub governance: Pubkey,
    pub funds: Pubkey,
    pub recovery: Pubkey,
    pub broker: Pubkey,
    pub broker_tokens: Pubkey,
    pub epoch: u64,
    pub mode: u8,
    pub funding_cap: u64,
    pub payout_cap: u64,
    pub epoch_released: u64,
    pub funding_sequence: u64,
    pub deposited: u64,
    pub returned: u64,
    pub paid: u64,
}
impl VaultConfig {
    fn bound(&self, domain: [u8; 32], epoch: u64) -> Result<()> {
        require!(
            self.schema == SCHEMA && domain == self.domain && epoch == self.epoch,
            VaultError::Domain
        );
        Ok(())
    }
    fn normal(&self, domain: [u8; 32], epoch: u64) -> Result<()> {
        self.bound(domain, epoch)?;
        require!(self.mode == NORMAL, VaultError::Mode);
        Ok(())
    }
}

#[account]
#[derive(InitSpace)]
pub struct CustomerCounter {
    pub config: Pubkey,
    pub owner: Pubkey,
    pub bump: u8,
    pub deposited: u64,
    /// Shared normal/recovery total, never reset on authority or recovery-root changes.
    pub paid: u64,
    pub payout_sequence: u64,
}

#[account]
#[derive(InitSpace)]
pub struct MovementReceipt {
    pub config: Pubkey,
    pub operation: [u8; 32],
    pub epoch: u64,
    pub kind: u8,
    pub owner: Pubkey,
    pub destination: Pubkey,
    pub amount: u64,
    pub sequence: u64,
}
impl MovementReceipt {
    fn record(
        &mut self,
        config: Pubkey,
        auth: &Movement,
        kind: u8,
        parties: (Pubkey, Pubkey),
        amount: u64,
        sequence: u64,
    ) {
        self.config = config;
        self.operation = auth.operation;
        self.epoch = auth.epoch;
        self.kind = kind;
        self.owner = parties.0;
        self.destination = parties.1;
        self.amount = amount;
        self.sequence = sequence;
    }
}

#[error_code]
pub enum VaultError {
    #[msg("Wrong deployment domain, authority epoch or empty identity")]
    Domain,
    #[msg("Unauthorized account or non-signing role")]
    Authority,
    #[msg("Custody roles must be distinct")]
    RoleAlias,
    #[msg("Unsupported mint precision")]
    Mint,
    #[msg("Delegated or externally closeable token account")]
    TokenAuthority,
    #[msg("Invalid custody mode")]
    Mode,
    #[msg("Custody amount exceeds its bound")]
    Bound,
    #[msg("Stale funding or payout counter")]
    Counter,
    #[msg("Movement authorization expired")]
    Expired,
    #[msg("Invalid recipient")]
    Recipient,
    #[msg("Integer overflow")]
    Arithmetic,
}
