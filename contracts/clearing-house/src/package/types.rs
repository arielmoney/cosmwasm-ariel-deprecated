use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Decimal, Uint128};

use crate::package::number::Number128;

#[derive(Clone, Debug, JsonSchema, Copy, Serialize, Deserialize, PartialEq)]
pub enum PositionDirection {
    Long,
    Short,
}

impl Default for PositionDirection {
    // UpOnly
    fn default() -> Self {
        PositionDirection::Long
    }
}

#[derive(Clone, Debug, JsonSchema, Copy, Serialize, Deserialize, PartialEq)]
pub enum SwapDirection {
    Add,
    Remove,
}

impl Default for SwapDirection {
    fn default() -> Self {
        SwapDirection::Add
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]

pub enum DepositDirection {
    DEPOSIT,
    WITHDRAW,
}

impl Default for DepositDirection {
    fn default() -> Self {
        DepositDirection::DEPOSIT
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum OracleSource {
    Oracle,
    // Simulated,
    // Zero, 
    // Bank
}

impl Default for OracleSource {
    // UpOnly
    fn default() -> Self {
        OracleSource::Oracle
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OracleStatus {
    pub price_data: OraclePriceData,
    pub oracle_mark_spread_pct: Number128,
    pub is_valid: bool,
    pub mark_too_divergent: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OraclePriceData {
    pub price: Number128,
    pub confidence: Uint128,
    pub delay: i64,
    pub has_sufficient_number_of_data_points: bool,
}



#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Order {
    pub ts: u64,
    pub status: OrderStatus,
    pub order_type: OrderType,
    pub position_index : u64,
    pub market_index: u64,
    pub price: Uint128,
    pub user_base_asset_amount: Number128,
    pub quote_asset_amount: Uint128,
    pub base_asset_amount: Uint128,
    pub base_asset_amount_filled: Uint128,
    pub quote_asset_amount_filled: Uint128,
    pub fee: Uint128,
    pub direction: PositionDirection,
    pub reduce_only: bool,
    pub post_only: bool,
    pub immediate_or_cancel: bool,
    pub discount_tier: OrderDiscountTier,
    pub trigger_price: Uint128,
    pub trigger_condition: OrderTriggerCondition,
    pub referrer: Addr,
    pub oracle_price_offset: Number128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum OrderStatus {
    Init,
    Open,
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum OrderType {
    Market,
    Limit,
    TriggerMarket,
    TriggerLimit,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum OrderTriggerCondition {
    Above,
    Below,
}

impl Default for OrderTriggerCondition {
    // UpOnly
    fn default() -> Self {
        OrderTriggerCondition::Above
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum OrderDiscountTier {
    None,
    First,
    Second,
    Third,
    Fourth,
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FeeStructure {
    pub fee: Decimal,

    pub first_tier_minimum_balance: Uint128,
    pub first_tier_discount: Decimal,

    pub second_tier_minimum_balance: Uint128,
    pub second_tier_discount: Decimal,

    pub third_tier_minimum_balance: Uint128,
    pub third_tier_discount: Decimal,

    pub fourth_tier_minimum_balance: Uint128,
    pub fourth_tier_discount: Decimal,


    pub referrer_reward: Decimal,
    pub referee_discount: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OracleGuardRails {
    pub use_for_liquidations: bool,
    // oracle price divergence rails
    pub mark_oracle_divergence: Decimal,
    // validity guard rails
    pub slots_before_stale: i64,
    pub confidence_interval_max_size: Uint128,
    pub too_volatile_ratio: Number128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OrderParams {
    pub order_type: OrderType,
    pub direction: PositionDirection,
    pub quote_asset_amount: Uint128,
    pub base_asset_amount: Uint128,
    pub price: Uint128,
    pub market_index: u64,
    pub reduce_only: bool,
    pub post_only: bool,
    pub immediate_or_cancel: bool,
    pub trigger_price: Uint128,
    pub trigger_condition: OrderTriggerCondition,
    pub position_limit: Uint128,
    pub oracle_price_offset: Number128,
}