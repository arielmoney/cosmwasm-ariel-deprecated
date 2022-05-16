use crate::package::types::{Order};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::ContractError;

use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OrderInfo {
    pub len: u64,
}

pub const ORDERS: Map<((&Addr, String), String), Order> = Map::new("orders");
pub const ORDERS_INFO: Item<OrderInfo> = Item::new("order_info");

pub fn has_oracle_price_offset(oo: &Order) -> bool {
    oo.oracle_price_offset.i128() != 0
}

pub fn get_limit_price(
    oo: &Order,
    valid_oracle_price: Option<i128>,
) -> Result<Uint128, ContractError> {
    // the limit price can be hardcoded on order or derived from oracle_price + oracle_price_offset
    let price = if has_oracle_price_offset(oo) {
        if let Some(oracle_price) = valid_oracle_price {
            let limit_price = oracle_price
                .checked_add(oo.oracle_price_offset.i128())
                .ok_or_else(|| (ContractError::MathError))?;

            if limit_price <= 0 {
                return Err(ContractError::InvalidOracleOffset);
            }

            Uint128::from(limit_price.unsigned_abs())
        } else {
            return Err(ContractError::OracleNotFoundToOffset);
        }
    } else {
        oo.price
    };
    Ok(price)
}
