use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Uint128, Addr};
use cw_storage_plus::Item;
// use cw_controllers::Admin;
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub admin: Addr,
    pub clearing_house: Addr,
    pub total_deposit: Uint128,
    pub denom_stable: String
}

pub const STATE: Item<State> = Item::new("state");
// pub const ADMIN: Admin = Admin::new("admin");