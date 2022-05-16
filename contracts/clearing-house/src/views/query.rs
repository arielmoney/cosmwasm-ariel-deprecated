use crate::helpers::position::calculate_base_asset_value_and_pnl;
use crate::helpers::position::direction_to_close_position;
use crate::states::constants::{DEFAULT_LIMIT, MAX_LIMIT};
use crate::states::market::MARKETS;
use crate::states::state::{FEESTRUCTURE, ORACLEGUARDRAILS, ORDERSTATE, STATE};
use crate::states::user::{Position, POSITIONS, USERS};
use crate::ContractError;

use crate::package::helper::addr_validate_to_lower;

use crate::package::number::Number128;
use crate::package::response::*;

use crate::package::types::PositionDirection;
use cosmwasm_std::{Deps, Order, Uint128};
use cw_storage_plus::{Bound, PrimaryKey};

pub fn get_user(deps: Deps, user_address: String) -> Result<Option<UserResponse>, ContractError> {
    let user = USERS.may_load(
        deps.storage,
        &addr_validate_to_lower(deps.api, &user_address)?,
    )?;
    match user {
        Some(user) => {
            let referrer: String;
            if user.referrer.is_none() {
                referrer = "".to_string();
            } else {
                referrer = user.referrer.unwrap().into();
            }
            let ur = UserResponse {
                collateral: user.collateral,
                cumulative_deposits: user.cumulative_deposits,
                total_fee_paid: user.total_fee_paid,
                total_token_discount: user.total_token_discount,
                total_referral_reward: user.total_referral_reward,
                total_referee_discount: user.total_token_discount,
                referrer,
            };
            Ok(Some(ur))
        }
        None => Ok(None),
    }
}

pub fn get_user_position(
    deps: Deps,
    user_address: String,
    index: u64,
) -> Result<Option<PositionResponse>, ContractError> {
    let position = POSITIONS.may_load(
        deps.storage,
        (
            &addr_validate_to_lower(deps.api, &user_address)?,
            index.to_string(),
        ),
    )?;
    match position{
        Some(position) => {
            if position.base_asset_amount.i128().unsigned_abs() == 0 {
                return Err(ContractError::UserHasNoPositionInMarket {});
            }
            let mut direction = direction_to_close_position(position.base_asset_amount.i128());
            if direction == PositionDirection::Long {
                direction = PositionDirection::Short;
            } else {
                direction = PositionDirection::Long;
            }
        
            let entry_notional = position.quote_asset_amount;
            let unrealized_pnl = calculate_unrealized_pnl(&deps, position.clone()).unwrap();
            let upr = PositionResponse {
                direction,
                initial_size: Uint128::from(position.base_asset_amount.i128().unsigned_abs()),
                entry_notional: Number128::new(entry_notional.u128() as i128),
                pnl: unrealized_pnl,
                base_asset_amount: position.base_asset_amount,
                quote_asset_amount: position.quote_asset_amount,
                last_cumulative_funding_rate: position.last_cumulative_funding_rate,
                last_cumulative_repeg_rebate: position.last_cumulative_repeg_rebate,
                last_funding_rate_ts: position.last_funding_rate_ts,
            };
            Ok(Some(upr))
        },
        None => Ok(None),
    }
    
}

pub fn get_market_length(deps: Deps) -> Result<MarketLengthResponse, ContractError> {
    let state = STATE.load(deps.storage)?;
    // let length = MarketLengthResponse {
    //     length: state.markets_length,
    // };
    Ok(MarketLengthResponse {
        length: state.markets_length,
    })
}

pub fn get_oracle_guard_rails(deps: Deps) -> Result<OracleGuardRailsResponse, ContractError> {
    let oracle_guard_rails = ORACLEGUARDRAILS.load(deps.storage)?;
    let ogr = OracleGuardRailsResponse {
        use_for_liquidations: oracle_guard_rails.use_for_liquidations,
        mark_oracle_divergence: oracle_guard_rails.mark_oracle_divergence,
        slots_before_stale: Number128::new(oracle_guard_rails.slots_before_stale as i128),
        confidence_interval_max_size: oracle_guard_rails.confidence_interval_max_size,
        too_volatile_ratio: oracle_guard_rails.too_volatile_ratio,
    };
    Ok(ogr)
}

pub fn get_order_state(deps: Deps) -> Result<OrderStateResponse, ContractError> {
    let orderstate = ORDERSTATE.load(deps.storage)?;
    let os = OrderStateResponse {
        min_order_quote_asset_amount: orderstate.min_order_quote_asset_amount,
        reward: orderstate.reward,
        time_based_reward_lower_bound: orderstate.time_based_reward_lower_bound,
    };
    Ok(os)
}

pub fn get_fee_structure(deps: Deps) -> Result<FeeStructureResponse, ContractError> {
    let fs = FEESTRUCTURE.load(deps.storage)?;
    let res = FeeStructureResponse {
        fee: fs.fee,
        first_tier_minimum_balance: fs.first_tier_minimum_balance,
        first_tier_discount: fs.first_tier_discount,
        second_tier_minimum_balance: fs.second_tier_minimum_balance,
        second_tier_discount: fs.second_tier_discount,
        third_tier_minimum_balance: fs.third_tier_minimum_balance,
        third_tier_discount: fs.third_tier_discount,
        fourth_tier_minimum_balance: fs.fourth_tier_minimum_balance,
        fourth_tier_discount: fs.fourth_tier_discount,
        referrer_reward: fs.referrer_reward,
        referee_discount: fs.referee_discount,
    };
    Ok(res)
}

pub fn get_market_info(deps: Deps, market_index: u64) -> Result<MarketInfoResponse, ContractError> {
    let market = MARKETS.load(deps.storage, market_index.to_string())?;
    let market_info = MarketInfoResponse {
        market_name: market.market_name,
        initialized: market.initialized,
        base_asset_amount_long: market.base_asset_amount_long,
        base_asset_amount_short: market.base_asset_amount_short,
        base_asset_amount: market.base_asset_amount,
        open_interest: market.open_interest,
        oracle: market.amm.oracle.into(),
        oracle_source: market.amm.oracle_source,
        base_asset_reserve: market.amm.base_asset_reserve,
        quote_asset_reserve: market.amm.quote_asset_reserve,
        cumulative_repeg_rebate_long: market.amm.cumulative_repeg_rebate_long,
        cumulative_repeg_rebate_short: market.amm.cumulative_repeg_rebate_short,
        cumulative_funding_rate_long: market.amm.cumulative_funding_rate_long,
        cumulative_funding_rate_short: market.amm.cumulative_funding_rate_short,
        last_funding_rate: market.amm.last_funding_rate,
        last_funding_rate_ts: market.amm.last_funding_rate_ts,
        funding_period: market.amm.funding_period,
        last_oracle_price_twap: market.amm.last_oracle_price_twap,
        last_mark_price_twap: market.amm.last_mark_price_twap,
        last_mark_price_twap_ts: market.amm.last_mark_price_twap_ts,
        sqrt_k: market.amm.sqrt_k,
        peg_multiplier: market.amm.peg_multiplier,
        total_fee: market.amm.total_fee,
        total_fee_minus_distributions: market.amm.total_fee_minus_distributions,
        total_fee_withdrawn: market.amm.total_fee_withdrawn,
        minimum_trade_size: Uint128::from(100000000 as u64),
        last_oracle_price_twap_ts: market.amm.last_oracle_price_twap_ts,
        last_oracle_price: market.amm.last_oracle_price,
        minimum_base_asset_trade_size: market.amm.minimum_base_asset_trade_size,
        minimum_quote_asset_trade_size: market.amm.minimum_quote_asset_trade_size,
    };
    Ok(market_info)
}

// get list in response
pub fn get_active_positions(
    deps: Deps,
    user_address: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<PositionResponse>, ContractError> {
    let user_addr = addr_validate_to_lower(deps.api, user_address.as_str())?;

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after
        .map(|start| start.joined_key())
        .map(Bound::Exclusive);

    let active_positions: Vec<PositionResponse> = POSITIONS
        .prefix(&user_addr)
        .range(deps.storage, start, None, Order::Ascending)
        .filter_map(|positions| {
            positions.ok().map(|position| PositionResponse {
                base_asset_amount: position.1.base_asset_amount,
                quote_asset_amount: position.1.quote_asset_amount,
                last_cumulative_funding_rate: position.1.last_cumulative_funding_rate,
                last_cumulative_repeg_rebate: position.1.last_cumulative_repeg_rebate,
                last_funding_rate_ts: position.1.last_funding_rate_ts,
                direction: if position.1.base_asset_amount.i128() > 0 {
                    PositionDirection::Long
                } else {
                    PositionDirection::Short
                },
                initial_size: Uint128::from(position.1.base_asset_amount.i128().unsigned_abs()),
                entry_notional: Number128::new(position.1.quote_asset_amount.u128() as i128),
                pnl: calculate_unrealized_pnl(&deps, position.1).unwrap(),
            })
        })
        .take(limit)
        .collect();

    Ok(active_positions)
}

pub fn calculate_unrealized_pnl(deps: &Deps, m: Position) -> Result<Number128, ContractError> {
    let mut unrealized_pnl: i128 = 0;
    // let m = POSITIONS.load(deps.storage, (user_addr, n.to_string()))?;

    let market = MARKETS.load(deps.storage, m.market_index.to_string())?;
    let a = &market.amm;
    let (_, amm_position_unrealized_pnl) = calculate_base_asset_value_and_pnl(&m, a)?;
    unrealized_pnl = unrealized_pnl
        .checked_add(amm_position_unrealized_pnl)
        .ok_or_else(|| (ContractError::HelpersError))?;
    Ok(Number128::new(unrealized_pnl))
}

pub fn get_global_state(deps: Deps) -> Result<StateResponse, ContractError> {
    let state = STATE.load(deps.storage)?;
    let os = StateResponse {
        admin: state.admin,
        exchange_paused: state.exchange_paused,
        funding_paused: state.funding_paused,
        admin_controls_prices: state.admin_controls_prices,
        collateral_vault: state.collateral_vault,
        insurance_vault: state.insurance_vault,
        history_contract: state.history_contract,
        oracle: state.oracle,
        margin_ratio_initial: state.margin_ratio_initial,
        margin_ratio_maintenance: state.margin_ratio_maintenance,
        margin_ratio_partial: state.margin_ratio_partial,
        partial_liquidation_close_percentage: state.partial_liquidation_close_percentage,
        partial_liquidation_penalty_percentage: state.partial_liquidation_penalty_percentage,
        full_liquidation_penalty_percentage: state.full_liquidation_penalty_percentage,
        partial_liquidation_liquidator_share_denominator: state
            .partial_liquidation_liquidator_share_denominator,
        full_liquidation_liquidator_share_denominator: state
            .full_liquidation_liquidator_share_denominator,
        max_deposit: state.max_deposit,
        markets_length: state.markets_length,
    };
    Ok(os)
}
