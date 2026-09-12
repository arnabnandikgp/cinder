use anchor_lang::prelude::*;
use cinder_common as cc;

declare_id!("h3Bw2xjj69JssRkaxr8Jxh6TtamvrjSxASXbfLeWyPg");

/// `cinder_vault` program id. Ledger adapter ixs authenticate against that Config.
pub const VAULT_PROGRAM_ID: Pubkey = pubkey!("9zhBFVgk13gnYT6iVuKPGfQiAvVfr6cYQq2bY2QUzXmg");

#[program]
pub mod cinder_ledger {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let cfg = load_vault_config(&ctx.accounts.config)?;
        require_keys_eq!(cfg.adapter, ctx.accounts.adapter.key(), LedgerError::Unauthorized);

        let book = &mut ctx.accounts.book;
        book.residual_len = 0;
        book.residuals = [Residual::default(); cc::MAX_BOOK_MARKETS];
        book.phoenix_collateral = 0;
        book.last_ack_slot_er = 0;
        book.invariant_ok = 1;
        book.halt = 0;
        book.bump = ctx.bumps.book;

        let fees = &mut ctx.accounts.fee_accrual;
        fees.phoenix_fees_paid = 0;
        fees.cinder_fees_accrued = 0;
        fees.bump = ctx.bumps.fee_accrual;
        Ok(())
    }

    pub fn init_user(ctx: Context<InitUser>) -> Result<()> {
        let cfg = load_vault_config(&ctx.accounts.config)?;
        require_keys_eq!(cfg.adapter, ctx.accounts.adapter.key(), LedgerError::Unauthorized);

        let ledger = &mut ctx.accounts.user_ledger;
        ledger.user = ctx.accounts.user.key();
        ledger.free = 0;
        ledger.reserved = 0;
        ledger.withdrawable = 0;
        ledger.pending_oid_count = 0;
        ledger.nonce = 0;
        ledger.last_funding_epoch = 0;
        ledger.positions_len = 0;
        ledger.positions = [Position::default(); cc::MAX_USER_POSITIONS];
        ledger.open_oids = [OpenOid::default(); cc::MAX_OPEN_OIDS_PER_USER];
        ledger.bump = ctx.bumps.user_ledger;
        Ok(())
    }

    pub fn credit_deposit(ctx: Context<CreditDeposit>, amount: u64) -> Result<()> {
        let cfg = load_vault_config(&ctx.accounts.config)?;
        require_keys_eq!(cfg.adapter, ctx.accounts.adapter.key(), LedgerError::Unauthorized);
        require!(!cc::deposit_blocked(cfg.paused), LedgerError::Halted);
        require!(!cc::deposit_blocked(ctx.accounts.book.halt), LedgerError::Halted);
        require!(amount > 0, LedgerError::ZeroAmount);

        let ledger = &mut ctx.accounts.user_ledger;
        ledger.free = ledger
            .free
            .checked_add(amount)
            .ok_or(LedgerError::Overflow)?;
        Ok(())
    }

    pub fn place_order(
        ctx: Context<PlaceOrder>,
        asset_id: u16,
        lots_delta: i64,
        client_oid: [u8; 16],
        max_slippage_bps: u16,
        reduce_only: bool,
        nonce: u64,
    ) -> Result<()> {
        require!(max_slippage_bps <= cc::BPS_DENOM as u16, LedgerError::BadSlippage);
        require!(lots_delta != 0, LedgerError::ZeroLots);

        let cfg = load_vault_config(&ctx.accounts.config)?;
        require!(!cc::entries_blocked(cfg.paused), LedgerError::Halted);
        require!(!cc::entries_blocked(ctx.accounts.book.halt), LedgerError::Halted);
        require!(cfg.is_allowlisted(asset_id), LedgerError::AssetNotAllowlisted);

        let ledger = &mut ctx.accounts.user_ledger;
        require_keys_eq!(ledger.user, ctx.accounts.user.key(), LedgerError::Unauthorized);
        require!(nonce == ledger.nonce, LedgerError::BadNonce);
        ledger.nonce = ledger
            .nonce
            .checked_add(1)
            .ok_or(LedgerError::Overflow)?;

        require!(
            (ledger.pending_oid_count as usize) < cc::MAX_OPEN_OIDS_PER_USER,
            LedgerError::OidCap
        );
        require!(!oid_duplicate(ledger, &client_oid), LedgerError::DuplicateOid);
        let slot = find_free_oid_slot(ledger).ok_or(LedgerError::OidCap)?;

        let old_lots = position_lots(ledger, asset_id);
        let new_lots = old_lots
            .checked_add(lots_delta)
            .ok_or(LedgerError::Overflow)?;

        if reduce_only {
            require!(old_lots != 0, LedgerError::ReduceOnlyIncrease);
            let reduces = new_lots == 0
                || (old_lots.signum() == new_lots.signum() && new_lots.abs() < old_lots.abs());
            require!(reduces, LedgerError::ReduceOnlyIncrease);
        }

        let old_im = stub_im(old_lots)?;
        let new_im = stub_im(new_lots)?;
        apply_im_delta(ledger, old_im, new_im)?;
        set_position_lots(ledger, asset_id, new_lots, new_im)?;

        let equity = ledger
            .free
            .checked_add(ledger.reserved)
            .ok_or(LedgerError::Overflow)?;
        let notional = cc::stub_notional(new_lots.unsigned_abs()).ok_or(LedgerError::Overflow)?;
        if equity > 0 {
            // leverage = notional / equity; reject if notional > equity * MAX_USER_LEVERAGE
            let cap = equity
                .checked_mul(cfg.max_user_leverage as u64)
                .ok_or(LedgerError::Overflow)?;
            require!(notional <= cap, LedgerError::LeverageCap);
        } else {
            require!(notional == 0, LedgerError::LeverageCap);
        }

        ledger.open_oids[slot] = OpenOid {
            client_oid,
            asset_id,
            lots_delta,
            state: cc::OID_PENDING,
        };
        ledger.pending_oid_count = ledger
            .pending_oid_count
            .checked_add(1)
            .ok_or(LedgerError::Overflow)?;
        Ok(())
    }

    pub fn ack_phoenix_fill(
        ctx: Context<AckFill>,
        client_oid: [u8; 16],
        filled_lots: i64,
        fee_usdc: u64,
        vwap_quote_lots: i64,
    ) -> Result<()> {
        let cfg = load_vault_config(&ctx.accounts.config)?;
        require_keys_eq!(cfg.adapter, ctx.accounts.adapter.key(), LedgerError::Unauthorized);
        require!(filled_lots != 0, LedgerError::ZeroLots);

        let ledger = &mut ctx.accounts.user_ledger;
        let (idx, oid) = take_pending_oid(ledger, &client_oid)?;
        require!(
            filled_lots.signum() == oid.lots_delta.signum()
                && filled_lots.abs() <= oid.lots_delta.abs(),
            LedgerError::BadFillSize
        );

        let unfilled = oid
            .lots_delta
            .checked_sub(filled_lots)
            .ok_or(LedgerError::Overflow)?;
        // Tentative lots already include lots_delta. Walk back the unfilled remainder.
        let tentative = position_lots(ledger, oid.asset_id);
        let final_lots = tentative
            .checked_sub(unfilled)
            .ok_or(LedgerError::Overflow)?;
        let old_im = stub_im(tentative)?;
        let new_im = stub_im(final_lots)?;
        apply_im_delta(ledger, old_im, new_im)?;
        add_entry_quote(ledger, oid.asset_id, final_lots, new_im, vwap_quote_lots)?;

        if fee_usdc > 0 {
            require!(ledger.free >= fee_usdc, LedgerError::InsufficientFree);
            ledger.free -= fee_usdc;
            ctx.accounts.fee_accrual.phoenix_fees_paid = ctx
                .accounts
                .fee_accrual
                .phoenix_fees_paid
                .checked_add(fee_usdc)
                .ok_or(LedgerError::Overflow)?;
        }

        ledger.open_oids[idx].state = cc::OID_ACKED;
        ledger.open_oids[idx].lots_delta = filled_lots;
        ledger.pending_oid_count = ledger
            .pending_oid_count
            .checked_sub(1)
            .ok_or(LedgerError::Overflow)?;

        let book = &mut ctx.accounts.book;
        add_book_lots(book, oid.asset_id, filled_lots)?;
        book.last_ack_slot_er = Clock::get()?.slot;
        Ok(())
    }

    pub fn ack_phoenix_fail(ctx: Context<AckFail>, client_oid: [u8; 16]) -> Result<()> {
        let cfg = load_vault_config(&ctx.accounts.config)?;
        require_keys_eq!(cfg.adapter, ctx.accounts.adapter.key(), LedgerError::Unauthorized);

        let ledger = &mut ctx.accounts.user_ledger;
        let (idx, oid) = take_pending_oid(ledger, &client_oid)?;

        let tentative = position_lots(ledger, oid.asset_id);
        let restored = tentative
            .checked_sub(oid.lots_delta)
            .ok_or(LedgerError::Overflow)?;
        let old_im = stub_im(tentative)?;
        let new_im = stub_im(restored)?;
        apply_im_delta(ledger, old_im, new_im)?;
        set_position_lots(ledger, oid.asset_id, restored, new_im)?;

        ledger.open_oids[idx].state = cc::OID_FAILED;
        ledger.pending_oid_count = ledger
            .pending_oid_count
            .checked_sub(1)
            .ok_or(LedgerError::Overflow)?;
        Ok(())
    }

    pub fn request_withdraw(ctx: Context<RequestWithdraw>, amount: u64) -> Result<()> {
        require!(amount > 0, LedgerError::ZeroAmount);
        let cfg = load_vault_config(&ctx.accounts.config)?;
        require!(!cc::withdraw_blocked(cfg.paused), LedgerError::Halted);
        require!(!cc::withdraw_blocked(ctx.accounts.book.halt), LedgerError::Halted);

        let ledger = &mut ctx.accounts.user_ledger;
        require_keys_eq!(ledger.user, ctx.accounts.user.key(), LedgerError::Unauthorized);
        require!(ledger.pending_oid_count == 0, LedgerError::NotFlat);
        require!(is_flat(ledger), LedgerError::NotFlat);
        require!(ledger.free >= amount, LedgerError::InsufficientFree);

        ledger.free -= amount;
        ledger.withdrawable = ledger
            .withdrawable
            .checked_add(amount)
            .ok_or(LedgerError::Overflow)?;
        Ok(())
    }

    pub fn complete_withdraw(ctx: Context<CompleteWithdraw>, amount: u64) -> Result<()> {
        let cfg = load_vault_config(&ctx.accounts.config)?;
        require_keys_eq!(cfg.adapter, ctx.accounts.adapter.key(), LedgerError::Unauthorized);
        require!(amount > 0, LedgerError::ZeroAmount);

        let ledger = &mut ctx.accounts.user_ledger;
        require!(
            ledger.withdrawable >= amount,
            LedgerError::InsufficientWithdrawable
        );
        ledger.withdrawable -= amount;
        Ok(())
    }

    pub fn set_book_halt(ctx: Context<AdapterBook>, halt: u8) -> Result<()> {
        let cfg = load_vault_config(&ctx.accounts.config)?;
        require_keys_eq!(cfg.adapter, ctx.accounts.adapter.key(), LedgerError::Unauthorized);
        ctx.accounts.book.halt = halt;
        Ok(())
    }

    pub fn update_book_collateral(ctx: Context<AdapterBook>, phoenix_collateral: u64) -> Result<()> {
        let cfg = load_vault_config(&ctx.accounts.config)?;
        require_keys_eq!(cfg.adapter, ctx.accounts.adapter.key(), LedgerError::Unauthorized);
        ctx.accounts.book.phoenix_collateral = phoenix_collateral;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub adapter: Signer<'info>,
    /// CHECK: `cinder_vault` Config PDA. Owner + discriminator verified in handler.
    #[account(
        seeds = [cc::SEED_CONFIG],
        bump,
        seeds::program = VAULT_PROGRAM_ID,
        owner = VAULT_PROGRAM_ID
    )]
    pub config: UncheckedAccount<'info>,
    #[account(
        init,
        payer = adapter,
        space = 8 + Book::INIT_SPACE,
        seeds = [cc::SEED_BOOK],
        bump
    )]
    pub book: Box<Account<'info, Book>>,
    #[account(
        init,
        payer = adapter,
        space = 8 + FeeAccrual::INIT_SPACE,
        seeds = [cc::SEED_FEES],
        bump
    )]
    pub fee_accrual: Box<Account<'info, FeeAccrual>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitUser<'info> {
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
    #[account(
        init,
        payer = adapter,
        space = 8 + UserLedger::INIT_SPACE,
        seeds = [cc::SEED_USER, user.key().as_ref()],
        bump
    )]
    pub user_ledger: Box<Account<'info, UserLedger>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreditDeposit<'info> {
    pub adapter: Signer<'info>,
    /// CHECK: vault Config
    #[account(
        seeds = [cc::SEED_CONFIG],
        bump,
        seeds::program = VAULT_PROGRAM_ID,
        owner = VAULT_PROGRAM_ID
    )]
    pub config: UncheckedAccount<'info>,
    #[account(seeds = [cc::SEED_BOOK], bump = book.bump)]
    pub book: Box<Account<'info, Book>>,
    #[account(
        mut,
        seeds = [cc::SEED_USER, user_ledger.user.as_ref()],
        bump = user_ledger.bump
    )]
    pub user_ledger: Box<Account<'info, UserLedger>>,
}

#[derive(Accounts)]
pub struct PlaceOrder<'info> {
    pub user: Signer<'info>,
    /// CHECK: vault Config (halt + allowlist)
    #[account(
        seeds = [cc::SEED_CONFIG],
        bump,
        seeds::program = VAULT_PROGRAM_ID,
        owner = VAULT_PROGRAM_ID
    )]
    pub config: UncheckedAccount<'info>,
    #[account(seeds = [cc::SEED_BOOK], bump = book.bump)]
    pub book: Box<Account<'info, Book>>,
    #[account(
        mut,
        seeds = [cc::SEED_USER, user.key().as_ref()],
        bump = user_ledger.bump,
        has_one = user @ LedgerError::Unauthorized
    )]
    pub user_ledger: Box<Account<'info, UserLedger>>,
}

#[derive(Accounts)]
pub struct AckFill<'info> {
    pub adapter: Signer<'info>,
    /// CHECK: vault Config
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
    #[account(mut, seeds = [cc::SEED_BOOK], bump = book.bump)]
    pub book: Box<Account<'info, Book>>,
    #[account(mut, seeds = [cc::SEED_FEES], bump = fee_accrual.bump)]
    pub fee_accrual: Box<Account<'info, FeeAccrual>>,
}

#[derive(Accounts)]
pub struct AckFail<'info> {
    pub adapter: Signer<'info>,
    /// CHECK: vault Config
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
}

#[derive(Accounts)]
pub struct RequestWithdraw<'info> {
    pub user: Signer<'info>,
    /// CHECK: vault Config
    #[account(
        seeds = [cc::SEED_CONFIG],
        bump,
        seeds::program = VAULT_PROGRAM_ID,
        owner = VAULT_PROGRAM_ID
    )]
    pub config: UncheckedAccount<'info>,
    #[account(seeds = [cc::SEED_BOOK], bump = book.bump)]
    pub book: Box<Account<'info, Book>>,
    #[account(
        mut,
        seeds = [cc::SEED_USER, user.key().as_ref()],
        bump = user_ledger.bump,
        has_one = user @ LedgerError::Unauthorized
    )]
    pub user_ledger: Box<Account<'info, UserLedger>>,
}

#[derive(Accounts)]
pub struct CompleteWithdraw<'info> {
    pub adapter: Signer<'info>,
    /// CHECK: vault Config
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
}

#[derive(Accounts)]
pub struct AdapterBook<'info> {
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
}

#[account]
#[derive(InitSpace)]
pub struct UserLedger {
    pub user: Pubkey,
    pub free: u64,
    pub reserved: u64,
    pub withdrawable: u64,
    pub pending_oid_count: u8,
    pub nonce: u64,
    pub last_funding_epoch: u64,
    pub positions_len: u8,
    pub positions: [Position; 16],
    pub open_oids: [OpenOid; 8],
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Book {
    pub residual_len: u8,
    pub residuals: [Residual; 32],
    pub phoenix_collateral: u64,
    pub last_ack_slot_er: u64,
    pub invariant_ok: u8,
    pub halt: u8,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct FeeAccrual {
    pub phoenix_fees_paid: u64,
    pub cinder_fees_accrued: u64,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, InitSpace)]
pub struct Position {
    pub asset_id: u16,
    pub lots: i64,
    pub entry_quote_lots: i64,
    pub unsettled_funding: i64,
    pub reserved_im: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, InitSpace)]
pub struct OpenOid {
    pub client_oid: [u8; 16],
    pub asset_id: u16,
    pub lots_delta: i64,
    pub state: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, InitSpace)]
pub struct Residual {
    pub asset_id: u16,
    pub lots: i64,
}

#[allow(dead_code)]
#[derive(AnchorDeserialize, Clone)]
struct VaultConfig {
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

impl VaultConfig {
    fn is_allowlisted(&self, asset_id: u16) -> bool {
        self.allowlist_assets
            .iter()
            .take(self.allowlist_len as usize)
            .any(|a| *a == asset_id)
    }
}

/// sha256("account:Config")[0..8] — Anchor account discriminator.
const CONFIG_DISCRIMINATOR: [u8; 8] = [155, 12, 170, 224, 30, 250, 204, 130];

fn load_vault_config(info: &AccountInfo) -> Result<VaultConfig> {
    require_keys_eq!(*info.owner, VAULT_PROGRAM_ID, LedgerError::InvalidConfig);
    let data = info.try_borrow_data()?;
    require!(data.len() >= 8, LedgerError::InvalidConfig);
    require!(data[0..8] == CONFIG_DISCRIMINATOR, LedgerError::InvalidConfig);
    VaultConfig::try_from_slice(&data[8..]).map_err(|_| error!(LedgerError::InvalidConfig))
}

fn stub_im(lots: i64) -> Result<u64> {
    cc::stub_cinder_im(lots.unsigned_abs()).ok_or(error!(LedgerError::Overflow))
}

fn apply_im_delta(ledger: &mut UserLedger, old_im: u64, new_im: u64) -> Result<()> {
    if new_im > old_im {
        let delta = new_im - old_im;
        require!(ledger.free >= delta, LedgerError::InsufficientFree);
        ledger.free -= delta;
        ledger.reserved = ledger
            .reserved
            .checked_add(delta)
            .ok_or(LedgerError::Overflow)?;
    } else if new_im < old_im {
        let delta = old_im - new_im;
        require!(ledger.reserved >= delta, LedgerError::InsufficientFree);
        ledger.reserved -= delta;
        ledger.free = ledger
            .free
            .checked_add(delta)
            .ok_or(LedgerError::Overflow)?;
    }
    Ok(())
}

fn position_lots(ledger: &UserLedger, asset_id: u16) -> i64 {
    ledger.positions[..ledger.positions_len as usize]
        .iter()
        .find(|p| p.asset_id == asset_id)
        .map(|p| p.lots)
        .unwrap_or(0)
}

fn set_position_lots(
    ledger: &mut UserLedger,
    asset_id: u16,
    lots: i64,
    reserved_im: u64,
) -> Result<()> {
    if let Some(pos) = ledger.positions[..ledger.positions_len as usize]
        .iter_mut()
        .find(|p| p.asset_id == asset_id)
    {
        pos.lots = lots;
        pos.reserved_im = reserved_im;
        if lots == 0 {
            pos.entry_quote_lots = 0;
            pos.unsettled_funding = 0;
            pos.reserved_im = 0;
            compact_positions(ledger);
        }
        return Ok(());
    }
    if lots == 0 {
        return Ok(());
    }
    require!(
        (ledger.positions_len as usize) < cc::MAX_USER_POSITIONS,
        LedgerError::PositionCap
    );
    let i = ledger.positions_len as usize;
    ledger.positions[i] = Position {
        asset_id,
        lots,
        entry_quote_lots: 0,
        unsettled_funding: 0,
        reserved_im,
    };
    ledger.positions_len += 1;
    Ok(())
}

fn add_entry_quote(
    ledger: &mut UserLedger,
    asset_id: u16,
    lots: i64,
    reserved_im: u64,
    vwap_quote_lots: i64,
) -> Result<()> {
    set_position_lots(ledger, asset_id, lots, reserved_im)?;
    if let Some(pos) = ledger.positions[..ledger.positions_len as usize]
        .iter_mut()
        .find(|p| p.asset_id == asset_id)
    {
        pos.entry_quote_lots = pos
            .entry_quote_lots
            .checked_add(vwap_quote_lots)
            .ok_or(LedgerError::Overflow)?;
    }
    Ok(())
}

fn compact_positions(ledger: &mut UserLedger) {
    let mut w = 0usize;
    let n = ledger.positions_len as usize;
    for r in 0..n {
        if ledger.positions[r].lots != 0 {
            if w != r {
                ledger.positions[w] = ledger.positions[r];
            }
            w += 1;
        }
    }
    for i in w..n {
        ledger.positions[i] = Position::default();
    }
    ledger.positions_len = w as u8;
}

fn is_flat(ledger: &UserLedger) -> bool {
    ledger.positions[..ledger.positions_len as usize]
        .iter()
        .all(|p| p.lots == 0)
}

fn oid_slot_free(oid: &OpenOid) -> bool {
    match oid.state {
        cc::OID_PENDING => oid.lots_delta == 0,
        cc::OID_ACKED | cc::OID_FAILED => true,
        cc::OID_LIQUIDATING => false,
        _ => true,
    }
}

fn oid_duplicate(ledger: &UserLedger, client_oid: &[u8; 16]) -> bool {
    ledger.open_oids.iter().any(|o| {
        !oid_slot_free(o) && o.client_oid == *client_oid
    })
}

fn find_free_oid_slot(ledger: &UserLedger) -> Option<usize> {
    ledger.open_oids.iter().position(oid_slot_free)
}

fn take_pending_oid(ledger: &UserLedger, client_oid: &[u8; 16]) -> Result<(usize, OpenOid)> {
    let idx = ledger
        .open_oids
        .iter()
        .position(|o| o.client_oid == *client_oid && o.state == cc::OID_PENDING && o.lots_delta != 0)
        .ok_or(LedgerError::OidNotFound)?;
    Ok((idx, ledger.open_oids[idx]))
}

fn add_book_lots(book: &mut Book, asset_id: u16, filled_lots: i64) -> Result<()> {
    if let Some(r) = book.residuals[..book.residual_len as usize]
        .iter_mut()
        .find(|r| r.asset_id == asset_id)
    {
        r.lots = r.lots.checked_add(filled_lots).ok_or(LedgerError::Overflow)?;
        if r.lots == 0 {
            compact_residuals(book);
        }
        return Ok(());
    }
    if filled_lots == 0 {
        return Ok(());
    }
    require!(
        (book.residual_len as usize) < cc::MAX_BOOK_MARKETS,
        LedgerError::PositionCap
    );
    let i = book.residual_len as usize;
    book.residuals[i] = Residual {
        asset_id,
        lots: filled_lots,
    };
    book.residual_len += 1;
    Ok(())
}

fn compact_residuals(book: &mut Book) {
    let mut w = 0usize;
    let n = book.residual_len as usize;
    for r in 0..n {
        if book.residuals[r].lots != 0 {
            if w != r {
                book.residuals[w] = book.residuals[r];
            }
            w += 1;
        }
    }
    for i in w..n {
        book.residuals[i] = Residual::default();
    }
    book.residual_len = w as u8;
}

#[error_code]
pub enum LedgerError {
    #[msg("unauthorized")]
    Unauthorized,
    #[msg("halted")]
    Halted,
    #[msg("asset not allowlisted")]
    AssetNotAllowlisted,
    #[msg("insufficient free collateral")]
    InsufficientFree,
    #[msg("nonce mismatch")]
    BadNonce,
    #[msg("too many open oids")]
    OidCap,
    #[msg("duplicate client oid")]
    DuplicateOid,
    #[msg("oid not found or not pending")]
    OidNotFound,
    #[msg("too many positions")]
    PositionCap,
    #[msg("reduce-only would increase exposure")]
    ReduceOnlyIncrease,
    #[msg("user is not flat")]
    NotFlat,
    #[msg("insufficient withdrawable")]
    InsufficientWithdrawable,
    #[msg("zero lots")]
    ZeroLots,
    #[msg("zero amount")]
    ZeroAmount,
    #[msg("leverage cap exceeded")]
    LeverageCap,
    #[msg("overflow")]
    Overflow,
    #[msg("invalid vault config")]
    InvalidConfig,
    #[msg("bad fill size")]
    BadFillSize,
    #[msg("bad slippage bps")]
    BadSlippage,
}
