use crate::package::types::{SwapDirection, PositionDirection};
use cosmwasm_std::Uint128;

use crate::error::ContractError;

use crate::states::constants::{
    MARK_PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO, PRICE_SPREAD_PRECISION, AMM_TIMES_PEG_TO_QUOTE_PRECISION_RATIO,
    AMM_RESERVE_PRECISION, PRICE_TO_QUOTE_PRECISION_RATIO
};
use crate::states::market::Amm;
use crate::states::user::Position;

use super::amm::{self, calculate_quote_asset_amount_swapped};


/// Calculates how much of withdrawal must come from collateral vault and how much comes from insurance vault
pub fn calculate_withdrawal_amounts(
    amount: Uint128,
    balance_collateral: Uint128,
    balance_insurance: Uint128
) -> Result<(Uint128, Uint128), ContractError> {
    return Ok(
        if balance_collateral.u128() >= amount.u128() {
            (amount, Uint128::zero())
        } else if balance_insurance.u128() > amount.u128() - balance_collateral.u128()
        {
            (balance_collateral, amount.checked_sub(balance_collateral)?)
        } else {
            (balance_collateral, balance_insurance)
        }
    );
}

pub fn calculate_updated_collateral(collateral: Uint128, pnl: i128) -> Result<Uint128, ContractError> {
    return Ok(if pnl.is_negative() && pnl.unsigned_abs() > collateral.u128() {
        Uint128::zero()
    } else if pnl > 0 {
        collateral
            .checked_add(Uint128::from(pnl.unsigned_abs()))?
    } else {
        collateral
            .checked_sub(Uint128::from(pnl.unsigned_abs()))?
    });
}


pub fn calculate_slippage(
    exit_value: Uint128,
    base_asset_amount: Uint128,
    mark_price_before: i128,
) -> Result<i128, ContractError> {
    let amm_exit_price = exit_value
        .checked_mul(MARK_PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO)?
        .checked_div(base_asset_amount)?;

    Ok((amm_exit_price.u128() as i128)
        .checked_sub(mark_price_before).unwrap_or(0 as i128))
}

pub fn calculate_slippage_pct(
    slippage: i128,
    mark_price_before: i128,
) -> Result<i128, ContractError> {
    slippage
        .checked_mul(PRICE_SPREAD_PRECISION)
        .ok_or_else(|| (ContractError::MathError))?
        .checked_div(mark_price_before)
        .ok_or_else(|| (ContractError::MathError))
}

pub fn reserve_to_asset_amount(
    quote_asset_reserve: Uint128,
    peg_multiplier: Uint128,
) -> Result<Uint128, ContractError> {
    Ok(quote_asset_reserve
        .checked_mul(peg_multiplier)?
        .checked_div(AMM_TIMES_PEG_TO_QUOTE_PRECISION_RATIO)?
    )
}

pub fn asset_to_reserve_amount(
    quote_asset_amount: Uint128,
    peg_multiplier: Uint128,
) -> Result<Uint128, ContractError> {
    Ok(quote_asset_amount
        .checked_mul(AMM_TIMES_PEG_TO_QUOTE_PRECISION_RATIO)?
        .checked_div(peg_multiplier)?
    )
}


pub fn calculate_base_asset_value_and_pnl(
    market_position: &Position,
    a: &Amm,
) -> Result<(Uint128, i128), ContractError> {
    return _calculate_base_asset_value_and_pnl(
        market_position.base_asset_amount.i128(),
        market_position.quote_asset_amount,
        a,
    );
}

pub fn _calculate_base_asset_value_and_pnl(
    base_asset_amount: i128,
    quote_asset_amount: Uint128,
    a: &Amm,
) -> Result<(Uint128, i128), ContractError> {
    if base_asset_amount == 0 {
        return Ok((Uint128::zero(), (0 as i128)));
    }

    let swap_direction = swap_direction_to_close_position(base_asset_amount);

    let (new_quote_asset_reserve, _new_base_asset_reserve) = amm::calculate_swap_output(
        Uint128::from(base_asset_amount.unsigned_abs()),
        a.base_asset_reserve,
        swap_direction,
        a.sqrt_k,
    )?;

    let base_asset_value = calculate_quote_asset_amount_swapped(
        a.quote_asset_reserve,
        new_quote_asset_reserve,
        swap_direction,
        a.peg_multiplier,
    )?;

    let pnl = calculate_pnl(base_asset_value, quote_asset_amount, swap_direction)?;

    return Ok((base_asset_value, pnl));
}

pub fn calculate_base_asset_value_and_pnl_with_oracle_price(
    market_position: &Position,
    oracle_price: i128,
) -> Result<(Uint128, i128), ContractError> {
    if market_position.base_asset_amount.i128() == 0 {
        return Ok((Uint128::zero(), 0));
    }

    let swap_direction = swap_direction_to_close_position(market_position.base_asset_amount.i128());

    let oracle_price = if oracle_price > 0 {
        Uint128::from(oracle_price.unsigned_abs())
    } else {
        Uint128::zero()
    };

    let base_asset_value = Uint128::from(market_position
        .base_asset_amount.i128()
        .unsigned_abs())
        .checked_mul(oracle_price)?
        .checked_div(AMM_RESERVE_PRECISION * PRICE_TO_QUOTE_PRECISION_RATIO)?;

    let pnl = calculate_pnl(
        base_asset_value,
        market_position.quote_asset_amount,
        swap_direction,
    )?;

    Ok((Uint128::from(base_asset_value), pnl))
}

pub fn direction_to_close_position(base_asset_amount: i128) -> PositionDirection {
    if base_asset_amount > 0 {
        PositionDirection::Short
    } else {
        PositionDirection::Long
    }
}

pub fn swap_direction_to_close_position(base_asset_amount: i128) -> SwapDirection {
    if base_asset_amount >= 0 {
        SwapDirection::Add
    } else {
        SwapDirection::Remove
    }
}

pub fn calculate_pnl(
    exit_value: Uint128,
    entry_value: Uint128,
    swap_direction_to_close: SwapDirection,
) -> Result<i128, ContractError> {
    let exit_value_i128 =  exit_value.u128() as i128;
    let entry_value_i128 = entry_value.u128() as i128;
    Ok(match swap_direction_to_close {
        SwapDirection::Add => exit_value_i128
            .checked_sub(entry_value_i128).ok_or_else(|| (ContractError::MathError {}))?,
        SwapDirection::Remove => entry_value_i128
            .checked_sub(exit_value_i128).ok_or_else(|| (ContractError::MathError {}))?,
    })
}