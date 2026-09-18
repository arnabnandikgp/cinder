//! Unsigned native instruction construction, not permission to dispatch.
//! Market accounts must be authenticated against the current native asset map
//! before use. User/pool admission, collateral funding, signing, persistence,
//! simulation and broadcast remain separate coordinator obligations.
use crate::rpc::{Result, RuntimeError};
use crate::transaction::{bytes, key, rise_ix};
use crate::{derive_venue_identity, Operation, OrderState, RuntimeConfig};
use anchor_lang::InstructionData;
use phoenix_rise_accounts::owned::PerpAssetMapOwned;
use phoenix_rise_ix::constants::*;
use phoenix_rise_ix::market_order::{
    create_place_market_order_delegated_ix, MarketOrderDelegatedParams, MarketOrderParams,
};
use phoenix_rise_ix::types::{OrderFlags, SelfTradeBehavior, Side};
use solana_instruction::Instruction;
use std::collections::BTreeSet;

pub(crate) fn guarded_ioc(
    config: &RuntimeConfig,
    operator: [u8; 32],
    cfg: &cinder_vault::Config,
    market: &IocMarketAccounts,
    ioc: Instruction,
    guard: cinder_vault::PhoenixExecutionGuard,
) -> Result<Vec<Instruction>> {
    let mut accounts = vec![
        (operator, true, false),
        (crate::runtime::config_key(), false, false),
        (bytes(&config.phoenix_program)?, false, false),
        (bytes(&config.phoenix_global_config)?, false, false),
        (bytes(&config.phoenix_asset_map)?, false, false),
        (bytes(&config.phoenix_trader)?, false, false),
        (market.orderbook, false, false),
        (
            phoenix_rise_ix::hawkeye::HAWKEYE_PROGRAM_ID.to_bytes(),
            false,
            false,
        ),
        (
            bytes("Sysvar1nstructions1111111111111111111111111")?,
            false,
            false,
        ),
        (cfg.vault_usdc_ata.to_bytes(), false, false),
        (cfg.usdc_mint.to_bytes(), false, false),
        (bytes(&config.phoenix_quote_mint)?, false, false),
    ];
    for account in config
        .global_trader_index
        .iter()
        .chain(&config.active_trader_buffer)
    {
        accounts.push((bytes(account)?, false, false));
    }
    let before = cinder_vault::instruction::GuardPhoenixExecution {
        guard,
        after: false,
    }
    .data();
    let fence = |after| {
        crate::transaction::anchor_ix(
            crate::runtime::vault_id(),
            accounts.clone(),
            if after {
                use sha2::Digest;
                cinder_vault::instruction::FinishPhoenixExecution {
                    guard_hash: sha2::Sha256::digest(&before).into(),
                }
                .data()
            } else {
                before.clone()
            },
        )
    };
    Ok(vec![fence(false), ioc, fence(true)])
}

/// Bind these keys and the asset ID to owner-checked native metadata, not REST
/// labels or a user-supplied orderbook. No isolated-account entrypoint exists.
pub struct IocMarketAccounts {
    cinder_asset_id: u16,
    phoenix_asset_id: u32,
    phoenix_program: [u8; 32],
    phoenix_asset_map: [u8; 32],
    orderbook: [u8; 32],
    spline_collection: [u8; 32],
}

impl IocMarketAccounts {
    /// Decode an account fetched at the configured asset-map address. This
    /// verifies ownership, native layout and the symbol/ID binding, but not
    /// snapshot freshness or the trader's signing authority.
    pub fn from_asset_map(
        config: &RuntimeConfig,
        cinder_asset_id: u16,
        address: [u8; 32],
        owner: [u8; 32],
        data: &[u8],
    ) -> Result<Self> {
        config.validate()?;
        let program = bytes(&config.phoenix_program)?;
        if address != bytes(&config.phoenix_asset_map)? || owner != program {
            return Err(RuntimeError::Identity);
        }
        let mapping = config.market(cinder_asset_id)?;
        let map =
            PerpAssetMapOwned::try_from_account_bytes(data).map_err(|_| RuntimeError::Decode)?;
        if usize::from(map.num_assets) != map.metadata.entries.len()
            || map.metadata.len > map.metadata.capacity
        {
            return Err(RuntimeError::Identity);
        }
        let mut candidates = map.iter().filter(|(symbol, metadata)| {
            **symbol == mapping.symbol
                || metadata.static_market_params.asset_id == mapping.phoenix_asset_id
        });
        let (symbol, metadata) = candidates.next().ok_or(RuntimeError::Incomplete)?;
        if candidates.next().is_some()
            || *symbol != mapping.symbol
            || metadata.static_market_params.asset_id != mapping.phoenix_asset_id
        {
            return Err(RuntimeError::Identity);
        }
        if metadata.risk_params.isolated_only != 0 {
            return Err(RuntimeError::Unsupported);
        }
        let orderbook = metadata.static_market_params.market_account.to_bytes();
        let spline_collection = solana_pubkey::Pubkey::find_program_address(
            &[b"spline", &orderbook],
            &key(&config.phoenix_program)?,
        )
        .0
        .to_bytes();
        Ok(Self {
            cinder_asset_id,
            phoenix_asset_id: mapping.phoenix_asset_id,
            phoenix_program: program,
            phoenix_asset_map: address,
            orderbook,
            spline_collection,
        })
    }

    pub fn orderbook(&self) -> [u8; 32] {
        self.orderbook
    }

    pub fn spline_collection(&self) -> [u8; 32] {
        self.spline_collection
    }
}

/// Build exactly one price-, quote- and slot-bounded cross-margin IOC from a
/// prepared journal operation. Budget presence is not fee/admission proof.
/// This function
/// neither signs nor sends; a successful build is not an admission proof.
/// The operator must be the pool trader's authority/primary position authority
/// (verified by the authoritative Rise view), not a secondary session key.
pub fn build_bounded_ioc(
    config: &RuntimeConfig,
    operation: &Operation,
    operator: [u8; 32],
    market: &IocMarketAccounts,
    current_slot: u64,
) -> Result<Instruction> {
    config.validate()?;
    let intent = &operation.intent;
    intent.validate().map_err(|_| RuntimeError::Configuration)?;
    let budget = operation.execution_budget.ok_or(RuntimeError::Incomplete)?;
    budget.validate().map_err(|_| RuntimeError::Configuration)?;
    let identity = derive_venue_identity(&intent.identity);
    if operation.state != OrderState::Prepared
        || operation.venue_signature.is_some()
        || operation.filled_lots != 0
        || identity.full_hash != operation.operation_id
        || identity != operation.venue_identity
    {
        return Err(RuntimeError::Identity);
    }
    // SDK builders serialize raw u64s, but the native decoder constrains
    // these units. Refuse an instruction that builds but cannot deserialize.
    phoenix_rise_math::BaseLots::new_checked(intent.requested_lots.unsigned_abs())
        .map_err(|_| RuntimeError::Configuration)?;
    phoenix_rise_math::Ticks::new_checked(intent.limit_price_ticks)
        .map_err(|_| RuntimeError::Configuration)?;
    if current_slot == 0 || current_slot > intent.last_valid_slot {
        return Err(RuntimeError::Stale);
    }
    let mapping = config.market(intent.asset_id)?;
    if market.cinder_asset_id != intent.asset_id
        || market.phoenix_asset_id != mapping.phoenix_asset_id
        || market.phoenix_program != bytes(&config.phoenix_program)?
        || market.phoenix_asset_map != bytes(&config.phoenix_asset_map)?
    {
        return Err(RuntimeError::Identity);
    }
    // Fail on incomplete/aliased account lists; never truncate a spill page.
    let mut distinct = BTreeSet::new();
    for account in config
        .global_trader_index
        .iter()
        .chain(&config.active_trader_buffer)
    {
        if !distinct.insert(bytes(account)?) {
            return Err(RuntimeError::Identity);
        }
    }
    for account in [
        bytes(&config.phoenix_program)?,
        bytes(&config.phoenix_global_config)?,
        operator,
        bytes(&config.phoenix_trader)?,
        bytes(&config.phoenix_asset_map)?,
        market.orderbook,
        market.spline_collection,
    ] {
        if account == [0; 32] || !distinct.insert(account) {
            return Err(RuntimeError::Identity);
        }
    }
    let operator = solana_pubkey::Pubkey::new_from_array(operator);
    let order = MarketOrderParams::builder()
        .trader(operator)
        .trader_account(key(&config.phoenix_trader)?)
        .perp_asset_map(key(&config.phoenix_asset_map)?)
        .orderbook(solana_pubkey::Pubkey::new_from_array(market.orderbook))
        .spline_collection(solana_pubkey::Pubkey::new_from_array(
            market.spline_collection,
        ))
        .global_trader_index(
            config
                .global_trader_index
                .iter()
                .map(|s| key(s))
                .collect::<Result<_>>()?,
        )
        .active_trader_buffer(
            config
                .active_trader_buffer
                .iter()
                .map(|s| key(s))
                .collect::<Result<_>>()?,
        )
        .side(if intent.requested_lots > 0 {
            Side::Bid
        } else {
            Side::Ask
        })
        .price_in_ticks(intent.limit_price_ticks)
        .num_base_lots(intent.requested_lots.unsigned_abs())
        .num_quote_lots(budget.max_quote_lots)
        .min_base_lots_to_fill(0)
        .min_quote_lots_to_fill(0)
        .self_trade_behavior(SelfTradeBehavior::Abort)
        .client_order_id(u128::from_le_bytes(
            derive_venue_identity(&intent.identity).venue_oid,
        ))
        .last_valid_slot(intent.last_valid_slot)
        .order_flags(OrderFlags::None)
        .cancel_existing(false)
        .symbol(mapping.symbol.clone())
        .subaccount_index(0)
        .build()
        .map_err(|_| RuntimeError::Configuration)?;
    let params = MarketOrderDelegatedParams::builder()
        .market_order(order)
        .trader_wallet(operator)
        .permission_account(operator)
        .build()
        .map_err(|_| RuntimeError::Configuration)?;
    let mut ix =
        create_place_market_order_delegated_ix(params).map_err(|_| RuntimeError::Configuration)?;
    // The SDK resolves process-global PHOENIX_ENV lazily. Runtime configuration
    // is authoritative instead: replace all three deployment-bound accounts,
    // including the account-group program key, before this instruction escapes.
    let addresses = if key(&config.phoenix_program)? == PROD_PHOENIX_PROGRAM_ID {
        PROD_PHOENIX_INSTRUCTION_ADDRESSES
    } else {
        BETA_PHOENIX_INSTRUCTION_ADDRESSES
    };
    ix.program_id = addresses.program_id;
    ix.accounts[0].pubkey = addresses.program_id;
    ix.accounts[1].pubkey = addresses.log_authority;
    ix.accounts[2].pubkey = addresses.global_configuration;
    Ok(rise_ix(ix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BoundedIntent, ExecutionBudget, Journal, OrderIdentity, OrderKind};
    use phoenix_rise_events::market_events::OrderPacketKind;

    fn budget() -> ExecutionBudget {
        ExecutionBudget {
            max_quote_lots: 1_200_000_000,
            max_fee_usdc: 1_000_000,
        }
    }

    fn prepared(intent: &BoundedIntent) -> Result<Operation> {
        let dir = tempfile::tempdir().unwrap();
        let mut journal = Journal::open(dir.path().join("private/journal.sqlite")).unwrap();
        let operation = journal
            .prepare_intent(intent.clone())
            .map_err(|_| RuntimeError::Configuration)?;
        journal
            .record_execution_budget(&operation.operation_id, budget())
            .map_err(|_| RuntimeError::Configuration)
    }

    // Existing identity/account tests now exercise the budgeted public builder
    // using an actual journal projection, not a synthetic execution plan.
    fn build_bounded_ioc(
        config: &RuntimeConfig,
        intent: &BoundedIntent,
        operator: [u8; 32],
        market: &IocMarketAccounts,
        current_slot: u64,
    ) -> Result<Instruction> {
        super::build_bounded_ioc(config, &prepared(intent)?, operator, market, current_slot)
    }

    // Minimal decoder fixture for the pinned 0.5.6 asset-map layout. It binds
    // market identities only; zero risk/oracle fields are not trading evidence.
    fn asset_map_fixture(isolated: bool, duplicate: bool) -> Vec<u8> {
        use phoenix_rise_accounts::perp_asset_map::{
            PriceComponent, RiskParams, StaticMarketParams,
        };
        let count: u16 = if duplicate { 2 } else { 1 };
        let mut data = vec![0u8; 48 + 1024 * 1584];
        data[..8].copy_from_slice(
            &phoenix_rise_accounts::discriminants::PhoenixAccount::PerpAssetMap.discriminant(),
        );
        data[24..26].copy_from_slice(&count.to_le_bytes());
        data[32..36].copy_from_slice(&u32::from(count).to_le_bytes());
        data[40..48].copy_from_slice(&1024u64.to_le_bytes());
        for index in 0..usize::from(count) {
            let entry = 48 + index * 1584;
            data[entry..entry + 3].copy_from_slice(b"SOL");
            let params = entry + 16 + std::mem::size_of::<PriceComponent>() + 16;
            data[params..params + 32].copy_from_slice(&[11; 32]);
            let risk = params + std::mem::size_of::<StaticMarketParams>() + 64;
            data[risk + std::mem::offset_of!(RiskParams, isolated_only)] = u8::from(isolated);
        }
        data
    }

    fn fixtures(lots: i64) -> (RuntimeConfig, BoundedIntent, IocMarketAccounts) {
        let mut cfg: serde_json::Value =
            serde_json::from_str(include_str!("../../../docs/operator-config.example.json"))
                .unwrap();
        cfg["phoenix_trader"] = crate::runtime::text(&[10; 32]).into();
        let config = serde_json::from_value(cfg).unwrap();
        let intent = BoundedIntent {
            identity: OrderIdentity {
                user_ledger: [21; 32],
                user_pubkey: [22; 32],
                user_nonce: 42,
                client_oid: [23; 16],
                kind: OrderKind::User,
            },
            asset_id: 1,
            requested_lots: lots,
            limit_price_ticks: 9931,
            last_valid_slot: 100,
            post_fail_position_im_usdc: 0,
            created_at_ms: 1000,
        };
        let market = IocMarketAccounts {
            cinder_asset_id: 1,
            phoenix_asset_id: 0,
            phoenix_program: PROD_PHOENIX_PROGRAM_ID.to_bytes(),
            phoenix_asset_map: bytes("2nHGAaEw3D5dd4hVueaUNoygkQFmoeKqRQWnSPqSMFUC").unwrap(),
            orderbook: [11; 32],
            spline_collection: [12; 32],
        };
        (config, intent, market)
    }

    #[test]
    fn native_packet_preserves_bounds_identity_partial_fills_and_cross_only_flags() {
        for lots in [1, -1, i64::from(u32::MAX), -i64::from(u32::MAX)] {
            let (config, intent, market) = fixtures(lots);
            let ix = build_bounded_ioc(&config, &intent, [13; 32], &market, 99).unwrap();
            assert_eq!(
                ix.data[..8],
                phoenix_rise_ix::PhoenixInstruction::PlaceMarketOrderDelegated.discriminant()
            );
            let packet: OrderPacketKind = borsh::from_slice(&ix.data[8..]).unwrap();
            match packet {
                OrderPacketKind::ImmediateOrCancel {
                    side,
                    price_in_ticks,
                    num_base_lots,
                    num_quote_lots,
                    min_base_lots_to_fill,
                    min_quote_lots_to_fill,
                    self_trade_behavior,
                    match_limit,
                    client_order_id,
                    last_valid_slot,
                    order_flags,
                    cancel_existing,
                } => {
                    assert_eq!(
                        side,
                        if lots > 0 {
                            phoenix_rise_math::Side::Bid
                        } else {
                            phoenix_rise_math::Side::Ask
                        }
                    );
                    assert_eq!(price_in_ticks.unwrap().as_inner(), intent.limit_price_ticks);
                    assert_eq!(num_base_lots.as_inner(), lots.unsigned_abs());
                    assert_eq!(num_quote_lots.unwrap().as_inner(), budget().max_quote_lots);
                    assert_eq!(min_base_lots_to_fill.as_inner(), 0);
                    assert_eq!(min_quote_lots_to_fill.as_inner(), 0);
                    assert_eq!(
                        self_trade_behavior,
                        phoenix_rise_events::market_events::SelfTradeBehavior::Abort
                    );
                    assert_eq!(match_limit, None);
                    assert_eq!(
                        client_order_id,
                        derive_venue_identity(&intent.identity).venue_oid
                    );
                    assert_eq!(last_valid_slot, Some(intent.last_valid_slot));
                    assert_eq!(
                        order_flags,
                        phoenix_rise_events::market_events::OrderFlags::from_bits(0)
                    );
                    assert!(!cancel_existing);
                }
                _ => panic!("not an IOC"),
            }
            let signers: Vec<_> = ix
                .accounts
                .iter()
                .filter(|a| a.is_signer)
                .map(|a| a.pubkey.to_bytes())
                .collect();
            assert_eq!(signers, vec![[13; 32]]);
            assert!(ix
                .accounts
                .iter()
                .all(|a| a.pubkey.to_bytes() != intent.identity.user_pubkey
                    && a.pubkey.to_bytes() != intent.identity.user_ledger));
            assert_eq!(ix.accounts[4].pubkey.to_bytes(), [13; 32]);
            assert_eq!(ix.accounts[5].pubkey.to_bytes(), [10; 32]);
        }
    }

    #[test]
    fn new_native_ioc_requires_an_unsubmitted_budgeted_operation() {
        let (config, intent, market) = fixtures(-10);
        let mut operation = prepared(&intent).unwrap();
        assert!(super::build_bounded_ioc(&config, &operation, [13; 32], &market, 99).is_ok());
        for mutation in 0..6 {
            let mut altered = operation.clone();
            match mutation {
                0 => altered.execution_budget = None,
                1 => altered.execution_budget.as_mut().unwrap().max_quote_lots = 0,
                2 => altered.operation_id = [0; 32],
                3 => altered.venue_identity.venue_oid = [0; 16],
                4 => altered.venue_signature = Some([1; 64]),
                _ => altered.filled_lots = -1,
            }
            assert!(super::build_bounded_ioc(&config, &altered, [13; 32], &market, 99).is_err());
        }
        operation.state = OrderState::SubmissionIntent;
        assert!(super::build_bounded_ioc(&config, &operation, [13; 32], &market, 99).is_err());
    }

    #[test]
    fn deployment_and_complete_spill_lists_follow_configuration_not_sdk_environment() {
        for addresses in [
            PROD_PHOENIX_INSTRUCTION_ADDRESSES,
            BETA_PHOENIX_INSTRUCTION_ADDRESSES,
        ] {
            let (mut config, intent, mut market) = fixtures(1);
            config.phoenix_program = addresses.program_id.to_string();
            config.phoenix_global_config = addresses.global_configuration.to_string();
            market.phoenix_program = addresses.program_id.to_bytes();
            config
                .global_trader_index
                .push(crate::runtime::text(&[14; 32]));
            config
                .active_trader_buffer
                .push(crate::runtime::text(&[15; 32]));
            let ix = build_bounded_ioc(&config, &intent, [13; 32], &market, 100).unwrap();
            assert_eq!(ix.program_id.to_bytes(), addresses.program_id.to_bytes());
            assert_eq!(
                ix.accounts[0].pubkey.to_bytes(),
                addresses.program_id.to_bytes()
            );
            assert_eq!(
                ix.accounts[1].pubkey.to_bytes(),
                addresses.log_authority.to_bytes()
            );
            assert_eq!(
                ix.accounts[2].pubkey.to_bytes(),
                addresses.global_configuration.to_bytes()
            );
            for key in [
                [14; 32],
                [15; 32],
                market.orderbook,
                market.spline_collection,
            ] {
                assert!(ix.accounts.iter().any(|a| a.pubkey.to_bytes() == key));
            }
        }
    }

    #[test]
    fn rejects_unbounded_expired_mismatched_or_aliased_inputs() {
        let (mut config, mut intent, mut market) = fixtures(1);
        assert!(build_bounded_ioc(&config, &intent, [13; 32], &market, 101).is_err());
        assert!(build_bounded_ioc(&config, &intent, [13; 32], &market, 0).is_err());
        for lots in [0, i64::MIN, i64::MAX, -i64::MAX] {
            intent.requested_lots = lots;
            assert!(build_bounded_ioc(&config, &intent, [13; 32], &market, 99).is_err());
        }
        intent.requested_lots = 1;
        intent.limit_price_ticks = u64::from(u32::MAX) + 1;
        assert!(build_bounded_ioc(&config, &intent, [13; 32], &market, 99).is_err());
        intent.limit_price_ticks = 0;
        assert!(build_bounded_ioc(&config, &intent, [13; 32], &market, 99).is_err());
        intent.limit_price_ticks = 9931;
        market.phoenix_asset_id = 1;
        assert!(build_bounded_ioc(&config, &intent, [13; 32], &market, 99).is_err());
        market.phoenix_asset_id = 0;
        market.orderbook = bytes(&config.phoenix_global_config).unwrap();
        assert!(build_bounded_ioc(&config, &intent, [13; 32], &market, 99).is_err());
        market.orderbook = [11; 32];
        market.spline_collection = market.orderbook;
        assert!(build_bounded_ioc(&config, &intent, [13; 32], &market, 99).is_err());
        market.spline_collection = [12; 32];
        config.active_trader_buffer = config.global_trader_index.clone();
        assert!(build_bounded_ioc(&config, &intent, [13; 32], &market, 99).is_err());
    }

    #[test]
    fn asset_map_requires_pinned_owner_address_and_valid_native_layout() {
        let (config, _, _) = fixtures(1);
        let address = bytes(&config.phoenix_asset_map).unwrap();
        let owner = bytes(&config.phoenix_program).unwrap();
        for (address, owner, expected) in [
            ([90; 32], owner, RuntimeError::Identity),
            (address, [91; 32], RuntimeError::Identity),
            (address, owner, RuntimeError::Decode),
        ] {
            assert!(
                matches!(IocMarketAccounts::from_asset_map(&config,1,address,owner,&[]),Err(e) if e==expected)
            );
        }
    }

    #[test]
    fn native_metadata_binds_the_market_and_derives_splines_for_each_deployment() {
        for addresses in [
            PROD_PHOENIX_INSTRUCTION_ADDRESSES,
            BETA_PHOENIX_INSTRUCTION_ADDRESSES,
        ] {
            let (mut config, intent, _) = fixtures(1);
            config.phoenix_program = addresses.program_id.to_string();
            config.phoenix_global_config = addresses.global_configuration.to_string();
            let market = IocMarketAccounts::from_asset_map(
                &config,
                1,
                bytes(&config.phoenix_asset_map).unwrap(),
                addresses.program_id.to_bytes(),
                &asset_map_fixture(false, false),
            )
            .unwrap();
            assert_eq!(market.orderbook(), [11; 32]);
            assert_eq!(
                market.spline_collection(),
                solana_pubkey::Pubkey::find_program_address(
                    &[b"spline", &[11; 32]],
                    &key(&config.phoenix_program).unwrap()
                )
                .0
                .to_bytes()
            );
            assert!(build_bounded_ioc(&config, &intent, [13; 32], &market, 99).is_ok());
        }
    }

    #[test]
    fn native_metadata_rejects_isolated_duplicate_missing_and_conflicting_mappings() {
        let (mut config, _, _) = fixtures(1);
        for (isolated, duplicate, expected) in [
            (true, false, RuntimeError::Unsupported),
            (false, true, RuntimeError::Identity),
        ] {
            assert!(
                matches!(IocMarketAccounts::from_asset_map(&config,1,bytes(&config.phoenix_asset_map).unwrap(),bytes(&config.phoenix_program).unwrap(),&asset_map_fixture(isolated,duplicate)),Err(e) if e==expected)
            );
        }
        config.markets[0].symbol = "BTC".into();
        assert!(matches!(
            IocMarketAccounts::from_asset_map(
                &config,
                1,
                bytes(&config.phoenix_asset_map).unwrap(),
                bytes(&config.phoenix_program).unwrap(),
                &asset_map_fixture(false, false)
            ),
            Err(RuntimeError::Identity)
        ));
        config.markets[0].phoenix_asset_id = 1;
        assert!(matches!(
            IocMarketAccounts::from_asset_map(
                &config,
                1,
                bytes(&config.phoenix_asset_map).unwrap(),
                bytes(&config.phoenix_program).unwrap(),
                &asset_map_fixture(false, false)
            ),
            Err(RuntimeError::Incomplete)
        ));
    }

    #[test]
    fn changing_deployment_or_map_cannot_reuse_an_old_market_binding() {
        let (mut config, intent, market) = fixtures(1);
        config.phoenix_program = BETA_PHOENIX_PROGRAM_ID.to_string();
        config.phoenix_global_config = BETA_PHOENIX_GLOBAL_CONFIGURATION.to_string();
        assert!(build_bounded_ioc(&config, &intent, [13; 32], &market, 99).is_err());
        config.phoenix_program = PROD_PHOENIX_PROGRAM_ID.to_string();
        config.phoenix_global_config = PROD_PHOENIX_GLOBAL_CONFIGURATION.to_string();
        config.phoenix_asset_map = crate::runtime::text(&[92; 32]);
        assert!(build_bounded_ioc(&config, &intent, [13; 32], &market, 99).is_err());
    }
}
