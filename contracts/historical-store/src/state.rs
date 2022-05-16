use crate::package::number::Number128;
use cosmwasm_std::{Uint128, Addr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cw_storage_plus::{Map, Item};
use crate::package::types::{PositionDirection, DepositDirection};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum Type {
    Repeg,
    UpdateK,
}

impl Default for Type {
    // UpOnly
    fn default() -> Self {
        Type::Repeg
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CurveRecord {
    pub ts: u64,
    pub market_index: u64,
    pub peg_multiplier_before: Uint128,
    pub peg_multiplier_after: Uint128,
    pub base_asset_reserve_before: Uint128,
    pub base_asset_reserve_after: Uint128,
    pub quote_asset_reserve_before: Uint128,
    pub quote_asset_reserve_after: Uint128,
    pub sqrt_k_before: Uint128,
    pub sqrt_k_after: Uint128,
    pub base_asset_amount_long: Uint128,
    pub base_asset_amount_short: Uint128,
    pub base_asset_amount: Number128,
    pub open_interest: Uint128,
    pub total_fee: Uint128,
    pub total_fee_minus_distributions: Uint128,
    pub adjustment_cost: Number128,
    pub oracle_price: Number128
}

pub const CURVEHISTORY: Map<String,  CurveRecord> = Map::new("curve_history");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DepositRecord {
    pub ts: u64,
    pub user: Addr,
    pub direction: DepositDirection,
    pub collateral_before: Uint128,
    pub cumulative_deposits_before: Uint128,
    pub amount: u64,
}

pub const DEPOSIT_HISTORY: Map<(&Addr, String),  DepositRecord> = Map::new("deposit_history");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FundingPaymentRecord {
    pub ts: u64,
    pub user: Addr,
    pub market_index: u64,
    pub funding_payment: Number128,
    pub base_asset_amount: Number128,
    pub user_last_cumulative_funding: Number128,
    pub user_last_funding_rate_ts: u64,
    pub amm_cumulative_funding_long: Number128,
    pub amm_cumulative_funding_short: Number128,
}

pub const FUNDING_PAYMENT_HISTORY: Map<(&Addr, String),  FundingPaymentRecord> = Map::new("funding_history");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FundingRateRecord {
    pub ts: u64,
    pub market_index: u64,
    pub funding_rate: Number128,
    pub cumulative_funding_rate_long: Number128,
    pub cumulative_funding_rate_short: Number128,
    pub oracle_price_twap: Number128,
    pub mark_price_twap: Uint128,
}

pub const FUNDING_RATE_HISTORY: Map<String,  FundingRateRecord> = Map::new("funding_payment_history");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LiquidationRecord {
    pub ts: u64,
    pub user: Addr,
    pub partial: bool,
    pub base_asset_value: Uint128,
    pub base_asset_value_closed: Uint128,
    pub liquidation_fee: Uint128,
    pub fee_to_liquidator: u64,
    pub fee_to_insurance_fund: u64,
    pub liquidator: Addr,
    pub total_collateral: Uint128,
    pub collateral: Uint128,
    pub unrealized_pnl: Number128,
    pub margin_ratio: Uint128,
}

pub const LIQUIDATION_HISTORY: Map<(&Addr, String),  LiquidationRecord> = Map::new("liquidation_history");

// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
// pub enum OrderAction {
//     Place,
//     Cancel,
//     Fill,
//     Expire,
// }

// impl Default for OrderAction {
//     // UpOnly
//     fn default() -> Self {
//         OrderAction::Place
//     }
// }

// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
// pub struct OrderRecord {
//     pub ts: u64,
//     pub user: Addr,
//     pub order: Order,
//     pub action: OrderAction,
//     pub filler: Addr,
//     pub trade_record_id: u64,
//     pub base_asset_amount_filled: Uint128,
//     pub quote_asset_amount_filled: Uint128,
//     pub fee: Uint128,
//     pub filler_reward: Uint128,
//     pub quote_asset_amount_surplus: Uint128,
//     pub position_index: u64,
// }

// pub const ORDER_HISTORY: Map<String,  OrderRecord> = Map::new("order_history");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TradeRecord {
    pub ts: u64,
    pub user: Addr,
    pub direction: PositionDirection,
    pub base_asset_amount: Uint128,
    pub quote_asset_amount: Uint128,
    pub mark_price_before: Uint128,
    pub mark_price_after: Uint128,
    pub fee: Uint128,
    pub referrer_reward: Uint128,
    pub referee_discount: Uint128,
    pub token_discount: Uint128,
    pub liquidation: bool,
    pub market_index: u64,
    pub oracle_price: Number128,
}

pub const TRADE_HISTORY: Map<(&Addr, String),  TradeRecord> = Map::new("trade_history");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Length {
    pub curve_history_length: u64,
    pub deposit_history_length: u64,
    pub funding_payment_history_length: u64,
    pub funding_rate_history_length: u64,
    pub liquidation_history_length: u64,
    pub trade_history_length: u64,
    // pub order_history_length: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub clearing_house: Addr,
    pub owner: Addr,
}

pub const STATE: Item<State> = Item::new("state");
pub const LENGTH : Item<Length> = Item::new("length");