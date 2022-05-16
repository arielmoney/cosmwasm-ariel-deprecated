use std::ops::Mul;
use crate::package::number::Number128;
use integer_sqrt::IntegerSquareRoot;
use cosmwasm_std::{DepsMut, Uint128};

use crate::error::{ContractError};

use crate::package::types::SwapDirection;

use crate::states::market::{Market, MARKETS};

use crate::helpers::amm::{calculate_quote_asset_amount_swapped, calculate_new_oracle_price_twap};
use crate::states::constants::MARK_PRICE_PRECISION;
use crate::helpers::amm;
use crate::helpers::position::_calculate_base_asset_value_and_pnl;
use crate::helpers::position::asset_to_reserve_amount;

pub fn update_mark_twap(
    deps: &mut DepsMut,
    market_index: u64,
    now: u64,
    precomputed_mark_price: Option<Uint128>,
) -> Result<Uint128, ContractError> {
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    let mark_twap = amm::calculate_new_mark_twap(&market.amm, now, precomputed_mark_price)?;
    market.amm.last_mark_price_twap = mark_twap;
    market.amm.last_mark_price_twap_ts = now;
    MARKETS.update(deps.storage, market_index.to_string(), |_m| -> Result<Market, ContractError> {
        Ok(market)
    })?;
    return Ok(mark_twap);
}

pub fn update_oracle_price_twap(
    deps: &mut DepsMut,
    market_index: u64,
    now: u64,
    oracle_price: i128,
) -> Result<i128, ContractError> {
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    let mut a = market.amm.clone();
    let new_oracle_price_spread = oracle_price
        .checked_sub(a.last_oracle_price_twap.i128())
        .ok_or_else(|| (ContractError::MathError))?;

    // cap new oracle update to 33% delta from twap
    let oracle_price_33pct = oracle_price.checked_div(3).ok_or_else(|| (ContractError::MathError))?;

    let capped_oracle_update_price =
        if new_oracle_price_spread.unsigned_abs() > oracle_price_33pct.unsigned_abs() {
            if oracle_price > a.last_oracle_price_twap.i128() {
                a.last_oracle_price_twap.i128()
                    .checked_add(oracle_price_33pct)
                    .ok_or_else(|| (ContractError::MathError))?
            } else {
                a.last_oracle_price_twap.i128()
                    .checked_sub(oracle_price_33pct)
                    .ok_or_else(|| (ContractError::MathError))?
            }
        } else {
            oracle_price
        };

    // sanity check
    let oracle_price_twap: i128;
    if capped_oracle_update_price > 0 && oracle_price > 0 {
        oracle_price_twap = calculate_new_oracle_price_twap(&a, now, capped_oracle_update_price)?;
        a.last_oracle_price = Number128::new(capped_oracle_update_price);
        a.last_oracle_price_twap = Number128::new(oracle_price_twap);
        a.last_oracle_price_twap_ts = now;
    } else {
        oracle_price_twap = a.last_oracle_price_twap.i128()
    }

    market.amm = a;
    MARKETS.update(deps.storage, market_index.to_string(), |_m| -> Result<Market, ContractError> {
        Ok(market)
    })?;

    Ok(oracle_price_twap)
}

/// To find the cost of adjusting k, compare the the net market value before and after adjusting k
/// Increasing k costs the protocol money because it reduces slippage and improves the exit price for net market position
/// Decreasing k costs the protocol money because it increases slippage and hurts the exit price for net market position
pub fn adjust_k_cost(deps: &mut DepsMut, market_index: u64, new_sqrt_k: Uint128) -> Result<i128, ContractError> {
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    // Find the net market value before adjusting k
    let (current_net_market_value, _) =
        _calculate_base_asset_value_and_pnl(market.base_asset_amount.i128(), Uint128::zero(), &market.amm)?;

    let ratio_scalar = MARK_PRICE_PRECISION;

    let sqrt_k_ratio = new_sqrt_k
        .checked_mul(ratio_scalar)?
        .checked_div(Uint128::from(market.amm.sqrt_k))?;

    // if decreasing k, max decrease ratio for single transaction is 2.5%
    if sqrt_k_ratio
        < ratio_scalar
            .checked_mul(Uint128::from(975 as u64))?
            .checked_div(Uint128::from(1000 as u64))?
    {
        return Err(ContractError::InvalidUpdateK.into());
    }
    let new_sqrt_k_val= new_sqrt_k;
    let new_base_asset_reserve = Uint128::from(market.amm.base_asset_reserve)
        .checked_mul(sqrt_k_ratio)?
        .checked_div(ratio_scalar)?;

        let new_quote_asset_reserve = market.amm.quote_asset_reserve
        .checked_mul(sqrt_k_ratio)?
        .checked_div(ratio_scalar)?;

    market.amm.sqrt_k = new_sqrt_k_val;
    market.amm.base_asset_reserve = new_base_asset_reserve;
    market.amm.quote_asset_reserve = new_quote_asset_reserve;

    let (_new_net_market_value, cost) = _calculate_base_asset_value_and_pnl(
        market.base_asset_amount.i128(),
        current_net_market_value,
        &market.amm,
    )?;

    MARKETS.update(deps.storage, market_index.to_string(), |_m| -> Result<Market, ContractError> {
        Ok(market)
    })?;

    Ok(cost)
}

pub fn swap_quote_asset(
    deps: &mut DepsMut,
    market_index: u64,
    quote_asset_amount: Uint128,
    direction: SwapDirection,
    now: u64,
    precomputed_mark_price: Option<Uint128>,
) -> Result<i128, ContractError> {
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    let a = market.amm.clone();
    update_mark_twap(deps, market_index, now, precomputed_mark_price)?;
    let quote_asset_reserve_amount =
        asset_to_reserve_amount(quote_asset_amount, a.peg_multiplier)?;

    if quote_asset_reserve_amount < a.minimum_quote_asset_trade_size {
        return Err(ContractError::TradeSizeTooSmall);
    }

    let initial_base_asset_reserve = a.base_asset_reserve;
    let (new_base_asset_reserve, new_quote_asset_reserve) = amm::calculate_swap_output(
        quote_asset_reserve_amount,
        a.quote_asset_reserve,
        direction,
        a.sqrt_k,
    )?;

    market.amm.base_asset_reserve = new_base_asset_reserve;
    market.amm.quote_asset_reserve = new_quote_asset_reserve;

    let base_asset_amount = (initial_base_asset_reserve.u128() as i128)
        .checked_sub(new_base_asset_reserve.u128() as i128)
        .ok_or_else(|| (ContractError::MathError4))?;

    MARKETS.update(deps.storage, market_index.to_string(), |_m| -> Result<Market, ContractError> {
        Ok(market)
    })?;

    return Ok(base_asset_amount);
}

pub fn swap_base_asset(
    deps: &mut DepsMut,
    market_index: u64,
    base_asset_swap_amount: Uint128,
    direction: SwapDirection,
    now: u64,
    precomputed_mark_price: Option<Uint128>
) -> Result<Uint128, ContractError> {
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    let a = market.amm.clone();
    
    update_mark_twap(deps, market_index, now, precomputed_mark_price)?;

    let initial_quote_asset_reserve = a.quote_asset_reserve;
    let (new_quote_asset_reserve, new_base_asset_reserve) = amm::calculate_swap_output(
        base_asset_swap_amount,
        a.base_asset_reserve,
        direction,
        a.sqrt_k,
    )?;

    market.amm.base_asset_reserve = new_base_asset_reserve;
    market.amm.quote_asset_reserve = new_quote_asset_reserve;

    MARKETS.update(deps.storage, market_index.to_string(), |_m| -> Result<Market, ContractError> {
        Ok(market)
    })?;

    calculate_quote_asset_amount_swapped(
        initial_quote_asset_reserve,
        new_quote_asset_reserve,
        direction,
        a.peg_multiplier,
    )

}

pub fn move_price(
    deps: &mut DepsMut, 
    market_index: u64,
    base_asset_reserve: Uint128,
    quote_asset_reserve: Uint128,
) -> Result<(), ContractError> {
    let k = base_asset_reserve
        .mul(quote_asset_reserve);

    let mut mark = MARKETS.load(deps.storage, market_index.to_string())?;
    
    mark.amm.base_asset_reserve = base_asset_reserve;
    mark.amm.quote_asset_reserve = quote_asset_reserve;
    mark.amm.sqrt_k = Uint128::from(k.u128().integer_sqrt());

    MARKETS.update(deps.storage, market_index.to_string(), |_m| -> Result<Market, ContractError> {
        Ok(mark)
    })?;
    Ok(())
}
