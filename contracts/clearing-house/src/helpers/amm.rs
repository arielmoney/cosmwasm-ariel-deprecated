use std::cmp::{max, min};
use integer_sqrt::IntegerSquareRoot;

use crate::error::ContractError;

use crate::package::types::{OracleGuardRails, SwapDirection, PositionDirection, OraclePriceData};
use cosmwasm_std::{Fraction, Uint128};

use crate::states::market::{Market, Amm};

use crate::states::constants::{PEG_PRECISION, PRICE_TO_PEG_PRECISION_RATIO,MARK_PRICE_PRECISION, PRICE_SPREAD_PRECISION, PRICE_SPREAD_PRECISION_U128};
use crate::helpers::position::{reserve_to_asset_amount, asset_to_reserve_amount};

pub fn calculate_price(
    quote_asset_reserve: Uint128,
    base_asset_reserve: Uint128,
    peg_multiplier: Uint128,
) -> Result<Uint128, ContractError> {
    let peg_quote_asset_amount = quote_asset_reserve
        .checked_mul(peg_multiplier)?;

    let res = peg_quote_asset_amount.checked_mul(PRICE_TO_PEG_PRECISION_RATIO)?.checked_div(base_asset_reserve)?;

    Ok(res)
}

pub fn calculate_terminal_price(market: &mut Market) -> Result<Uint128, ContractError> {
    let swap_direction = if market.base_asset_amount.i128() > 0 {
        SwapDirection::Add
    } else {
        SwapDirection::Remove
    };
    let (new_quote_asset_amount, new_base_asset_amount) = calculate_swap_output(
        Uint128::from(market.base_asset_amount.i128().unsigned_abs()),
        market.amm.base_asset_reserve,
        swap_direction,
        market.amm.sqrt_k,
    )?;

    let terminal_price = calculate_price(
        new_quote_asset_amount,
        new_base_asset_amount,
        market.amm.peg_multiplier,
    )?;

    Ok(terminal_price)
}

pub fn calculate_new_mark_twap(
    a: &Amm,
    now: u64,
    precomputed_mark_price: Option<Uint128>,
) -> Result<Uint128, ContractError> {
    let since_last = max(
        1,
        (now as i64).checked_sub(a.last_mark_price_twap_ts as i64)
            .ok_or_else(|| (ContractError::MathError5))?,
    );
    let from_start = max(
        1,
        (a.funding_period as i64)
            .checked_sub(since_last as i64)
            .ok_or_else(|| (ContractError::MathError6))?,
    );
    let current_price = match precomputed_mark_price {
        Some(mark_price) => mark_price,
        None => get_mark_price(&a)?,
    };

    let new_twap = (calculate_twap(
        current_price.u128() as i128,
        a.last_mark_price_twap.u128() as i128,
        since_last as i128,
        from_start as i128,
    )?).unsigned_abs();

    return Ok(Uint128::from(new_twap));
}

pub fn calculate_new_oracle_price_twap(
    a: &Amm,
    now: u64,
    oracle_price: i128,
) -> Result<i128, ContractError> {
    let since_last = max(
        1,
        (now as i64).checked_sub(a.last_oracle_price_twap_ts as i64)
            .ok_or_else(|| (ContractError::MathError))?,
    );

    let from_start = max(
        1,
        (a.funding_period as i64)
            .checked_sub(since_last as i64)
            .ok_or_else(|| (ContractError::MathError))?,
        );

    // ensure amm.last_oracle_price is proper
    // let capped_last_oracle_price = if a.last_oracle_price > 0 {
    //     a.last_oracle_price
    // } else {
    //     oracle_price
    // };

    // nudge last_oracle_price up to .1% toward oracle price
    // let capped_last_oracle_price_10bp = capped_last_oracle_price
    // .checked_div(1000)
    // .ok_or_else(|| (ContractError::MathError))?;

    // let interpolated_oracle_price = min(
    //     capped_last_oracle_price
    //         .checked_add(capped_last_oracle_price_10bp)
    //         .ok_or_else(|| (ContractError::MathError))?,
    //     max(
    //         capped_last_oracle_price
    //             .checked_sub(capped_last_oracle_price_10bp)
    //             .ok_or_else(|| (ContractError::MathError))?,
    //         oracle_price,
    //     ),
    // );

    let new_twap = calculate_twap(
        oracle_price,
        a.last_oracle_price_twap.i128(),
        since_last as i128,
        from_start as i128,
    )?;

    return Ok(new_twap);
}

pub fn calculate_twap(
    new_data: i128,
    old_data: i128,
    new_weight: i128,
    old_weight: i128,
) -> Result<i128, ContractError> {
    let denominator = new_weight
        .checked_add(old_weight)
        .ok_or_else(|| (ContractError::MathError))?;
    let prev_twap_99 = old_data.checked_mul(old_weight).ok_or_else(|| (ContractError::MathError))?;
    let latest_price_01 = new_data.checked_mul(new_weight).ok_or_else(|| (ContractError::MathError))?;
    let new_twap = prev_twap_99
        .checked_add(latest_price_01)
        .ok_or_else(|| (ContractError::MathError))?
        .checked_div(denominator)
        .ok_or_else(|| (ContractError::MathError));
    return new_twap;
}

pub fn calculate_swap_output(
    swap_amount: Uint128,
    input_asset_amount: Uint128,
    direction: SwapDirection,
    invariant_sqrt: Uint128,
) -> Result<(Uint128, Uint128), ContractError> {
    let invariant = invariant_sqrt
        .checked_mul(invariant_sqrt)?;

    if direction == SwapDirection::Remove && swap_amount > input_asset_amount {
        return Err(ContractError::TradeSizeTooLarge);
    }

    let new_input_amount = if let SwapDirection::Add = direction {
        input_asset_amount
            .checked_add(swap_amount)?
    } else {
        input_asset_amount
            .checked_sub(swap_amount)?
    };

    let new_output_amount = invariant
        .checked_div(new_input_amount)?;

    return Ok((new_output_amount, new_input_amount));
}

pub fn calculate_quote_asset_amount_swapped(
    quote_asset_reserve_before: Uint128,
    quote_asset_reserve_after: Uint128,
    swap_direction: SwapDirection,
    peg_multiplier: Uint128,
) -> Result<Uint128, ContractError> {
    let quote_asset_reserve_change = match swap_direction {
        SwapDirection::Add => quote_asset_reserve_before
            .checked_sub(quote_asset_reserve_after)?,

        SwapDirection::Remove => quote_asset_reserve_after
            .checked_sub(quote_asset_reserve_before)?,
    };

    let mut quote_asset_amount =
    reserve_to_asset_amount(quote_asset_reserve_change, peg_multiplier)?;

    // when a user goes long base asset, make the base asset slightly more expensive
    // by adding one unit of quote asset
    if swap_direction == SwapDirection::Remove {
        quote_asset_amount = quote_asset_amount
            .checked_add(Uint128::from(1 as u64))?;
    }

    Ok(quote_asset_amount)
}


pub fn normalise_oracle_price(
    a: &Amm,
    oracle_price: &OraclePriceData,
    precomputed_mark_price: Option<Uint128>,
) -> Result<i128, ContractError> {
    let OraclePriceData {
        price: oracle_price,
        confidence: oracle_conf,
        ..
    } = *oracle_price;

    let mark_price = match precomputed_mark_price {
        Some(mark_price) => mark_price.u128() as i128,
        None => a.mark_price()?.u128() as i128,
    };

    let mark_price_1bp = mark_price.checked_div(10000).ok_or_else(|| (ContractError::MathError))?;
    let conf_int = oracle_conf.u128() as i128;

    //  normalises oracle toward mark price based on the oracle’s confidence interval
    //  if mark above oracle: use oracle+conf unless it exceeds .9999 * mark price
    //  if mark below oracle: use oracle-conf unless it less than 1.0001 * mark price
    //  (this guarantees more reasonable funding rates in volatile periods)
    let normalised_price = if mark_price > oracle_price.i128() {
        min(
            max(
                mark_price
                    .checked_sub(mark_price_1bp)
                    .ok_or_else(|| (ContractError::MathError))?,
                oracle_price.i128(),
            ),
            oracle_price.i128()
                .checked_add(conf_int)
                .ok_or_else(|| (ContractError::MathError))?,
        )
    } else {
        max(
            min(
                mark_price
                    .checked_add(mark_price_1bp)
                    .ok_or_else(|| (ContractError::MathError))?,
                oracle_price.i128(),
            ),
            oracle_price.i128()
                .checked_sub(conf_int)
                .ok_or_else(|| (ContractError::MathError))?,
        )
    };

    Ok(normalised_price)
}


pub fn calculate_oracle_mark_spread(
    a: &Amm,
    oracle_price_data: &OraclePriceData,
    precomputed_mark_price: Option<Uint128>,
) -> Result<(i128, i128), ContractError> {
    let mark_price = match precomputed_mark_price {
        Some(mark_price) => mark_price.u128() as i128,
        None => a.mark_price()?.u128() as i128,
    };

    let oracle_price = oracle_price_data.price.i128();

    let price_spread = mark_price
        .checked_sub(oracle_price)
        .ok_or_else(|| (ContractError::MathError))?;

    Ok((oracle_price, price_spread))

}

pub fn calculate_oracle_mark_spread_pct(
    a: &Amm,
    oracle_price_data: &OraclePriceData,
    precomputed_mark_price: Option<Uint128>,
) -> Result<i128, ContractError> {
    let (oracle_price, price_spread) =
        calculate_oracle_mark_spread(a, oracle_price_data, precomputed_mark_price)?;

    price_spread
        .checked_mul(PRICE_SPREAD_PRECISION)
        .ok_or_else(|| (ContractError::MathError))?
        .checked_div(oracle_price)
        .ok_or_else(|| (ContractError::MathError))
}

pub fn is_oracle_mark_too_divergent(
    price_spread_pct: i128,
    oracle_guard_rails: &OracleGuardRails,
) -> Result<bool, ContractError> {
    let max_divergence = oracle_guard_rails
        .mark_oracle_divergence.numerator()
        .checked_mul(PRICE_SPREAD_PRECISION_U128.u128())
        .ok_or_else(|| (ContractError::MathError))?
        .checked_div(oracle_guard_rails.mark_oracle_divergence.denominator())
        .ok_or_else(|| (ContractError::MathError))?;

    // Ok(max_divergence.lt(&Uint128::from(price_spread_pct.unsigned_abs())))
    Ok(Uint128::from(price_spread_pct.unsigned_abs()).gt(&Uint128::from(max_divergence)))
}

pub fn calculate_mark_twap_spread_pct(a: &Amm, mark_price: Uint128) -> Result<i128, ContractError> {
    let mark_price = mark_price.u128() as i128;
    let mark_twap = a.last_mark_price_twap.u128() as i128;

    let price_spread = mark_price
        .checked_sub(mark_twap)
        .ok_or_else(|| (ContractError::MathError))?;

    price_spread
        .checked_mul(PRICE_SPREAD_PRECISION)
        .ok_or_else(|| (ContractError::MathError))?
        .checked_div(mark_twap)
        .ok_or_else(|| (ContractError::MathError))
}

pub fn use_oracle_price_for_margin_calculation(
    price_spread_pct: i128,
    oracle_guard_rails: &OracleGuardRails,
) -> Result<bool, ContractError> {
    let max_divergence = oracle_guard_rails
        .mark_oracle_divergence.numerator()
        .checked_mul(PRICE_SPREAD_PRECISION_U128.u128())
        .ok_or_else(|| (ContractError::MathError))?
        .checked_div(3)
        .ok_or_else(|| (ContractError::MathError))?
        .checked_div(oracle_guard_rails.mark_oracle_divergence.denominator())
        .ok_or_else(|| (ContractError::MathError))?;

    Ok(price_spread_pct.unsigned_abs() > max_divergence)
}


pub fn is_oracle_valid(
    a: &Amm,
    oracle_price_data: &OraclePriceData,
    valid_oracle_guard_rails: &OracleGuardRails,
) -> Result<bool, ContractError> {
    let OraclePriceData {
        price: oracle_price,
        confidence: oracle_conf,
        delay: oracle_delay,
        has_sufficient_number_of_data_points,
        ..
    } = *oracle_price_data;

    let is_oracle_price_nonpositive = oracle_price.i128() <= 0;

    let is_oracle_price_too_volatile = ((oracle_price.i128()
        .checked_div(max(1, a.last_oracle_price_twap.i128()))
        .ok_or_else(|| (ContractError::MathError))?)
    .gt(&valid_oracle_guard_rails.too_volatile_ratio.i128()))
        || ((a
            .last_oracle_price_twap.i128()
            .checked_div(max(1, oracle_price.i128()))
            .ok_or_else(|| (ContractError::MathError))?)
        .gt(&valid_oracle_guard_rails.too_volatile_ratio.i128()));

    let conf_denom_of_price = Uint128::from(oracle_price.i128().unsigned_abs())
        .checked_div(Uint128::from(max(1 as u128, oracle_conf.u128())))?;

    let is_conf_too_large =
        conf_denom_of_price.lt(&valid_oracle_guard_rails.confidence_interval_max_size);

    let is_stale = oracle_delay.gt(&valid_oracle_guard_rails.slots_before_stale);

    Ok(!(is_stale
        || !has_sufficient_number_of_data_points
        || is_oracle_price_nonpositive
        || is_oracle_price_too_volatile
        || is_conf_too_large))
}

pub fn calculate_max_base_asset_amount_to_trade(
    amm: &Amm,
    limit_price: Uint128,
) -> Result<(Uint128, PositionDirection), ContractError> {
    let invariant = amm.sqrt_k
        .checked_mul(amm.sqrt_k)?;

    let new_base_asset_reserve_squared = invariant
        .checked_mul(MARK_PRICE_PRECISION)?
        .checked_div(limit_price)?
        .checked_mul(amm.peg_multiplier)?
        .checked_div(PEG_PRECISION)?;

    let new_base_asset_reserve = new_base_asset_reserve_squared.u128().integer_sqrt();

    if new_base_asset_reserve > amm.base_asset_reserve.u128() {
        let max_trade_amount = Uint128::from(new_base_asset_reserve)
            .checked_sub(amm.base_asset_reserve)?;
        Ok((max_trade_amount, PositionDirection::Short))
    } else {
        let max_trade_amount = amm
            .base_asset_reserve
            .checked_sub(Uint128::from(new_base_asset_reserve))?;
        Ok((max_trade_amount, PositionDirection::Long))
    }
}

pub fn should_round_trade(
    a: &Amm,
    quote_asset_amount: Uint128,
    base_asset_value: Uint128,
) -> Result<bool, ContractError> {
    let difference = if quote_asset_amount > base_asset_value {
        quote_asset_amount
            .checked_sub(base_asset_value)?
    } else {
        base_asset_value
            .checked_sub(quote_asset_amount)?
    };

    let quote_asset_reserve_amount = asset_to_reserve_amount(difference, a.peg_multiplier)?;

    Ok(quote_asset_reserve_amount < a.minimum_quote_asset_trade_size)
}

pub fn get_mark_price(a: &Amm) -> Result<Uint128, ContractError> {
    calculate_price(
        a.quote_asset_reserve,
        a.base_asset_reserve,
        a.peg_multiplier,
    )
}
