//! Shared full/manual and bounded/autonomous private liquidation transition.
use super::*;

pub(super) fn liquidate(
    ctx: Context<LiquidateUser>,
    asset_id: u16,
    client_oid: [u8; 16],
    limit_price_ticks: u64,
    last_valid_slot: u64,
    close_lots: Option<u64>,
    post_position_im_usdc: u64,
) -> Result<()> {
    let cfg = load_vault_config(&ctx.accounts.config)?;
    require_keys_eq!(
        cfg.adapter,
        ctx.accounts.adapter.key(),
        LedgerError::Unauthorized
    );
    require_book_schema(&ctx.accounts.book)?;
    require_user_schema(&ctx.accounts.user_ledger)?;
    require!(limit_price_ticks != 0, LedgerError::ZeroLimitPrice);
    require!(last_valid_slot != 0, LedgerError::ZeroDeadline);
    // This is an L1 Phoenix deadline, not an ER slot.

    let ledger = &mut ctx.accounts.user_ledger;
    if close_lots.is_some() {
        require!(ledger.pending_oid_count == 0, LedgerError::OidCap);
    }
    revert_pending_on_asset(ledger, asset_id)?;
    if let Some(idx) = find_position_index(ledger, asset_id) {
        let delta = ledger.positions[idx].unsettled_funding;
        apply_signed_cash(ledger, delta)?;
        ledger.positions[idx].unsettled_funding = 0;
    }

    let lots = position_lots(ledger, asset_id);
    if lots == 0 {
        compact_positions(ledger);
        let under_margined = rebalance_stored_margins(ledger)?;
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

    let amount = close_lots.unwrap_or(lots.unsigned_abs());
    require!(
        amount > 0 && amount <= lots.unsigned_abs(),
        LedgerError::Overflow
    );
    let lots_delta = i64::try_from(amount)
        .map_err(|_| LedgerError::Overflow)?
        .checked_mul(-lots.signum())
        .ok_or(LedgerError::Overflow)?;
    let remaining = lots.checked_add(lots_delta).ok_or(LedgerError::Overflow)?;
    require!(
        remaining != 0 || post_position_im_usdc == 0,
        LedgerError::Overflow
    );
    set_position_lots(ledger, asset_id, remaining, post_position_im_usdc)?;
    compact_positions(ledger);
    let under_margined = rebalance_stored_margins(ledger)?;

    ledger.open_oids[slot] = OpenOid {
        client_oid,
        asset_id,
        lots_delta,
        state: cc::OID_LIQUIDATING,
        limit_price_ticks,
        last_valid_slot,
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
