use cosmwasm_std::{Uint128, Addr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::package::types::{PositionDirection, DepositDirection};
use crate::package::number::Number128;
use crate::state::{CurveRecord, FundingPaymentRecord, FundingRateRecord, LiquidationRecord, TradeRecord, DepositRecord};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateAdmin {
        new_admin: String,
    },
    UpdateClearingHouse {
        new_house: String,
    },
    RecordCurve {
        c: CurveRecord
    },
    RecordDeposit {
        d: DepositRecord
    },
    RecordFundingPayment {
        f: FundingPaymentRecord
    },
    RecordFundingPaymentsMultiple {
        vecf: Vec<FundingPaymentRecord>
    },
    RecordFundingRate {
        f: FundingRateRecord
    },
    RecordLiquidation {
        l: LiquidationRecord
    },
    RecordTrade {
        t: TradeRecord
    },
    // RecordOrder {
    //     o: OrderRecord
    // },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetConfig {},
    GetLength {},
    GetCurveHistory {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    GetDepositHistory {
        user_address: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    GetFundingPaymentHistory {
        user_address: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    GetFundingRateHistory {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    GetLiquidationHistory {
        user_address: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    GetTradeHistory {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    GetTradeHistoryByAddress {
        user_address: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CurveHistoryResponse {
    pub ts: u64,
    pub market_index: u64,
    pub peg_multiplier_before: Uint128,
    pub base_asset_reserve_before: Uint128,
    pub quote_asset_reserve_before: Uint128,
    pub sqrt_k_before: Uint128,
    pub peg_multiplier_after: Uint128,
    pub base_asset_reserve_after: Uint128,
    pub quote_asset_reserve_after: Uint128,
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DepositHistoryResponse {
    pub ts: u64,
    pub user: String,
    pub direction: DepositDirection,
    pub collateral_before: Uint128,
    pub cumulative_deposits_before: Uint128,
    pub amount: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FundingPaymentHistoryResponse {
    pub ts: u64,
    pub user: String,
    pub market_index: u64,
    pub funding_payment: Number128,
    pub base_asset_amount: Number128,
    pub user_last_cumulative_funding: Number128,
    pub user_last_funding_rate_ts: u64,
    pub amm_cumulative_funding_long: Number128,
    pub amm_cumulative_funding_short: Number128,
}
    
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FundingRateHistoryResponse {
    pub ts: u64,
    pub market_index: u64,
    pub funding_rate: Number128,
    pub cumulative_funding_rate_long: Number128,
    pub cumulative_funding_rate_short: Number128,
    pub oracle_price_twap: Number128,
    pub mark_price_twap: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LiquidationHistoryResponse {
    pub ts: u64,
    pub user: String,
    pub partial: bool,
    pub base_asset_value: Uint128,
    pub base_asset_value_closed: Uint128,
    pub liquidation_fee: Uint128,
    pub fee_to_liquidator: u64,
    pub fee_to_insurance_fund: u64,
    pub liquidator: String,
    pub total_collateral: Uint128,
    pub collateral: Uint128,
    pub unrealized_pnl: Number128,
    pub margin_ratio: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TradeHistoryResponse {
    pub ts: u64,
    pub user: String,
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LengthResponse {
    pub curve_history_length: u64,
    pub deposit_history_length: u64,
    pub funding_payment_history_length: u64,
    pub funding_rate_history_length: u64,
    pub liquidation_history_length: u64,
    // pub order_history_length: u64,
    pub trade_history_length: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub clearing_house: Addr,
    pub owner: Addr,
}