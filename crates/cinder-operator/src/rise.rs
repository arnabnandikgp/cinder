//! Authoritative Rise views and the shared risk boundary. No REST cache,
//! stub margin, or local fetch timestamp stands in for a fresh venue mark.
use crate::rpc::{data, number, string, unix_ms, Result, RuntimeError};
use crate::runtime::{config_key, decode, text, vault_id, Context};
use crate::transaction::{bytes, key, rise_ix, sign};
use base64::{engine::general_purpose::STANDARD, Engine};
use cinder_adapter::{MarketStatus, RiseRiskEngine, RiskEngine, RiskSnapshot, UnitStatus};
use cinder_common as cc;
use cinder_ledger::UserLedger;
use phoenix_rise_accounts::owned::{GlobalConfiguration, PerpAssetMapOwned, Trader};
use phoenix_rise_ix::hawkeye::{
    self, HawkeyeReturnData, HawkeyeTraderViewAccounts, ViewMarginReturn,
};
use phoenix_rise_math::{
    self as math, LeverageTier, LeverageTiers, LimitOrderMarginState, PerpAssetMetadata,
    SignedBaseLots, SignedQuoteLots, TraderPosition,
};
use serde_json::{json, Value};
use solana_signer::Signer;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct RiseView {
    pub observed_ms: u64,
    pub mark_ms: u64,
    pub slot: u64,
    pub positions: BTreeMap<u16, i64>,
    pub collateral: u64,
    pub funding: i128,
    pub vault_balance: u64,
    pub halt: u8,
    pub safe: bool,
    pub markets: BTreeMap<u16, PerpAssetMetadata>,
}
fn token_balance(row: &Value, mint: &[u8; 32], authority: &[u8; 32]) -> Result<u64> {
    let bytes = data(row, "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")?;
    if bytes.len() != 165
        || bytes[..32] != *mint
        || bytes[32..64] != *authority
        || bytes[108] != 1
        || bytes[72..76] != [0; 4]
        || bytes[109..113] != [0; 4]
        || bytes[129..133] != [0; 4]
    {
        return Err(RuntimeError::Identity);
    }
    Ok(u64::from_le_bytes(
        bytes[64..72].try_into().map_err(|_| RuntimeError::Decode)?,
    ))
}
fn mint_decimals(row: &Value) -> Result<()> {
    let bytes = data(row, "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")?;
    if bytes.len() != 82 || bytes[44] != 6 || bytes[45] != 1 {
        return Err(RuntimeError::Unsupported);
    }
    Ok(())
}
fn slot_time(context: &mut Context, slot: u64) -> Result<u64> {
    let result = context
        .l1
        .call("getBlockTime", json!([slot]), &context.signer)?;
    number(&result)?
        .checked_mul(1000)
        .ok_or(RuntimeError::Decode)
}
fn simulate(
    context: &mut Context,
    ix: phoenix_rise_ix::types::Instruction,
    min_slot: u64,
) -> Result<(u64, HawkeyeReturnData)> {
    let signed = sign(&mut context.l1, &context.signer, rise_ix(ix))?;
    let result=context.l1.call("simulateTransaction",json!([signed.encoded,{"encoding":"base64","sigVerify":true,"commitment":"confirmed","minContextSlot":min_slot}]),&context.signer)?;
    if result["value"]["err"] != Value::Null
        || string(&result["value"]["returnData"]["programId"])?
            != hawkeye::HAWKEYE_PROGRAM_ID.to_string()
        || string(&result["value"]["returnData"]["data"][1])? != "base64"
    {
        return Err(RuntimeError::Rpc);
    }
    let bytes = STANDARD
        .decode(string(&result["value"]["returnData"]["data"][0])?)
        .map_err(|_| RuntimeError::Decode)?;
    let slot = number(&result["context"]["slot"])?;
    if slot < min_slot {
        return Err(RuntimeError::Stale);
    }
    Ok((
        slot,
        hawkeye::decode_hawkeye_return_data(&bytes).map_err(|_| RuntimeError::Decode)?,
    ))
}

pub(crate) fn load(context: &mut Context, assets: &BTreeSet<u16>) -> Result<RiseView> {
    let cfg = context.vault_config()?;
    if cfg.allowlist_len as usize > cfg.allowlist_assets.len() {
        return Err(RuntimeError::Identity);
    }
    let mut assets = assets.clone();
    assets.extend(&cfg.allowlist_assets[..cfg.allowlist_len as usize]);
    // The experimental runtime is bounded. Never silently truncate a market
    // or turn an unsupported account layout into an apparently empty pool.
    if assets.contains(&0) || assets.len() > 32 {
        return Err(RuntimeError::Unsupported);
    }
    let mut keys = vec![
        text(&config_key()),
        context.config.phoenix_global_config.clone(),
        context.config.phoenix_asset_map.clone(),
        context.config.phoenix_trader.clone(),
        cfg.vault_usdc_ata.to_string(),
        cfg.usdc_mint.to_string(),
        context.config.phoenix_quote_mint.clone(),
    ];
    keys.extend(
        context
            .config
            .global_trader_index
            .iter()
            .chain(&context.config.active_trader_buffer)
            .cloned(),
    );
    let (slot, rows) = context.l1.accounts(&keys, &context.signer)?;
    let observed_ms = unix_ms();
    let cfg: cinder_vault::Config = decode(&rows[0], &text(&vault_id()))?;
    let program = &context.config.phoenix_program;
    let global = GlobalConfiguration::try_from_account_bytes(&data(&rows[1], program)?)
        .map_err(|_| RuntimeError::Decode)?;
    let map = PerpAssetMapOwned::try_from_account_bytes(&data(&rows[2], program)?)
        .map_err(|_| RuntimeError::Decode)?;
    let trader = Trader::try_from_account_bytes(&data(&rows[3], program)?)
        .map_err(|_| RuntimeError::Decode)?;
    if global.account_key.to_bytes() != bytes(&context.config.phoenix_global_config)?
        || global.perp_asset_map_key.to_bytes() != bytes(&context.config.phoenix_asset_map)?
        || global.quote_decimals != 6
        || global.canonical_token_mint_key.to_bytes() != bytes(&context.config.phoenix_quote_mint)?
        || trader.key.to_bytes() != bytes(&context.config.phoenix_trader)?
        || trader.trader_subaccount_index != 0
        || (trader.position_authority.to_bytes() != context.signer.pubkey().to_bytes()
            && trader.authority.to_bytes() != context.signer.pubkey().to_bytes())
        || trader.native_sol_collateral != 0
        || trader.withdraw_queue_node.is_some()
        || trader.num_markets_with_splines != 0
        || !trader.occupied_conditional_order_indices.is_empty()
        || global.global_trader_index_header_key.to_bytes()
            != bytes(&context.config.global_trader_index[0])?
        || global.active_trader_buffer_header_key.to_bytes()
            != bytes(&context.config.active_trader_buffer[0])?
    {
        return Err(RuntimeError::Identity);
    }
    mint_decimals(&rows[5])?;
    // Phoenix's canonical collateral token is converted by Ember; it is not
    // necessarily the vault's USDC mint. The official SDK uses the same native
    // six-decimal amount in both legs. Pin and verify both mint identities.
    mint_decimals(&rows[6])?;
    let vault_balance = token_balance(
        &rows[4],
        &cfg.usdc_mint.to_bytes(),
        &cfg.vault_authority.to_bytes(),
    )?;
    let accounts = HawkeyeTraderViewAccounts {
        phoenix_program_id: key(program)?,
        global_config: key(&context.config.phoenix_global_config)?,
        perp_asset_map: key(&context.config.phoenix_asset_map)?,
        trader: key(&context.config.phoenix_trader)?,
        global_trader_index: context
            .config
            .global_trader_index
            .iter()
            .map(|s| key(s))
            .collect::<Result<_>>()?,
        active_trader_buffer: context
            .config
            .active_trader_buffer
            .iter()
            .map(|s| key(s))
            .collect::<Result<_>>()?,
    };
    // Hawkeye reads hot/cold/active trader state through Rise itself. The cold
    // trader account alone is not a current exposure or collateral source.
    let (view_slot, margin) = simulate(
        context,
        hawkeye::create_hawkeye_view_margin_ix(accounts.clone()),
        slot,
    )?;
    let margin: ViewMarginReturn = match margin {
        HawkeyeReturnData::Margin(m) => m,
        _ => return Err(RuntimeError::Decode),
    };
    if margin.version != hawkeye::HAWKEYE_RETURN_VERSION {
        return Err(RuntimeError::Unsupported);
    }
    let mut markets = BTreeMap::new();
    let mut positions = BTreeMap::new();
    let mut count = 0u16;
    let mut mark_ms = u64::MAX;
    for asset in assets {
        let mapping = context.config.market(asset)?;
        let native = mapping.phoenix_asset_id;
        let (symbol, metadata) = map
            .iter()
            .find(|(_, m)| m.static_market_params.asset_id == native)
            .ok_or(RuntimeError::Unsupported)?;
        if *symbol != mapping.symbol {
            return Err(RuntimeError::Identity);
        }
        if metadata.risk_params.isolated_only != 0 {
            return Err(RuntimeError::Unsupported);
        }
        let (asset_slot, view) = simulate(
            context,
            hawkeye::create_hawkeye_view_margin_for_asset_ix(accounts.clone(), native),
            view_slot,
        )?;
        let view = match view {
            HawkeyeReturnData::Asset(a) => a,
            _ => return Err(RuntimeError::Decode),
        };
        if view.version != hawkeye::HAWKEYE_RETURN_VERSION
            || view.asset_id != native
            || asset_slot < view_slot
            || view.base_lots == i64::MIN
        {
            return Err(RuntimeError::Identity);
        }
        if view.has_position_or_orders != 0 {
            count = count.checked_add(1).ok_or(RuntimeError::Decode)?;
        }
        if view.base_lots != 0 {
            positions.insert(asset, view.base_lots);
        }
        let tiers = metadata
            .risk_params
            .leverage_tiers
            .iter()
            .map(|t| LeverageTier {
                upper_bound_size: t.upper_bound_size,
                max_leverage: t.max_leverage,
                limit_order_risk_factor: t.limit_order_risk_factor,
            })
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| RuntimeError::Unsupported)?;
        let tiers = LeverageTiers::new(tiers).map_err(|_| RuntimeError::Unsupported)?;
        let factors = metadata
            .risk_params
            .risk_factors
            .clone()
            .try_into()
            .map_err(|_| RuntimeError::Unsupported)?;
        let mut normalized = PerpAssetMetadata::new(
            symbol.clone(),
            u64::from(asset),
            metadata.static_market_params.base_lot_decimals,
            math::Ticks::new(view.mark_price_ticks),
            metadata.static_market_params.tick_size,
            tiers,
            factors,
            metadata.risk_params.cancel_order_risk_factor,
            u16::try_from(metadata.risk_params.upnl_risk_factor.as_inner())
                .map_err(|_| RuntimeError::Unsupported)?,
            u16::try_from(
                metadata
                    .risk_params
                    .upnl_risk_factor_for_withdrawals
                    .as_inner(),
            )
            .map_err(|_| RuntimeError::Unsupported)?,
        );
        normalized.cumulative_funding_rate = metadata.funding_accumulator.cumulative_funding_rate;
        let mark_slot = metadata.oracle_price.mark_price.price.slot;
        if mark_slot > asset_slot
            || view.mark_price_ticks == 0
            || normalized.tick_size.as_inner() == 0
        {
            return Err(RuntimeError::Stale);
        }
        mark_ms = mark_ms.min(slot_time(context, mark_slot)?);
        markets.insert(asset, normalized);
    }
    if count != margin.position_count {
        return Err(RuntimeError::Incomplete);
    }
    // Reject mutation while simulations are in flight. The simulated result
    // must belong to the same actual public state, not adjacent snapshots.
    let (_, after) = context.l1.accounts(&keys, &context.signer)?;
    if rows != after {
        return Err(RuntimeError::Stale);
    }
    let now = unix_ms();
    let trader_ms = slot_time(context, view_slot)?;
    if mark_ms == u64::MAX {
        mark_ms = trader_ms;
    }
    for time in [observed_ms, trader_ms, mark_ms] {
        if now
            .checked_sub(time)
            .is_none_or(|age| age > cc::MARK_STALE_MS)
        {
            return Err(RuntimeError::Stale);
        }
    }
    let collateral =
        u64::try_from(margin.collateral_quote_lots).map_err(|_| RuntimeError::Identity)?;
    let exchange_active =
        global.is_exchange_active(phoenix_rise_accounts::global_config::LastRestartSlot::Unknown);
    Ok(RiseView {
        observed_ms: trader_ms,
        mark_ms,
        slot: view_slot,
        positions,
        collateral,
        funding: i128::from(margin.unsettled_funding_quote_lots),
        vault_balance,
        halt: cfg.paused,
        safe: exchange_active
            && margin.risk_tier == 0
            && margin.is_liquidatable == 0
            && margin.effective_collateral_quote_lots >= 0
            && u64::try_from(margin.effective_collateral_quote_lots)
                .is_ok_and(|e| e >= margin.initial_margin_quote_lots),
        markets,
    })
}

pub(crate) fn user_margin(
    view: &RiseView,
    ledger: &UserLedger,
    asset: u16,
    post_lots: i64,
    entry: i64,
) -> Result<u64> {
    let metadata = view.markets.get(&asset).ok_or(RuntimeError::Unsupported)?;
    let price = metadata
        .mark_price
        .as_inner()
        .checked_mul(metadata.tick_size.as_inner())
        .ok_or(RuntimeError::Decode)?;
    let notional = post_lots
        .unsigned_abs()
        .checked_mul(price)
        .ok_or(RuntimeError::Decode)?;
    // Rise wrappers use signed i64 quote values. Refuse clipping in its uPnL
    // helper or overflow in margin math before entering the SDK.
    let upnl = i128::from(post_lots)
        .checked_mul(i128::from(price))
        .and_then(|v| v.checked_sub(i128::from(entry)))
        .ok_or(RuntimeError::Decode)?;
    i64::try_from(upnl).map_err(|_| RuntimeError::Decode)?;
    let mut position = TraderPosition::new();
    position.base_lot_position = SignedBaseLots::new(post_lots);
    position.virtual_quote_lot_position =
        SignedQuoteLots::new(entry.checked_neg().ok_or(RuntimeError::Decode)?);
    let im = math::initial_margin_for_asset(
        metadata,
        &position,
        &LimitOrderMarginState::empty(),
        math::risk::RiskAction::View,
    )
    .map_err(|_| RuntimeError::Decode)?;
    let mm = math::position_maintenance_margin(metadata, im).map_err(|_| RuntimeError::Decode)?;
    let mut total = notional;
    for p in &ledger.positions[..ledger.positions_len as usize] {
        if p.asset_id == asset {
            continue;
        }
        let m = view
            .markets
            .get(&p.asset_id)
            .ok_or(RuntimeError::Unsupported)?;
        let n = p
            .lots
            .unsigned_abs()
            .checked_mul(m.mark_price.as_inner())
            .and_then(|v| v.checked_mul(m.tick_size.as_inner()))
            .ok_or(RuntimeError::Decode)?;
        total = total.checked_add(n).ok_or(RuntimeError::Decode)?;
    }
    let snapshot = RiskSnapshot {
        asset_id: asset,
        position_lots: post_lots,
        mark_price_ticks: metadata.mark_price.as_inner(),
        observed_slot: view.slot,
        observed_at_ms: view.mark_ms,
        market_status: MarketStatus::Active,
        units: UnitStatus::Verified,
        phoenix_initial_margin_usdc: im.as_inner(),
        phoenix_maintenance_margin_usdc: mm.as_inner(),
        total_notional_usdc: total,
        unrealized_pnl_usdc: upnl,
        first_tier_leverage: metadata
            .leverage_tiers
            .get_leverage_constant(math::BaseLots::new(1))
            .as_inner(),
        upnl_gain_factor_bps: Some(metadata.upnl_risk_factor),
        tick_size_in_quote_lots_per_base_lot: metadata.tick_size.as_inner(),
        quote_lot_to_usdc_numerator: 1,
        quote_lot_to_usdc_denominator: 1,
        post_pool_health: None,
    };
    let cash =
        i128::from(ledger.free) + i128::from(ledger.reserved) - i128::from(ledger.bad_debt_usdc);
    RiseRiskEngine
        .quote_user_health(&snapshot, post_lots, cash, unix_ms())
        .map(|q| q.post_position_im_usdc)
        .map_err(|_| RuntimeError::Stale)
}
