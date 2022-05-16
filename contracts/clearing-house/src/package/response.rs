use cosmwasm_std::{Uint128, Decimal, Addr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::package::{types::{OracleSource, PositionDirection}, number::Number128};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserResponse {
    pub collateral: Uint128,
    pub cumulative_deposits: Uint128,
    pub total_fee_paid: Uint128,
    pub total_token_discount: Uint128,
    pub total_referral_reward: Uint128,
    pub total_referee_discount: Uint128,
    pub referrer: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserPositionResponse {
    pub base_asset_amount: Number128,
    pub quote_asset_amount: Uint128,
    pub last_cumulative_funding_rate: Number128,
    pub last_cumulative_repeg_rebate: Uint128,
    pub last_funding_rate_ts: u64,
    
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateResponse {
    pub admin: Addr,
    pub exchange_paused: bool,
    pub funding_paused: bool,
    pub admin_controls_prices: bool,
    pub collateral_vault: Addr,
    pub insurance_vault: Addr,
    pub history_contract: Addr,
    pub oracle: Addr,
    pub margin_ratio_initial: Uint128,
    pub margin_ratio_maintenance: Uint128,
    pub margin_ratio_partial: Uint128,
    
    pub partial_liquidation_close_percentage: Decimal,
    pub partial_liquidation_penalty_percentage: Decimal,
    pub full_liquidation_penalty_percentage: Decimal,

    pub partial_liquidation_liquidator_share_denominator: u64,
    pub full_liquidation_liquidator_share_denominator: u64,

    pub max_deposit: Uint128,
    pub markets_length: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PositionResponse {
    pub base_asset_amount: Number128,
    pub quote_asset_amount: Uint128,
    pub last_cumulative_funding_rate: Number128,
    pub last_cumulative_repeg_rebate: Uint128,
    pub last_funding_rate_ts: u64,
    pub direction: PositionDirection,
    pub initial_size: Uint128,
    pub entry_notional: Number128,
    pub pnl: Number128
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FeeStructureResponse {
    pub fee: Decimal,
    pub first_tier_minimum_balance: Uint128,
    pub first_tier_discount : Decimal,
    pub second_tier_minimum_balance : Uint128,
    pub second_tier_discount : Decimal,
    pub third_tier_minimum_balance : Uint128,
    pub third_tier_discount : Decimal,
    pub fourth_tier_minimum_balance : Uint128,
    pub fourth_tier_discount : Decimal,
    pub referrer_reward : Decimal,
    pub referee_discount : Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OracleGuardRailsResponse {
    pub use_for_liquidations: bool,
    // oracle price divergence rails
    pub mark_oracle_divergence: Decimal,
    // validity guard rails
    pub slots_before_stale: Number128,
    pub confidence_interval_max_size: Uint128,
    pub too_volatile_ratio: Number128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OrderStateResponse {
    pub min_order_quote_asset_amount: Uint128, // minimum est. quote_asset_amount for place_order to succeed
    pub reward: Decimal,
    pub time_based_reward_lower_bound: Uint128, // minimum filler reward for time-based reward
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MarketLengthResponse {
    pub length: u64,
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MarketInfoResponse {
    pub market_name: String,
    pub initialized: bool,
    pub base_asset_amount_long: Number128,
    pub base_asset_amount_short: Number128,
    pub base_asset_amount: Number128, // net market bias
    pub open_interest: Uint128,
    pub oracle: String,
    pub oracle_source: OracleSource,
    pub base_asset_reserve: Uint128,
    pub quote_asset_reserve: Uint128,
    pub cumulative_repeg_rebate_long: Uint128,
    pub cumulative_repeg_rebate_short: Uint128,
    pub cumulative_funding_rate_long: Number128,
    pub cumulative_funding_rate_short: Number128,
    pub last_funding_rate: Number128,
    pub last_funding_rate_ts: u64,
    pub funding_period: u64,
    pub last_oracle_price_twap: Number128,
    pub last_mark_price_twap: Uint128,
    pub last_mark_price_twap_ts: u64,
    pub sqrt_k: Uint128,
    pub peg_multiplier: Uint128,
    pub total_fee: Uint128,
    pub total_fee_minus_distributions: Uint128,
    pub total_fee_withdrawn: Uint128,
    pub minimum_trade_size: Uint128,
    pub last_oracle_price_twap_ts: u64,
    pub last_oracle_price: Number128,
    pub minimum_base_asset_trade_size: Uint128,
    pub minimum_quote_asset_trade_size: Uint128
}

// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
// pub struct Response {
//     pub length: u64,
// }
