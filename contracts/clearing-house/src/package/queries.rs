use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetUser {
        user_address: String,
    },
    GetUserMarketPosition {
        user_address: String,
        index: u64,
    },
    GetUserPositions {
        user_address: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    GetOracleGuardRails {},
    GetOrderState {},
    GetFeeStructure {},
    GetMarketInfo {
        market_index: u64,
    },
    GetGlobalState {}
}
