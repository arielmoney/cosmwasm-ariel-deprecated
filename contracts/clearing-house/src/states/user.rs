use crate::package::number::Number128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Map;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct User {
    pub collateral: Uint128,
    pub cumulative_deposits: Uint128,
    pub total_fee_paid: Uint128,
    pub total_token_discount: Uint128,
    pub total_referral_reward: Uint128,
    pub total_referee_discount: Uint128,
    pub referrer: Option<Addr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Position {
    pub market_index: u64,
    pub base_asset_amount: Number128,
    pub quote_asset_amount: Uint128,
    pub last_cumulative_funding_rate: Number128,
    pub last_cumulative_repeg_rebate: Uint128,
    pub last_funding_rate_ts: u64,
    pub order_length: u64,
}

pub const USERS: Map<&Addr, User> = Map::new("users");
pub const POSITIONS: Map<(&Addr, String), Position> = Map::new("market_positions");

impl Position {
    pub fn is_for(&self, market_index: u64) -> bool {
        self.market_index == market_index && (self.is_open_position() || self.has_open_order())
    }

    pub fn is_available(&self) -> bool {
        !self.is_open_position() && !self.has_open_order()
    }

    pub fn is_open_position(&self) -> bool {
        self.base_asset_amount.i128() != 0
    }

    pub fn has_open_order(&self) -> bool {
        self.order_length != 0
    }
}
