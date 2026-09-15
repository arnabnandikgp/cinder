use anchor_lang::prelude::*;
use cinder_common as cc;
use ephemeral_rollups_sdk::anchor::ephemeral;

mod migration;
mod privacy;
pub use migration::*;
pub use privacy::*;

declare_id!("h3Bw2xjj69JssRkaxr8Jxh6TtamvrjSxASXbfLeWyPg");

/// `cinder_vault` program id. Ledger adapter ixs authenticate against that Config.
pub const VAULT_PROGRAM_ID: Pubkey = pubkey!("9zhBFVgk13gnYT6iVuKPGfQiAvVfr6cYQq2bY2QUzXmg");

#[ephemeral]
#[program]
pub mod cinder_ledger {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let cfg = load_vault_config(&ctx.accounts.config)?;
        require_keys_eq!(
            cfg.adapter,
            ctx.accounts.adapter.key(),
            LedgerError::Unauthorized
        );

        let book = &mut ctx.accounts.book;
        book.schema_version = cc::ACCOUNT_SCHEMA_VERSION;
        book.residual_len = 0;
        book.residuals = [Residual::default(); cc::MAX_BOOK_MARKETS];
        book.phoenix_collateral = 0;
        book.last_ack_slot_er = 0;
        book.invariant_ok = 1;
        book.halt = 0;
        book.funding_epoch = 0;
        book.last_scan_ms = 0;
        book.bump = ctx.bumps.book;

        let fees = &mut ctx.accounts.fee_accrual;
        fees.phoenix_fees_paid = 0;
        fees.cinder_fees_accrued = 0;
        fees.bump = ctx.bumps.fee_accrual;

        privacy::fund_permission_rent(
            ctx.accounts.adapter.to_account_info(),
            ctx.accounts.book.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            cc::MAX_POOL_PERMISSION_MEMBERS,
        )?;
        privacy::fund_permission_rent(
            ctx.accounts.adapter.to_account_info(),
            ctx.accounts.fee_accrual.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            cc::MAX_POOL_PERMISSION_MEMBERS,
        )?;
        Ok(())
    }

    pub fn init_user(ctx: Context<InitUser>) -> Result<()> {
        let cfg = load_vault_config(&ctx.accounts.config)?;
        require_keys_eq!(
            cfg.adapter,
            ctx.accounts.adapter.key(),
            LedgerError::Unauthorized
        );
        require_book_schema(&ctx.accounts.book)?;

        let ledger = &mut ctx.accounts.user_ledger;
        ledger.schema_version = cc::ACCOUNT_SCHEMA_VERSION;
        ledger.user = ctx.accounts.user.key();
        ledger.free = 0;
        ledger.reserved = 0;
        ledger.withdrawable = 0;
        ledger.bad_debt_usdc = 0;
        ledger.pending_oid_count = 0;
        ledger.nonce = 0;
        ledger.last_funding_epoch = ctx.accounts.book.funding_epoch;
        ledger.positions_len = 0;
        ledger.positions = [Position::default(); cc::MAX_USER_POSITIONS];
        ledger.open_oids = [OpenOid::default(); cc::MAX_OPEN_OIDS_PER_USER];
        ledger.bump = ctx.bumps.user_ledger;

        privacy::fund_permission_rent(
            ctx.accounts.adapter.to_account_info(),
            ctx.accounts.user_ledger.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            cc::MAX_USER_PERMISSION_MEMBERS,
        )?;
        Ok(())
    }

    pub fn credit_deposit(ctx: Context<CreditDeposit>, amount: u64) -> Result<()> {
        let cfg = load_vault_config(&ctx.accounts.config)?;
        require_keys_eq!(
            cfg.adapter,
            ctx.accounts.adapter.key(),
            LedgerError::Unauthorized
        );
        require_book_schema(&ctx.accounts.book)?;
        require_user_schema(&ctx.accounts.user_ledger)?;
        require!(!cc::deposit_blocked(cfg.paused), LedgerError::Halted);
        require!(
            !cc::deposit_blocked(ctx.accounts.book.halt),
            LedgerError::Halted
        );
        require!(amount > 0, LedgerError::ZeroAmount);

        let ledger = &mut ctx.accounts.user_ledger;
        let mut free = ledger.free;
        let mut bad_debt = ledger.bad_debt_usdc;
        cc::credit_cash(&mut free, &mut bad_debt, amount).ok_or(LedgerError::Overflow)?;
        ledger.free = free;
        ledger.bad_debt_usdc = bad_debt;
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
        require!(
            max_slippage_bps <= cc::BPS_DENOM as u16,
            LedgerError::BadSlippage
        );
        require!(lots_delta != 0, LedgerError::ZeroLots);

        let cfg = load_vault_config(&ctx.accounts.config)?;
        require_book_schema(&ctx.accounts.book)?;
        require_user_schema(&ctx.accounts.user_ledger)?;
        require!(!cc::entries_blocked(cfg.paused), LedgerError::Halted);
        require!(
            !cc::entries_blocked(ctx.accounts.book.halt),
            LedgerError::Halted
        );
        require!(
            asset_allowlisted(&cfg, asset_id),
            LedgerError::AssetNotAllowlisted
        );

        let ledger = &mut ctx.accounts.user_ledger;
        require_keys_eq!(
            ledger.user,
            ctx.accounts.user.key(),
            LedgerError::Unauthorized
        );
        require!(ledger.bad_debt_usdc == 0, LedgerError::Halted);
        require!(nonce == ledger.nonce, LedgerError::BadNonce);
        ledger.nonce = ledger.nonce.checked_add(1).ok_or(LedgerError::Overflow)?;

        require!(
            (ledger.pending_oid_count as usize) < cc::MAX_OPEN_OIDS_PER_USER,
            LedgerError::OidCap
        );
        require!(
            !oid_duplicate(ledger, &client_oid),
            LedgerError::DuplicateOid
        );
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
        let notional = stub_notional_all(ledger)?;
        if equity > 0 {
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
            limit_price_ticks: 0,
            last_valid_slot: 0,
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
        post_position_im_usdc: u64,
    ) -> Result<()> {
        let cfg = load_vault_config(&ctx.accounts.config)?;
        require_keys_eq!(
            cfg.adapter,
            ctx.accounts.adapter.key(),
            LedgerError::Unauthorized
        );
        require!(filled_lots != 0, LedgerError::ZeroLots);
        require_book_schema(&ctx.accounts.book)?;
        require_user_schema(&ctx.accounts.user_ledger)?;

        let ledger = &mut ctx.accounts.user_ledger;
        let pending = take_pending_oid(ledger, &client_oid);
        let (idx, oid) = match pending {
            Ok(found) => found,
            Err(err) => {
                if let Some(acked) = find_acknowledged_oid(ledger, &client_oid) {
                    require!(acked.lots_delta == filled_lots, LedgerError::BadFillSize);
                    return Ok(());
                }
                return Err(err);
            }
        };
        require!(
            filled_lots.signum() == oid.lots_delta.signum()
                && filled_lots.abs() <= oid.lots_delta.abs(),
            LedgerError::BadFillSize
        );

        let unfilled = oid
            .lots_delta
            .checked_sub(filled_lots)
            .ok_or(LedgerError::Overflow)?;
        // Tentative lots include every pending oid on this asset. Confirmed lots
        // subtract all of them so out-of-order acks use the right basis.
        let tentative = position_lots(ledger, oid.asset_id);
        let lots_before = tentative
            .checked_sub(pending_delta_for_asset(ledger, oid.asset_id)?)
            .ok_or(LedgerError::Overflow)?;
        let final_lots = tentative
            .checked_sub(unfilled)
            .ok_or(LedgerError::Overflow)?;
        let entry_before = position_entry(ledger, oid.asset_id);
        let realized = cc::realize_on_fill(lots_before, filled_lots, entry_before, vwap_quote_lots)
            .ok_or(LedgerError::Overflow)?;
        let prior_position_im = position_reserved_im(ledger, oid.asset_id);
        set_position_lots(ledger, oid.asset_id, final_lots, prior_position_im)?;
        {
            let n = ledger.positions_len as usize;
            if let Some(pos) = ledger.positions[..n]
                .iter_mut()
                .find(|p| p.asset_id == oid.asset_id)
            {
                pos.entry_quote_lots = realized.new_entry_quote;
            }
        }
        apply_signed_cash(ledger, realized.realized_usdc)?;
        if final_lots == 0 {
            {
                let n = ledger.positions_len as usize;
                if let Some(pos) = ledger.positions[..n]
                    .iter_mut()
                    .find(|p| p.asset_id == oid.asset_id)
                {
                    pos.entry_quote_lots = 0;
                }
            }
            compact_positions(ledger);
        }

        if fee_usdc > 0 {
            debit_cash(ledger, fee_usdc)?;
            ctx.accounts.fee_accrual.phoenix_fees_paid = ctx
                .accounts
                .fee_accrual
                .phoenix_fees_paid
                .checked_add(fee_usdc)
                .ok_or(LedgerError::Overflow)?;
        }

        let under_margined =
            rebalance_position_margin(ledger, oid.asset_id, post_position_im_usdc)?;

        ledger.open_oids[idx].state = cc::OID_ACKED;
        ledger.open_oids[idx].lots_delta = filled_lots;
        ledger.pending_oid_count = ledger
            .pending_oid_count
            .checked_sub(1)
            .ok_or(LedgerError::Overflow)?;

        let book = &mut ctx.accounts.book;
        add_book_lots(book, oid.asset_id, filled_lots)?;
        book.last_ack_slot_er = Clock::get()?.slot;
        if under_margined {
            book.halt |= cc::HALT_ENTRIES;
        }
        if ledger.bad_debt_usdc > 0 {
            book.halt |= cc::BAD_DEBT | cc::HALT_ENTRIES | cc::HALT_WITHDRAW;
        }
        emit!(PhoenixFillAcknowledged {
            user: ledger.user,
            client_oid,
            asset_id: oid.asset_id,
            filled_lots,
            realized_usdc: realized.realized_usdc,
            fee_usdc,
            bad_debt_usdc: ledger.bad_debt_usdc,
            under_margined,
        });
        Ok(())
    }

    pub fn ack_phoenix_fail(ctx: Context<AckFail>, client_oid: [u8; 16]) -> Result<()> {
        let cfg = load_vault_config(&ctx.accounts.config)?;
        require_keys_eq!(
            cfg.adapter,
            ctx.accounts.adapter.key(),
            LedgerError::Unauthorized
        );
        require_user_schema(&ctx.accounts.user_ledger)?;

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
        require_book_schema(&ctx.accounts.book)?;
        require_user_schema(&ctx.accounts.user_ledger)?;
        require!(!cc::withdraw_blocked(cfg.paused), LedgerError::Halted);
        require!(
            !cc::withdraw_blocked(ctx.accounts.book.halt),
            LedgerError::Halted
        );

        let ledger = &mut ctx.accounts.user_ledger;
        require_keys_eq!(
            ledger.user,
            ctx.accounts.user.key(),
            LedgerError::Unauthorized
        );
        require!(ledger.bad_debt_usdc == 0, LedgerError::Halted);
        require!(ledger.pending_oid_count == 0, LedgerError::NotFlat);
        require!(is_flat(ledger), LedgerError::NotFlat);
        require!(
            unsettled_funding_zero(ledger),
            LedgerError::UnsettledFunding
        );
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
        require_keys_eq!(
            cfg.adapter,
            ctx.accounts.adapter.key(),
            LedgerError::Unauthorized
        );
        require!(amount > 0, LedgerError::ZeroAmount);
        require_user_schema(&ctx.accounts.user_ledger)?;

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
        require_keys_eq!(
            cfg.adapter,
            ctx.accounts.adapter.key(),
            LedgerError::Unauthorized
        );
        require_book_schema(&ctx.accounts.book)?;
        ctx.accounts.book.halt = halt;
        Ok(())
    }

    pub fn bump_funding_epoch(ctx: Context<AdapterBook>, epoch: u64) -> Result<()> {
        let cfg = load_vault_config(&ctx.accounts.config)?;
        require_keys_eq!(
            cfg.adapter,
            ctx.accounts.adapter.key(),
            LedgerError::Unauthorized
        );
        require_book_schema(&ctx.accounts.book)?;
        require!(
            (ctx.accounts.book.halt & cc::INVARIANT_BROKEN) == 0
                && (cfg.paused & cc::INVARIANT_BROKEN) == 0,
            LedgerError::Halted
        );
        require!(
            epoch
                == ctx
                    .accounts
                    .book
                    .funding_epoch
                    .checked_add(1)
                    .ok_or(LedgerError::Overflow)?,
            LedgerError::BadFundingEpoch
        );
        ctx.accounts.book.funding_epoch = epoch;
        Ok(())
    }

    pub fn allocate_funding(
        ctx: Context<AllocateFunding>,
        epoch: u64,
        fold: bool,
        entries: Vec<FundingEntry>,
    ) -> Result<()> {
        let cfg = load_vault_config(&ctx.accounts.config)?;
        require_keys_eq!(
            cfg.adapter,
            ctx.accounts.adapter.key(),
            LedgerError::Unauthorized
        );
        require_book_schema(&ctx.accounts.book)?;
        require_user_schema(&ctx.accounts.user_ledger)?;
        require!(
            (ctx.accounts.book.halt & cc::INVARIANT_BROKEN) == 0
                && (cfg.paused & cc::INVARIANT_BROKEN) == 0,
            LedgerError::Halted
        );
        require!(
            entries.len() <= cc::MAX_USER_POSITIONS,
            LedgerError::PositionCap
        );

        let book_epoch = ctx.accounts.book.funding_epoch;
        let ledger = &mut ctx.accounts.user_ledger;

        if fold {
            if ledger
                .last_funding_epoch
                .checked_add(1)
                .ok_or(LedgerError::Overflow)?
                == epoch
                && epoch == book_epoch
            {
                apply_funding_entries(ledger, &entries)?;
                ledger.last_funding_epoch = epoch;
            } else {
                require!(
                    epoch == book_epoch && epoch == ledger.last_funding_epoch,
                    LedgerError::BadFundingEpoch
                );
            }
            let under_margined = fold_unsettled(ledger)?;
            apply_cash_halt(
                &mut ctx.accounts.book,
                ledger.bad_debt_usdc > 0,
                under_margined,
            );
            return Ok(());
        }

        require!(epoch == book_epoch, LedgerError::BadFundingEpoch);
        require!(
            epoch
                == ledger
                    .last_funding_epoch
                    .checked_add(1)
                    .ok_or(LedgerError::Overflow)?,
            LedgerError::BadFundingEpoch
        );
        apply_funding_entries(ledger, &entries)?;
        ledger.last_funding_epoch = epoch;
        Ok(())
    }

    pub fn update_book_collateral(
        ctx: Context<AdapterBook>,
        phoenix_collateral: u64,
    ) -> Result<()> {
        let cfg = load_vault_config(&ctx.accounts.config)?;
        require_keys_eq!(
            cfg.adapter,
            ctx.accounts.adapter.key(),
            LedgerError::Unauthorized
        );
        require_book_schema(&ctx.accounts.book)?;
        ctx.accounts.book.phoenix_collateral = phoenix_collateral;
        Ok(())
    }

    /// Adapter heartbeat. Writes `Book.last_scan_ms`. Does not flatten.
    pub fn heartbeat_scan(ctx: Context<AdapterBook>, now_ms: u64) -> Result<()> {
        let cfg = load_vault_config(&ctx.accounts.config)?;
        require_keys_eq!(
            cfg.adapter,
            ctx.accounts.adapter.key(),
            LedgerError::Unauthorized
        );
        require_book_schema(&ctx.accounts.book)?;
        ctx.accounts.book.last_scan_ms = now_ms;
        Ok(())
    }

    /// Tentative flatten one asset (P-L4). Adapter only. Reverts user pending oids
    /// on that asset, parks a liquidating oid, **does not** move Book. Ack of the
    /// reducing hedge moves Book, same as `place_order`.
    pub fn liquidate_user(
        ctx: Context<LiquidateUser>,
        asset_id: u16,
        client_oid: [u8; 16],
    ) -> Result<()> {
        let cfg = load_vault_config(&ctx.accounts.config)?;
        require_keys_eq!(
            cfg.adapter,
            ctx.accounts.adapter.key(),
            LedgerError::Unauthorized
        );
        require_book_schema(&ctx.accounts.book)?;
        require_user_schema(&ctx.accounts.user_ledger)?;

        let ledger = &mut ctx.accounts.user_ledger;
        revert_pending_on_asset(ledger, asset_id)?;
        if let Some(idx) = find_position_index(ledger, asset_id) {
            let delta = ledger.positions[idx].unsettled_funding;
            apply_signed_cash(ledger, delta)?;
            ledger.positions[idx].unsettled_funding = 0;
        }

        let lots = position_lots(ledger, asset_id);
        if lots == 0 {
            compact_positions(ledger);
            let under_margined = resync_stub_margin(ledger)?;
            apply_cash_halt(
                &mut ctx.accounts.book,
                ledger.bad_debt_usdc > 0,
                under_margined,
            );
            return Ok(());
        }

        require!(
            (ledger.pending_oid_count as usize) < cc::MAX_OPEN_OIDS_PER_USER,
            LedgerError::OidCap
        );
        require!(
            !oid_duplicate(ledger, &client_oid),
            LedgerError::DuplicateOid
        );
        let slot = find_free_oid_slot(ledger).ok_or(LedgerError::OidCap)?;

        let lots_delta = lots.checked_neg().ok_or(LedgerError::Overflow)?;
        let prior_position_im = position_reserved_im(ledger, asset_id);
        set_position_lots(ledger, asset_id, 0, prior_position_im)?;
        compact_positions(ledger);
        let under_margined = resync_stub_margin(ledger)?;

        ledger.open_oids[slot] = OpenOid {
            client_oid,
            asset_id,
            lots_delta,
            state: cc::OID_LIQUIDATING,
            limit_price_ticks: 0,
            last_valid_slot: 0,
        };
        ledger.pending_oid_count = ledger
            .pending_oid_count
            .checked_add(1)
            .ok_or(LedgerError::Overflow)?;
        apply_cash_halt(
            &mut ctx.accounts.book,
            ledger.bad_debt_usdc > 0,
            under_margined,
        );
        Ok(())
    }

    pub fn delegate_user(ctx: Context<DelegateUser>) -> Result<()> {
        privacy::delegate_user_handler(ctx)
    }

    pub fn delegate_book(ctx: Context<DelegateBook>) -> Result<()> {
        privacy::delegate_book_handler(ctx)
    }

    pub fn delegate_fees(ctx: Context<DelegateFees>) -> Result<()> {
        privacy::delegate_fees_handler(ctx)
    }

    /// One-shot conversion of a legacy delegated UserLedger to the current layout.
    pub fn migrate_user_ledger(ctx: Context<MigrateUserLedger>) -> Result<()> {
        migration::migrate_user_ledger_handler(ctx)
    }

    /// One-shot conversion of the legacy delegated Book to the current layout.
    pub fn migrate_book(ctx: Context<MigrateBook>) -> Result<()> {
        migration::migrate_book_handler(ctx)
    }

    pub fn init_user_permission(ctx: Context<UserPermission>) -> Result<()> {
        privacy::init_user_permission_handler(ctx)
    }

    pub fn init_book_permission(ctx: Context<BookPermission>) -> Result<()> {
        privacy::init_book_permission_handler(ctx)
    }

    pub fn init_fees_permission(ctx: Context<FeesPermission>) -> Result<()> {
        privacy::init_fees_permission_handler(ctx)
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
        space = Book::ACCOUNT_SPACE,
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
    #[account(seeds = [cc::SEED_BOOK], bump = book.bump)]
    pub book: Box<Account<'info, Book>>,
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

#[derive(Accounts)]
pub struct LiquidateUser<'info> {
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
}

#[account]
#[derive(InitSpace)]
pub struct UserLedger {
    pub schema_version: u8,
    pub user: Pubkey,
    pub free: u64,
    pub reserved: u64,
    pub withdrawable: u64,
    pub bad_debt_usdc: u64,
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
    pub schema_version: u8,
    pub residual_len: u8,
    pub residuals: [Residual; 32],
    pub phoenix_collateral: u64,
    pub last_ack_slot_er: u64,
    pub invariant_ok: u8,
    pub halt: u8,
    pub funding_epoch: u64,
    pub last_scan_ms: u64,
    pub bump: u8,
}

impl Book {
    pub const ACCOUNT_SPACE: usize = 8 + Self::INIT_SPACE;
}

impl UserLedger {
    pub const ACCOUNT_SPACE: usize = 8 + Self::INIT_SPACE;
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
    /// Exact Phoenix execution bound. Zero means no bound was supplied and must not reach the venue.
    pub limit_price_ticks: u64,
    /// Phoenix L1 expiry slot. Zero means no deadline was supplied and must not reach the venue.
    pub last_valid_slot: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, InitSpace)]
pub struct Residual {
    pub asset_id: u16,
    pub lots: i64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub struct FundingEntry {
    pub asset_id: u16,
    pub delta_usdc: i64,
}

#[event]
pub struct PhoenixFillAcknowledged {
    pub user: Pubkey,
    pub client_oid: [u8; 16],
    pub asset_id: u16,
    pub filled_lots: i64,
    pub realized_usdc: i64,
    pub fee_usdc: u64,
    pub bad_debt_usdc: u64,
    pub under_margined: bool,
}

#[derive(Accounts)]
pub struct AllocateFunding<'info> {
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
}

fn asset_allowlisted(cfg: &cinder_vault::Config, asset_id: u16) -> bool {
    cfg.allowlist_assets
        .iter()
        .take(cfg.allowlist_len as usize)
        .any(|a| *a == asset_id)
}

fn load_vault_config(info: &AccountInfo) -> Result<cinder_vault::Config> {
    require_keys_eq!(*info.owner, VAULT_PROGRAM_ID, LedgerError::InvalidConfig);
    let data = info.try_borrow_data()?;
    let mut slice: &[u8] = &data;
    cinder_vault::Config::try_deserialize(&mut slice)
        .map_err(|_| error!(LedgerError::InvalidConfig))
}

fn require_user_schema(ledger: &UserLedger) -> Result<()> {
    require!(
        ledger.schema_version == cc::ACCOUNT_SCHEMA_VERSION,
        LedgerError::UnsupportedAccountSchema
    );
    Ok(())
}

fn require_book_schema(book: &Book) -> Result<()> {
    require!(
        book.schema_version == cc::ACCOUNT_SCHEMA_VERSION,
        LedgerError::UnsupportedAccountSchema
    );
    Ok(())
}

fn stub_im(lots: i64) -> Result<u64> {
    cc::stub_cinder_im(lots.unsigned_abs()).ok_or(error!(LedgerError::Overflow))
}

fn stub_notional_all(ledger: &UserLedger) -> Result<u64> {
    let mut n = 0u64;
    for p in ledger.positions[..ledger.positions_len as usize].iter() {
        let piece = cc::stub_notional(p.lots.unsigned_abs()).ok_or(LedgerError::Overflow)?;
        n = n.checked_add(piece).ok_or(LedgerError::Overflow)?;
    }
    Ok(n)
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

fn pending_delta_for_asset(ledger: &UserLedger, asset_id: u16) -> Result<i64> {
    let mut s = 0i64;
    for o in ledger.open_oids.iter() {
        if o.asset_id == asset_id
            && o.lots_delta != 0
            && (o.state == cc::OID_PENDING || o.state == cc::OID_LIQUIDATING)
        {
            s = s.checked_add(o.lots_delta).ok_or(LedgerError::Overflow)?;
        }
    }
    Ok(s)
}

fn position_entry(ledger: &UserLedger, asset_id: u16) -> i64 {
    ledger.positions[..ledger.positions_len as usize]
        .iter()
        .find(|p| p.asset_id == asset_id)
        .map(|p| p.entry_quote_lots)
        .unwrap_or(0)
}

fn position_reserved_im(ledger: &UserLedger, asset_id: u16) -> u64 {
    ledger.positions[..ledger.positions_len as usize]
        .iter()
        .find(|p| p.asset_id == asset_id)
        .map(|p| p.reserved_im)
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
            pos.reserved_im = 0;
            // Keep a 0-lot row while entry remains so a reducing ack can realize.
            if pos.entry_quote_lots == 0 && pos.unsettled_funding == 0 {
                compact_positions(ledger);
            }
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

fn compact_positions(ledger: &mut UserLedger) {
    let mut w = 0usize;
    let n = ledger.positions_len as usize;
    for r in 0..n {
        if ledger.positions[r].lots != 0
            || ledger.positions[r].entry_quote_lots != 0
            || ledger.positions[r].unsettled_funding != 0
        {
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

fn unsettled_funding_zero(ledger: &UserLedger) -> bool {
    ledger.positions[..ledger.positions_len as usize]
        .iter()
        .all(|p| p.unsettled_funding == 0)
}

fn find_position_index(ledger: &UserLedger, asset_id: u16) -> Option<usize> {
    ledger.positions[..ledger.positions_len as usize]
        .iter()
        .position(|p| p.asset_id == asset_id)
}

fn apply_funding_entries(ledger: &mut UserLedger, entries: &[FundingEntry]) -> Result<()> {
    let mut seen = [false; 16];
    for e in entries.iter() {
        if e.delta_usdc == 0 {
            continue;
        }
        let idx = find_position_index(ledger, e.asset_id).ok_or(LedgerError::FundingNoPosition)?;
        require!(!seen[idx], LedgerError::DuplicateFundingAsset);
        seen[idx] = true;
        let pos = &mut ledger.positions[idx];
        pos.unsettled_funding = pos
            .unsettled_funding
            .checked_add(e.delta_usdc)
            .ok_or(LedgerError::Overflow)?;
    }
    Ok(())
}

fn apply_signed_cash(ledger: &mut UserLedger, delta: i64) -> Result<()> {
    let mut free = ledger.free;
    let mut reserved = ledger.reserved;
    let mut bad_debt = ledger.bad_debt_usdc;
    cc::apply_signed_cash(&mut free, &mut reserved, &mut bad_debt, delta)
        .ok_or(error!(LedgerError::Overflow))?;
    ledger.free = free;
    ledger.reserved = reserved;
    ledger.bad_debt_usdc = bad_debt;
    Ok(())
}

fn debit_cash(ledger: &mut UserLedger, amount: u64) -> Result<()> {
    let mut free = ledger.free;
    let mut reserved = ledger.reserved;
    let mut bad_debt = ledger.bad_debt_usdc;
    cc::debit_cash(&mut free, &mut reserved, &mut bad_debt, amount)
        .ok_or(error!(LedgerError::Overflow))?;
    ledger.free = free;
    ledger.reserved = reserved;
    ledger.bad_debt_usdc = bad_debt;
    Ok(())
}

fn fold_unsettled(ledger: &mut UserLedger) -> Result<bool> {
    let n = ledger.positions_len as usize;
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&i| ledger.positions[i].unsettled_funding < 0);
    for i in order {
        let delta = ledger.positions[i].unsettled_funding;
        apply_signed_cash(ledger, delta)?;
        ledger.positions[i].unsettled_funding = 0;
    }
    resync_stub_margin(ledger)
}

fn resync_stub_margin(ledger: &mut UserLedger) -> Result<bool> {
    let n = ledger.positions_len as usize;
    let mut targets = [0u64; cc::MAX_USER_POSITIONS];
    let mut target_total = 0u64;
    for i in 0..n {
        targets[i] = if ledger.positions[i].lots == 0 {
            0
        } else {
            stub_im(ledger.positions[i].lots)?
        };
        target_total = target_total
            .checked_add(targets[i])
            .ok_or(LedgerError::Overflow)?;
    }
    rebalance_margin_targets(ledger, &targets[..n], target_total)
}

fn rebalance_position_margin(
    ledger: &mut UserLedger,
    asset_id: u16,
    post_position_im_usdc: u64,
) -> Result<bool> {
    let n = ledger.positions_len as usize;
    let mut targets = [0u64; cc::MAX_USER_POSITIONS];
    let mut target_total = 0u64;
    let mut target_position_found = false;
    for (i, position) in ledger.positions[..n].iter().enumerate() {
        targets[i] = if position.asset_id == asset_id && position.lots != 0 {
            target_position_found = true;
            post_position_im_usdc
        } else {
            position.reserved_im
        };
        target_total = target_total
            .checked_add(targets[i])
            .ok_or(LedgerError::Overflow)?;
    }
    require!(
        target_position_found || post_position_im_usdc == 0,
        LedgerError::BadPostFillMargin
    );
    rebalance_margin_targets(ledger, &targets[..n], target_total)
}

fn rebalance_margin_targets(
    ledger: &mut UserLedger,
    targets: &[u64],
    target_total: u64,
) -> Result<bool> {
    let cash = ledger
        .free
        .checked_add(ledger.reserved)
        .ok_or(LedgerError::Overflow)?;
    let actual_total = cash.min(target_total);
    ledger.reserved = actual_total;
    ledger.free = cash
        .checked_sub(actual_total)
        .ok_or(LedgerError::Overflow)?;

    let mut remaining = actual_total;
    for (position, target) in ledger.positions[..targets.len()]
        .iter_mut()
        .zip(targets.iter().copied())
    {
        let actual = remaining.min(target);
        position.reserved_im = actual;
        remaining -= actual;
    }
    require!(remaining == 0, LedgerError::Overflow);
    Ok(actual_total < target_total)
}

fn apply_cash_halt(book: &mut Book, has_bad_debt: bool, under_margined: bool) {
    if under_margined {
        book.halt |= cc::HALT_ENTRIES;
    }
    if has_bad_debt {
        book.halt |= cc::BAD_DEBT | cc::HALT_ENTRIES | cc::HALT_WITHDRAW;
    }
}

fn oid_slot_free(oid: &OpenOid) -> bool {
    match oid.state {
        cc::OID_PENDING => oid.lots_delta == 0,
        cc::OID_ACKED | cc::OID_FAILED => true,
        cc::OID_LIQUIDATING => oid.lots_delta == 0,
        _ => true,
    }
}

fn oid_duplicate(ledger: &UserLedger, client_oid: &[u8; 16]) -> bool {
    ledger
        .open_oids
        .iter()
        .any(|o| !oid_slot_free(o) && o.client_oid == *client_oid)
}

fn find_free_oid_slot(ledger: &UserLedger) -> Option<usize> {
    ledger.open_oids.iter().position(oid_slot_free)
}

fn revert_pending_on_asset(ledger: &mut UserLedger, asset_id: u16) -> Result<()> {
    let idxs: Vec<usize> = ledger
        .open_oids
        .iter()
        .enumerate()
        .filter(|(_, o)| o.asset_id == asset_id && o.state == cc::OID_PENDING && o.lots_delta != 0)
        .map(|(i, _)| i)
        .collect();
    for idx in idxs {
        let oid = ledger.open_oids[idx];
        let tentative = position_lots(ledger, oid.asset_id);
        let restored = tentative
            .checked_sub(oid.lots_delta)
            .ok_or(LedgerError::Overflow)?;
        let old_im = stub_im(tentative)?;
        let new_im = stub_im(restored)?;
        apply_im_delta(ledger, old_im, new_im)?;
        set_position_lots(ledger, oid.asset_id, restored, new_im)?;
        ledger.open_oids[idx].state = cc::OID_LIQUIDATING;
        ledger.open_oids[idx].lots_delta = 0;
        ledger.pending_oid_count = ledger
            .pending_oid_count
            .checked_sub(1)
            .ok_or(LedgerError::Overflow)?;
    }
    Ok(())
}

fn take_pending_oid(ledger: &UserLedger, client_oid: &[u8; 16]) -> Result<(usize, OpenOid)> {
    let idx = ledger
        .open_oids
        .iter()
        .position(|o| {
            o.client_oid == *client_oid
                && o.lots_delta != 0
                && (o.state == cc::OID_PENDING || o.state == cc::OID_LIQUIDATING)
        })
        .ok_or(LedgerError::OidNotFound)?;
    Ok((idx, ledger.open_oids[idx]))
}

fn find_acknowledged_oid<'a>(ledger: &'a UserLedger, client_oid: &[u8; 16]) -> Option<&'a OpenOid> {
    ledger
        .open_oids
        .iter()
        .find(|oid| oid.client_oid == *client_oid && oid.state == cc::OID_ACKED)
}

fn add_book_lots(book: &mut Book, asset_id: u16, filled_lots: i64) -> Result<()> {
    if let Some(r) = book.residuals[..book.residual_len as usize]
        .iter_mut()
        .find(|r| r.asset_id == asset_id)
    {
        r.lots = r
            .lots
            .checked_add(filled_lots)
            .ok_or(LedgerError::Overflow)?;
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
    #[msg("post-fill margin must be zero for a flat position")]
    BadPostFillMargin,
    #[msg("bad slippage bps")]
    BadSlippage,
    #[msg("missing ER validator")]
    MissingValidator,
    #[msg("validator is not Config.er_validator")]
    BadValidator,
    #[msg("funding epoch mismatch")]
    BadFundingEpoch,
    #[msg("no position for funding asset")]
    FundingNoPosition,
    #[msg("duplicate funding asset")]
    DuplicateFundingAsset,
    #[msg("unsettled funding must be folded first")]
    UnsettledFunding,
    #[msg("account has already been migrated")]
    AccountAlreadyMigrated,
    #[msg("unsupported account schema")]
    UnsupportedAccountSchema,
    #[msg("invalid legacy account data")]
    InvalidMigrationData,
    #[msg("legacy account does not match its expected PDA")]
    InvalidMigrationAccount,
    #[msg("account must be rent-exempt at the new size before migration")]
    MigrationRentShortfall,
}
