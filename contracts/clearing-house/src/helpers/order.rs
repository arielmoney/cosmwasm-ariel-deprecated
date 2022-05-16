use crate::error::ContractError;

use crate::package::types::PositionDirection;
use cosmwasm_std::Uint128;

use crate::states::constants::{
    AMM_TO_QUOTE_PRECISION_RATIO, MARK_PRICE_PRECISION,
    MARK_PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO
};

pub fn calculate_quote_asset_amount_for_maker_order(
    base_asset_amount: Uint128,
    limit_price: Uint128,
) -> Result<Uint128, ContractError> {
    let res = base_asset_amount
    .checked_mul(limit_price)?
    .checked_div(MARK_PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO)?;
    Ok(res)
}

pub fn limit_price_satisfied(
    limit_price: Uint128,
    quote_asset_amount: Uint128,
    base_asset_amount: Uint128,
    direction: PositionDirection,
) -> Result<bool, ContractError> {
    let price = quote_asset_amount
        .checked_mul(MARK_PRICE_PRECISION * AMM_TO_QUOTE_PRECISION_RATIO)?
        .checked_div(base_asset_amount)?;

    match direction {
        PositionDirection::Long => {
            if price > limit_price {
                return Ok(false);
            }
        }
        PositionDirection::Short => {
            if price < limit_price {
                return Ok(false);
            }
        }
    }

    Ok(true)
}