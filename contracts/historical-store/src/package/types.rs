use cosmwasm_std::{Uint128, Addr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::number::Number128;

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
pub enum OrderDiscountTier {
    None,
    First,
    Second,
    Third,
    Fourth,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum OrderTriggerCondition {
    Above,
    Below,
}

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
