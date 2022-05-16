use cosmwasm_std::{Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, JsonSchema, Clone, Copy)]
pub struct Number128 {
    pub amount: Uint128,
    pub is_positive: bool,
}

impl Number128 {
    pub const fn new(value: i128) -> Self {
        Number128{
            amount: Uint128::new(value.unsigned_abs()),
            is_positive: value.is_positive()
        }
    }
    pub const fn zero() -> Self {
        Number128 { amount: Uint128::zero(), is_positive: true }
    }
    /// Returns a copy of the internal data
    pub const fn i128(&self) -> i128 {
        if self.is_positive {
            self.amount.u128() as i128
        } else {
            -(self.amount.u128() as i128)
        }
    }
}