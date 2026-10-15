//! Atomic pending-owner custody; no native credit or financial recognition is inferred.
use super::*;

pub const PENDING: u8 = 0;
pub const SWEPT: u8 = 1;
pub const REFUNDED: u8 = 2;

pub fn register(
    ctx: Context<RegisterVenueRoute>,
    domain: [u8; 32],
    epoch: u64,
    venue: [u8; 32],
) -> Result<()> {
    ctx.accounts.config.normal(domain, epoch)?;
    nonzero(&venue)?;
    let r = &mut ctx.accounts.route;
    r.schema = SCHEMA;
    r.bump = ctx.bumps.route;
    r.config = ctx.accounts.config.key();
    r.venue = venue;
    r.native_account = ctx.accounts.config.broker;
    Ok(())
}
pub fn open(ctx: Context<OpenVenueEscrow>, domain: [u8; 32]) -> Result<()> {
    ctx.accounts
        .config
        .normal(domain, ctx.accounts.config.epoch)?;
    let e = &mut ctx.accounts.escrow;
    e.schema = SCHEMA;
    e.bump = ctx.bumps.escrow;
    e.tokens_bump = ctx.bumps.escrow_tokens;
    e.route = ctx.accounts.route.key();
    e.owner = ctx.accounts.owner.key();
    Ok(())
}
pub fn deposit_batch(ctx: Context<DepositEscrow>, auth: Movement, amount: u64) -> Result<()> {
    ctx.accounts.config.normal(auth.domain, auth.epoch)?;
    auth.valid(amount)?;
    clean(&ctx.accounts.source)?;
    backed(&ctx.accounts.escrow, &ctx.accounts.escrow_tokens)?;
    let pending = add(ctx.accounts.escrow.pending, amount)?;
    let b = &mut ctx.accounts.batch;
    b.schema = SCHEMA;
    b.bump = ctx.bumps.batch;
    b.state = PENDING;
    b.escrow = ctx.accounts.escrow.key();
    b.operation = auth.operation;
    b.amount = amount;
    b.epoch = auth.epoch;
    b.expires_at_slot = auth.expires_at_slot;
    ctx.accounts.escrow.pending = pending;
    token::transfer_checked(
        CpiContext::new(
            token::ID,
            TransferChecked {
                from: ctx.accounts.source.to_account_info(),
                to: ctx.accounts.escrow_tokens.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
            },
        ),
        amount,
        ctx.accounts.config.decimals,
    )
}
pub fn sweep(ctx: Context<SweepEscrow>, auth: Movement, expected_sequence: u64) -> Result<()> {
    let c = &ctx.accounts.config;
    c.normal(auth.domain, auth.epoch)?;
    let b = &ctx.accounts.batch;
    require!(b.state == PENDING, VaultError::EscrowConsumed);
    auth.valid(b.amount)?;
    require!(b.epoch == c.epoch, VaultError::Domain);
    require!(Clock::get()?.slot <= b.expires_at_slot, VaultError::Expired);
    require!(expected_sequence == c.funding_sequence, VaultError::Counter);
    let released = add(c.epoch_released, b.amount)?;
    require!(released <= c.funding_cap, VaultError::Bound);
    let sequence = add(c.funding_sequence, 1)?;
    let deposited = add(c.deposited, b.amount)?;
    let credited = add(ctx.accounts.customer.deposited, b.amount)?;
    let pending = consume(&ctx.accounts.escrow, &ctx.accounts.escrow_tokens, b.amount)?;
    clean(&ctx.accounts.broker_tokens)?;
    ctx.accounts.receipt.record(
        c.key(),
        &auth,
        4,
        (ctx.accounts.escrow.owner, ctx.accounts.broker_tokens.key()),
        b.amount,
        sequence,
    );
    let amount = b.amount;
    ctx.accounts.batch.finish(
        SWEPT,
        auth.operation,
        ctx.accounts.broker_tokens.key(),
        sequence,
    )?;
    ctx.accounts.config.epoch_released = released;
    ctx.accounts.config.funding_sequence = sequence;
    ctx.accounts.config.deposited = deposited;
    ctx.accounts.customer.deposited = credited;
    ctx.accounts.escrow.pending = pending;
    transfer_from_escrow(
        &ctx.accounts.escrow,
        &ctx.accounts.escrow_tokens,
        &ctx.accounts.broker_tokens,
        &ctx.accounts.mint,
        amount,
    )
}
pub fn refund(ctx: Context<RefundEscrow>, domain: [u8; 32]) -> Result<()> {
    // Pending owner custody is not a trading/recovery estate. Epoch/mode/expiry
    // cannot prevent this refund; domain/schema/account identities still bind it.
    ctx.accounts
        .config
        .bound(domain, ctx.accounts.config.epoch)?;
    let b = &ctx.accounts.batch;
    require!(b.state == PENDING, VaultError::EscrowConsumed);
    let amount = b.amount;
    let pending = consume(&ctx.accounts.escrow, &ctx.accounts.escrow_tokens, amount)?;
    clean(&ctx.accounts.destination)?;
    ctx.accounts
        .batch
        .finish(REFUNDED, [0; 32], ctx.accounts.destination.key(), 0)?;
    ctx.accounts.escrow.pending = pending;
    transfer_from_escrow(
        &ctx.accounts.escrow,
        &ctx.accounts.escrow_tokens,
        &ctx.accounts.destination,
        &ctx.accounts.mint,
        amount,
    )
}
fn backed(e: &VenueEscrow, tokens: &Account<TokenAccount>) -> Result<()> {
    clean(tokens)?;
    require!(tokens.amount >= e.pending, VaultError::EscrowBacking);
    Ok(())
}
fn consume(e: &VenueEscrow, tokens: &Account<TokenAccount>, amount: u64) -> Result<u64> {
    backed(e, tokens)?;
    require!(amount > 0, VaultError::Bound);
    e.pending
        .checked_sub(amount)
        .ok_or_else(|| error!(VaultError::EscrowBacking))
}
fn transfer_from_escrow<'info>(
    e: &Account<'info, VenueEscrow>,
    source: &Account<'info, TokenAccount>,
    destination: &Account<'info, TokenAccount>,
    mint: &Account<'info, Mint>,
    amount: u64,
) -> Result<()> {
    require_keys_neq!(source.key(), destination.key(), VaultError::Recipient);
    let bump = [e.bump];
    let seeds: &[&[u8]] = &[b"venue_escrow", e.route.as_ref(), e.owner.as_ref(), &bump];
    token::transfer_checked(
        CpiContext::new_with_signer(
            token::ID,
            TransferChecked {
                from: source.to_account_info(),
                to: destination.to_account_info(),
                authority: e.to_account_info(),
                mint: mint.to_account_info(),
            },
            &[seeds],
        ),
        amount,
        mint.decimals,
    )
}

#[derive(Accounts)]
pub struct RegisterVenueRoute<'info> {
    #[account(seeds = [b"cinder_vault", config.domain.as_ref(), config.pool.as_ref(), config.mint.as_ref()], bump = config.bump, has_one = governance)]
    pub config: Box<Account<'info, VaultConfig>>,
    #[account(mut)]
    pub governance: Signer<'info>,
    #[account(init, payer = governance, space = 8 + VenueRoute::INIT_SPACE, seeds = [b"venue_route", config.key().as_ref()], bump)]
    pub route: Box<Account<'info, VenueRoute>>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct OpenVenueEscrow<'info> {
    #[account(seeds = [b"cinder_vault", config.domain.as_ref(), config.pool.as_ref(), config.mint.as_ref()], bump = config.bump, has_one = mint)]
    pub config: Box<Account<'info, VaultConfig>>,
    #[account(seeds = [b"venue_route", config.key().as_ref()], bump = route.bump, has_one = config,
        constraint = route.schema == SCHEMA && route.native_account == config.broker @ VaultError::Domain)]
    pub route: Box<Account<'info, VenueRoute>>,
    #[account(seeds = [b"customer", config.key().as_ref(), owner.key().as_ref()], bump = customer.bump, has_one = config, has_one = owner)]
    pub customer: Box<Account<'info, CustomerCounter>>,
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(init, payer = owner, space = 8 + VenueEscrow::INIT_SPACE, seeds = [b"venue_escrow", route.key().as_ref(), owner.key().as_ref()], bump)]
    pub escrow: Box<Account<'info, VenueEscrow>>,
    #[account(init, payer = owner, seeds = [b"escrow_tokens", escrow.key().as_ref()], bump,
        token::mint = mint, token::authority = escrow, token::token_program = token_program)]
    pub escrow_tokens: Box<Account<'info, TokenAccount>>,
    pub mint: Box<Account<'info, Mint>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
#[instruction(auth: Movement)]
pub struct DepositEscrow<'info> {
    #[account(seeds = [b"cinder_vault", config.domain.as_ref(), config.pool.as_ref(), config.mint.as_ref()], bump = config.bump, has_one = mint)]
    pub config: Box<Account<'info, VaultConfig>>,
    #[account(seeds = [b"venue_route", config.key().as_ref()], bump = route.bump, has_one = config,
        constraint = route.schema == SCHEMA && route.native_account == config.broker @ VaultError::Domain)]
    pub route: Box<Account<'info, VenueRoute>>,
    #[account(seeds = [b"customer", config.key().as_ref(), owner.key().as_ref()], bump = customer.bump, has_one = config, has_one = owner)]
    pub customer: Box<Account<'info, CustomerCounter>>,
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut, seeds = [b"venue_escrow", route.key().as_ref(), owner.key().as_ref()], bump = escrow.bump, has_one = route, has_one = owner,
        constraint = escrow.schema == SCHEMA @ VaultError::Domain)]
    pub escrow: Box<Account<'info, VenueEscrow>>,
    #[account(mut, seeds = [b"escrow_tokens", escrow.key().as_ref()], bump = escrow.tokens_bump,
        token::mint = mint, token::authority = escrow, token::token_program = token_program)]
    pub escrow_tokens: Box<Account<'info, TokenAccount>>,
    #[account(mut, token::mint = mint, token::authority = owner, token::token_program = token_program)]
    pub source: Box<Account<'info, TokenAccount>>,
    pub mint: Box<Account<'info, Mint>>,
    #[account(init, payer = owner, space = 8 + EscrowBatch::INIT_SPACE,
        seeds = [b"escrow_batch", escrow.key().as_ref(), auth.operation.as_ref()], bump)]
    pub batch: Box<Account<'info, EscrowBatch>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
#[instruction(auth: Movement)]
pub struct SweepEscrow<'info> {
    #[account(mut, seeds = [b"cinder_vault", config.domain.as_ref(), config.pool.as_ref(), config.mint.as_ref()], bump = config.bump, has_one = mint, has_one = funds, has_one = broker_tokens)]
    pub config: Box<Account<'info, VaultConfig>>,
    #[account(seeds = [b"venue_route", config.key().as_ref()], bump = route.bump, has_one = config,
        constraint = route.schema == SCHEMA && route.native_account == config.broker @ VaultError::Domain)]
    pub route: Box<Account<'info, VenueRoute>>,
    #[account(mut)]
    pub funds: Signer<'info>,
    #[account(mut, seeds = [b"venue_escrow", route.key().as_ref(), escrow.owner.as_ref()], bump = escrow.bump, has_one = route,
        constraint = escrow.schema == SCHEMA @ VaultError::Domain)]
    pub escrow: Box<Account<'info, VenueEscrow>>,
    #[account(mut, seeds = [b"customer", config.key().as_ref(), escrow.owner.as_ref()], bump = customer.bump, has_one = config,
        constraint = customer.owner == escrow.owner @ VaultError::Authority)]
    pub customer: Box<Account<'info, CustomerCounter>>,
    #[account(mut, seeds = [b"escrow_batch", escrow.key().as_ref(), batch.operation.as_ref()], bump = batch.bump, has_one = escrow,
        constraint = batch.schema == SCHEMA @ VaultError::Domain)]
    pub batch: Box<Account<'info, EscrowBatch>>,
    #[account(mut, seeds = [b"escrow_tokens", escrow.key().as_ref()], bump = escrow.tokens_bump,
        token::mint = mint, token::authority = escrow, token::token_program = token_program)]
    pub escrow_tokens: Box<Account<'info, TokenAccount>>,
    #[account(mut, token::mint = mint, constraint = broker_tokens.owner == config.broker @ VaultError::Authority, token::token_program = token_program)]
    pub broker_tokens: Box<Account<'info, TokenAccount>>,
    pub mint: Box<Account<'info, Mint>>,
    #[account(init, payer = funds, space = 8 + MovementReceipt::INIT_SPACE,
        seeds = [b"receipt", config.key().as_ref(), auth.operation.as_ref()], bump)]
    pub receipt: Box<Account<'info, MovementReceipt>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct RefundEscrow<'info> {
    #[account(seeds = [b"cinder_vault", config.domain.as_ref(), config.pool.as_ref(), config.mint.as_ref()], bump = config.bump, has_one = mint)]
    pub config: Box<Account<'info, VaultConfig>>,
    #[account(seeds = [b"venue_route", config.key().as_ref()], bump = route.bump, has_one = config,
        constraint = route.schema == SCHEMA && route.native_account == config.broker @ VaultError::Domain)]
    pub route: Box<Account<'info, VenueRoute>>,
    pub owner: Signer<'info>,
    #[account(mut, seeds = [b"venue_escrow", route.key().as_ref(), owner.key().as_ref()], bump = escrow.bump, has_one = route, has_one = owner,
        constraint = escrow.schema == SCHEMA @ VaultError::Domain)]
    pub escrow: Box<Account<'info, VenueEscrow>>,
    #[account(mut, seeds = [b"escrow_batch", escrow.key().as_ref(), batch.operation.as_ref()], bump = batch.bump, has_one = escrow,
        constraint = batch.schema == SCHEMA @ VaultError::Domain)]
    pub batch: Box<Account<'info, EscrowBatch>>,
    #[account(mut, seeds = [b"escrow_tokens", escrow.key().as_ref()], bump = escrow.tokens_bump,
        token::mint = mint, token::authority = escrow, token::token_program = token_program)]
    pub escrow_tokens: Box<Account<'info, TokenAccount>>,
    #[account(mut, token::mint = mint, token::authority = owner, token::token_program = token_program)]
    pub destination: Box<Account<'info, TokenAccount>>,
    pub mint: Box<Account<'info, Mint>>,
    pub token_program: Program<'info, Token>,
}
#[account]
#[derive(InitSpace)]
pub struct VenueRoute {
    pub schema: u8,
    pub bump: u8,
    pub config: Pubkey,
    pub venue: [u8; 32],
    pub native_account: Pubkey,
}
#[account]
#[derive(InitSpace)]
pub struct VenueEscrow {
    pub schema: u8,
    pub bump: u8,
    pub tokens_bump: u8,
    pub route: Pubkey,
    pub owner: Pubkey,
    /// Exact outstanding batches only; donations never increase this obligation.
    pub pending: u64,
}
#[account]
#[derive(InitSpace)]
pub struct EscrowBatch {
    pub schema: u8,
    pub bump: u8,
    pub state: u8,
    pub escrow: Pubkey,
    pub operation: [u8; 32],
    pub amount: u64,
    pub epoch: u64,
    pub expires_at_slot: u64,
    pub terminal_operation: [u8; 32],
    pub destination: Pubkey,
    pub sequence: u64,
    pub terminal_slot: u64,
}
impl EscrowBatch {
    fn finish(
        &mut self,
        state: u8,
        operation: [u8; 32],
        destination: Pubkey,
        sequence: u64,
    ) -> Result<()> {
        self.state = state;
        self.terminal_operation = operation;
        self.destination = destination;
        self.sequence = sequence;
        self.terminal_slot = Clock::get()?.slot;
        Ok(())
    }
}
