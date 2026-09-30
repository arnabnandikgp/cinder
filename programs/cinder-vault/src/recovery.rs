//! Operator-assisted final recovery. Membership is not a proof of complete liabilities.
use super::*;
use solana_sha256_hasher::hashv;

pub const STAGED: u8 = 2;
pub const ACTIVE: u8 = 3;
pub const CLOSED: u8 = 4;
pub const MAX_RECOVERY_LEAVES: u32 = 65_536;
pub const MAX_RECOVERY_PROOF: usize = 16;

/// Publisher assertions must be independently qualified by the activating operator.
/// The program checks their shape and rejects explicitly unresolved state; these are
/// NOT an on-chain proof of external venue fencing, completeness or private accounting.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct RecoveryQualification {
    pub unresolved_operations: u64,
    pub outstanding_reservations: u64,
    pub unresolved_inputs: u64,
    pub venue_exposure_zero: bool,
    pub claims_available: bool,
}
impl RecoveryQualification {
    fn valid(&self) -> Result<()> {
        require!(
            self.unresolved_operations == 0
                && self.outstanding_reservations == 0
                && self.unresolved_inputs == 0
                && self.venue_exposure_zero
                && self.claims_available,
            VaultError::RecoveryQualification
        );
        Ok(())
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct RecoveryStatement {
    pub domain: [u8; 32],
    pub epoch: u64,
    pub root: [u8; 32],
    pub tree_size: u32,
    /// Sum of final unpaid claims, already net of ordinary payouts and settled losses.
    pub total: u64,
    pub journal_cutoff: u64,
    pub journal_hash: [u8; 32],
    /// Binds reconciliation, venue fencing/settlement, backing and claim-delivery evidence.
    pub evidence_hash: [u8; 32],
    pub policy_hash: [u8; 32],
    pub normal_paid: u64,
    pub funding_sequence: u64,
    pub qualification: RecoveryQualification,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct RecoveryClaim {
    pub index: u32,
    pub amount: u64,
    pub paid_base: u64,
    pub payout_sequence_base: u64,
    pub claim_id: [u8; 32],
    pub salt: [u8; 32],
}

pub(crate) fn stage(ctx: Context<StageRecovery>, statement: RecoveryStatement) -> Result<()> {
    let c = &ctx.accounts.config;
    c.bound(statement.domain, statement.epoch)?;
    require!(c.mode == FROZEN, VaultError::Mode);
    statement.qualification.valid()?;
    for digest in [
        statement.root,
        statement.journal_hash,
        statement.evidence_hash,
        statement.policy_hash,
    ] {
        nonzero(&digest)?;
    }
    require!(
        statement.tree_size > 0
            && statement.tree_size <= MAX_RECOVERY_LEAVES
            && statement.total > 0
            && statement.journal_cutoff > 0,
        VaultError::Bound
    );
    require!(
        statement.normal_paid == c.paid && statement.funding_sequence == c.funding_sequence,
        VaultError::Counter
    );
    let r = &mut ctx.accounts.recovery_epoch;
    r.config = c.key();
    r.bump = ctx.bumps.recovery_epoch;
    r.context_hash = context_hash(c, c.key(), &statement);
    r.remaining = statement.total;
    r.statement = statement;
    ctx.accounts.config.mode = STAGED;
    Ok(())
}

pub(crate) fn activate(
    ctx: Context<ActivateRecovery>,
    domain: [u8; 32],
    epoch: u64,
    expected_root: [u8; 32],
) -> Result<()> {
    let c = &ctx.accounts.config;
    let r = &ctx.accounts.recovery_epoch;
    c.bound(domain, epoch)?;
    require!(c.mode == STAGED, VaultError::Mode);
    require!(
        r.statement.epoch == epoch && r.statement.root == expected_root,
        VaultError::RecoveryProof
    );
    r.statement.qualification.valid()?;
    require!(
        c.paid == r.statement.normal_paid && c.funding_sequence == r.statement.funding_sequence,
        VaultError::Counter
    );
    clean(&ctx.accounts.vault)?;
    require!(
        ctx.accounts.vault.amount >= r.remaining
            && ctx.accounts.vault.state == token::spl_token::state::AccountState::Initialized,
        VaultError::RecoveryBacking
    );
    ctx.accounts.config.mode = ACTIVE;
    Ok(())
}

pub(crate) fn claim(
    ctx: Context<ClaimRecovery>,
    domain: [u8; 32],
    epoch: u64,
    claim: RecoveryClaim,
    proof: Vec<[u8; 32]>,
) -> Result<()> {
    let c = &ctx.accounts.config;
    let r = &ctx.accounts.recovery_epoch;
    c.bound(domain, epoch)?;
    require!(c.mode == ACTIVE, VaultError::Mode);
    require!(r.statement.epoch == epoch, VaultError::Domain);
    require!(claim.amount > 0, VaultError::Bound);
    nonzero(&claim.claim_id)?;
    nonzero(&claim.salt)?;
    require!(
        ctx.accounts.customer.paid == claim.paid_base
            && ctx.accounts.customer.payout_sequence == claim.payout_sequence_base,
        VaultError::Counter
    );
    let leaf = leaf_hash(
        r.context_hash,
        ctx.accounts.owner.key(),
        ctx.accounts.destination.key(),
        &claim,
    );
    verify(
        r.statement.root,
        leaf,
        claim.index,
        r.statement.tree_size,
        &proof,
    )?;
    // Containment stops ALL claims after impairment, not only the last claimant.
    require!(
        ctx.accounts.vault.amount >= r.remaining && claim.amount <= r.remaining,
        VaultError::RecoveryBacking
    );
    let remaining = r
        .remaining
        .checked_sub(claim.amount)
        .ok_or_else(|| error!(VaultError::Arithmetic))?;
    let count = r
        .claimed_count
        .checked_add(1)
        .ok_or_else(|| error!(VaultError::Arithmetic))?;
    require!(count <= r.statement.tree_size, VaultError::RecoveryTotal);
    require!(
        count != r.statement.tree_size || remaining == 0,
        VaultError::RecoveryTotal
    );
    let paid = add(claim.paid_base, claim.amount)?;
    let sequence = add(claim.payout_sequence_base, 1)?;
    let total = add(c.paid, claim.amount)?;
    clean(&ctx.accounts.destination)?;
    pay(
        c,
        &ctx.accounts.vault,
        &ctx.accounts.destination,
        &ctx.accounts.mint,
        claim.amount,
    )?;
    ctx.accounts.customer.paid = paid;
    ctx.accounts.customer.payout_sequence = sequence;
    ctx.accounts.config.paid = total;
    ctx.accounts.recovery_epoch.remaining = remaining;
    ctx.accounts.recovery_epoch.claimed_count = count;
    if count == ctx.accounts.recovery_epoch.statement.tree_size {
        ctx.accounts.config.mode = CLOSED;
    }
    let receipt = &mut ctx.accounts.claim_receipt;
    receipt.config = ctx.accounts.config.key();
    receipt.owner = ctx.accounts.owner.key();
    receipt.destination = ctx.accounts.destination.key();
    receipt.epoch = epoch;
    receipt.index = claim.index;
    receipt.claim_id = claim.claim_id;
    receipt.leaf = leaf;
    receipt.amount = claim.amount;
    receipt.paid = paid;
    receipt.payout_sequence = sequence;
    Ok(())
}

fn context_hash(c: &VaultConfig, config: Pubkey, s: &RecoveryStatement) -> [u8; 32] {
    hashv(&[
        b"CINDER_RECOVERY_CONTEXT_V1",
        crate::ID.as_ref(),
        config.as_ref(),
        &c.domain,
        &c.pool,
        c.mint.as_ref(),
        token::ID.as_ref(),
        &[c.decimals],
        &s.epoch.to_le_bytes(),
        &s.tree_size.to_le_bytes(),
        &s.total.to_le_bytes(),
        &s.journal_cutoff.to_le_bytes(),
        &s.journal_hash,
        &s.evidence_hash,
        &s.policy_hash,
        &s.normal_paid.to_le_bytes(),
        &s.funding_sequence.to_le_bytes(),
    ])
    .to_bytes()
}
fn leaf_hash(context: [u8; 32], owner: Pubkey, destination: Pubkey, c: &RecoveryClaim) -> [u8; 32] {
    hashv(&[
        &[0],
        b"CINDER_RECOVERY_CLAIM_V1",
        &context,
        &c.index.to_le_bytes(),
        &c.claim_id,
        owner.as_ref(),
        destination.as_ref(),
        &c.amount.to_le_bytes(),
        &c.paid_base.to_le_bytes(),
        &c.payout_sequence_base.to_le_bytes(),
        &c.salt,
    ])
    .to_bytes()
}

/// RFC 9162 ordered inclusion verification. No sorted pairs or duplicate-last padding.
/// Exact consumption rejects both missing and superfluous siblings, including odd trees.
fn verify(
    root: [u8; 32],
    mut leaf: [u8; 32],
    index: u32,
    size: u32,
    proof: &[[u8; 32]],
) -> Result<()> {
    require!(
        size > 0
            && size <= MAX_RECOVERY_LEAVES
            && index < size
            && proof.len() <= MAX_RECOVERY_PROOF,
        VaultError::RecoveryProof
    );
    let mut f = index;
    let mut s = size - 1;
    for p in proof {
        require!(s != 0, VaultError::RecoveryProof);
        if f & 1 != 0 || f == s {
            leaf = hashv(&[&[1], p, &leaf]).to_bytes();
            while f & 1 == 0 && f != 0 {
                f >>= 1;
                s >>= 1;
            }
        } else {
            leaf = hashv(&[&[1], &leaf, p]).to_bytes();
        }
        f >>= 1;
        s >>= 1;
    }
    require!(s == 0 && leaf == root, VaultError::RecoveryProof);
    Ok(())
}

#[derive(Accounts)]
pub struct StageRecovery<'info> {
    #[account(mut, seeds = [b"cinder_vault", config.domain.as_ref(), config.pool.as_ref(), config.mint.as_ref()], bump = config.bump, has_one = governance)]
    pub config: Box<Account<'info, VaultConfig>>,
    #[account(mut)]
    pub governance: Signer<'info>,
    // One account per config, not per epoch: final roots cannot roll over/reset history.
    #[account(init, payer = governance, space = 8 + RecoveryEpoch::INIT_SPACE,
        seeds = [b"recovery", config.key().as_ref()], bump)]
    pub recovery_epoch: Box<Account<'info, RecoveryEpoch>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ActivateRecovery<'info> {
    #[account(mut, seeds = [b"cinder_vault", config.domain.as_ref(), config.pool.as_ref(), config.mint.as_ref()], bump = config.bump, has_one = recovery)]
    pub config: Box<Account<'info, VaultConfig>>,
    pub recovery: Signer<'info>,
    #[account(seeds = [b"recovery", config.key().as_ref()], bump = recovery_epoch.bump, has_one = config)]
    pub recovery_epoch: Box<Account<'info, RecoveryEpoch>>,
    #[account(seeds = [b"tokens", config.key().as_ref()], bump = config.vault_bump,
        constraint = vault.mint == config.mint @ VaultError::Mint, token::authority = config, token::token_program = token_program)]
    pub vault: Box<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ClaimRecovery<'info> {
    #[account(mut, seeds = [b"cinder_vault", config.domain.as_ref(), config.pool.as_ref(), config.mint.as_ref()], bump = config.bump, has_one = mint)]
    pub config: Box<Account<'info, VaultConfig>>,
    #[account(mut, seeds = [b"recovery", config.key().as_ref()], bump = recovery_epoch.bump, has_one = config)]
    pub recovery_epoch: Box<Account<'info, RecoveryEpoch>>,
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut, seeds = [b"customer", config.key().as_ref(), owner.key().as_ref()], bump = customer.bump, has_one = config, has_one = owner)]
    pub customer: Box<Account<'info, CustomerCounter>>,
    #[account(mut, token::mint = mint, token::authority = owner, token::token_program = token_program)]
    pub destination: Box<Account<'info, TokenAccount>>,
    #[account(mut, seeds = [b"tokens", config.key().as_ref()], bump = config.vault_bump,
        token::mint = mint, token::authority = config, token::token_program = token_program)]
    pub vault: Box<Account<'info, TokenAccount>>,
    pub mint: Box<Account<'info, Mint>>,
    // One immutable receipt per owner/config defeats duplicate-owner leaves even with
    // a maliciously advanced paid baseline. There is no close/reset instruction.
    #[account(init, payer = owner, space = 8 + RecoveryClaimReceipt::INIT_SPACE,
        seeds = [b"recovery_paid", config.key().as_ref(), owner.key().as_ref()], bump)]
    pub claim_receipt: Box<Account<'info, RecoveryClaimReceipt>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[account]
#[derive(InitSpace)]
pub struct RecoveryEpoch {
    pub config: Pubkey,
    pub bump: u8,
    pub statement: RecoveryStatement,
    pub context_hash: [u8; 32],
    pub remaining: u64,
    pub claimed_count: u32,
}

#[account]
#[derive(InitSpace)]
pub struct RecoveryClaimReceipt {
    pub config: Pubkey,
    pub owner: Pubkey,
    pub destination: Pubkey,
    pub epoch: u64,
    pub index: u32,
    pub claim_id: [u8; 32],
    pub leaf: [u8; 32],
    pub amount: u64,
    pub paid: u64,
    pub payout_sequence: u64,
}
