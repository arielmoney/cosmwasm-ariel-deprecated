use std::cmp::max;

use cosmwasm_std::{Uint128};

use crate::error::ContractError;

use crate::states::market::Market;
use crate::states::user::Position;

use crate::states::constants::{
    AMM_TO_QUOTE_PRECISION_RATIO, FUNDING_PAYMENT_PRECISION, MARK_PRICE_PRECISION,
    QUOTE_TO_BASE_AMT_FUNDING_PRECISION, SHARE_OF_FEES_ALLOCATED_TO_CLEARING_HOUSE_DENOMINATOR,SHARE_OF_FEES_ALLOCATED_TO_CLEARING_HOUSE_NUMERATOR
};

/// With a virtual AMM, there can be an imbalance between longs and shorts and thus funding can be asymmetric.
/// To account for this, amm keeps track of the cumulative funding rate for both longs and shorts.
/// When there is a period with asymmetric funding, the clearing house will pay/receive funding from/to it's collected fees.
pub fn calculate_funding_rate_long_short(
    market: &Market,
    funding_rate: i128,
) -> Result<(i128, i128, Uint128), ContractError> {
    // Calculate the funding payment owed by the net_market_position if funding is not capped
    // If the net market position owes funding payment, the clearing house receives payment
    let net_market_position = market.base_asset_amount.i128().clone();
    let net_market_position_funding_payment =
        calculate_funding_payment_in_quote_precision(funding_rate, net_market_position)?;
    let uncapped_funding_pnl = -net_market_position_funding_payment;

    // If the uncapped_funding_pnl is positive, the clearing house receives money.
    if uncapped_funding_pnl >= 0 {
        let new_total_fee_minus_distributions = market
            .amm
            .total_fee_minus_distributions
            .checked_add(Uint128::from(uncapped_funding_pnl.unsigned_abs()))?;
        return Ok((funding_rate, funding_rate, new_total_fee_minus_distributions));
    }

    let (capped_funding_rate, capped_funding_pnl) =
        calculate_capped_funding_rate(&market, uncapped_funding_pnl, funding_rate)?;

    let new_total_fee_minus_distributions = market
        .amm
        .total_fee_minus_distributions
        .checked_sub(Uint128::from(capped_funding_pnl.unsigned_abs()))?;

    // clearing house is paying part of funding imbalance
    if capped_funding_pnl != 0 {
        let total_fee_minus_distributions_lower_bound = market
            .amm
            .total_fee
            .checked_mul(SHARE_OF_FEES_ALLOCATED_TO_CLEARING_HOUSE_NUMERATOR)?
            .checked_div(SHARE_OF_FEES_ALLOCATED_TO_CLEARING_HOUSE_DENOMINATOR)?;

        // makes sure the clearing house doesn't pay more than the share of fees allocated to `distributions`
        if new_total_fee_minus_distributions.lt(&total_fee_minus_distributions_lower_bound) {
            return Err(ContractError::InvalidFundingProfitability.into());
        }
    }
    
    let funding_rate_long = if funding_rate < 0 {
        capped_funding_rate
    } else {
        funding_rate
    };

    let funding_rate_short = if funding_rate > 0 {
        capped_funding_rate
    } else {
        funding_rate
    };

    return Ok((funding_rate_long, funding_rate_short, new_total_fee_minus_distributions));
}

fn calculate_capped_funding_rate(
    market: &Market,
    uncapped_funding_pnl: i128, // if negative, users would net recieve from clearinghouse
    funding_rate: i128,
) -> Result<(i128, i128), ContractError> {
    // The funding_rate_pnl_limit is the amount of fees the clearing house can use before it hits it's lower bound
    let total_fee_minus_distributions_lower_bound = market
        .amm
        .total_fee
        .checked_mul(SHARE_OF_FEES_ALLOCATED_TO_CLEARING_HOUSE_NUMERATOR)?
        .checked_div(SHARE_OF_FEES_ALLOCATED_TO_CLEARING_HOUSE_DENOMINATOR)?;

    // limit to 2/3 of current fee pool per funding period
    let funding_rate_pnl_limit =
        if market.amm.total_fee_minus_distributions > total_fee_minus_distributions_lower_bound {
            -(market
                    .amm
                    .total_fee_minus_distributions
                    .checked_sub(total_fee_minus_distributions_lower_bound)?
                    .checked_mul(Uint128::from(2 as u32))?
                    .checked_div(Uint128::from(3 as u32))?
                    .u128() as i128)
        } else {
            0
        };

    // if theres enough in fees, give user's uncapped funding
    // if theres a little/nothing in fees, give the user's capped outflow funding
    let capped_funding_pnl = max(uncapped_funding_pnl, funding_rate_pnl_limit);
    let capped_funding_rate = if uncapped_funding_pnl < funding_rate_pnl_limit {
        // Calculate how much funding payment is already available from users
        let funding_payment_from_users = if funding_rate > 0 {
            calculate_funding_payment_in_quote_precision(
                funding_rate,
                market.base_asset_amount_long.i128(),
            )
        } else {
            calculate_funding_payment_in_quote_precision(
                funding_rate,
                market.base_asset_amount_short.i128(),
            )
        }?;

        // increase the funding_rate_pnl_limit by accounting for the funding payment already being made by users
        // this makes it so that the capped rate includes funding payments from users and clearing house collected fees
        let funding_rate_pnl_limit = funding_rate_pnl_limit
            .checked_sub(funding_payment_from_users.abs())
            .ok_or_else(|| (ContractError::MathError))?;

        if funding_rate < 0 {
            // longs receive
            calculate_funding_rate_from_pnl_limit(
                funding_rate_pnl_limit,
                market.base_asset_amount_long.i128(),
            )?
        } else {
            // shorts receive
            calculate_funding_rate_from_pnl_limit(
                funding_rate_pnl_limit,
                market.base_asset_amount_short.i128(),
            )?
        }
    } else {
        funding_rate
    };

    return Ok((capped_funding_rate, capped_funding_pnl));
}

pub fn calculate_funding_payment(
    amm_cumulative_funding_rate: i128,
    market_position: &Position,
) -> Result<i128, ContractError> {
    let funding_rate_delta = amm_cumulative_funding_rate
        .checked_sub(market_position.last_cumulative_funding_rate.i128())
        .ok_or_else(|| (ContractError::MathError))?;

    let funding_rate_payment =
        _calculate_funding_payment(funding_rate_delta, market_position.base_asset_amount.i128())?;

    return Ok(funding_rate_payment);
}

fn _calculate_funding_payment(
    funding_rate_delta: i128,
    base_asset_amount: i128,
) -> Result<i128, ContractError> {
    let funding_rate_delta_sign: i128 = if funding_rate_delta > 0 { 1 } else { -1 };

    let funding_rate_payment_magnitude = funding_rate_delta.unsigned_abs()
            .checked_mul(base_asset_amount.unsigned_abs())
            .ok_or_else(|| (ContractError::MathError))?
            .checked_div(MARK_PRICE_PRECISION.u128())
            .ok_or_else(|| (ContractError::MathError))?
            .checked_div(FUNDING_PAYMENT_PRECISION.u128())
            .ok_or_else(|| (ContractError::MathError))?;

    // funding_rate: longs pay shorts
    let funding_rate_payment_sign: i128 = if base_asset_amount > 0 { -1 } else { 1 };

    let funding_rate_payment = (funding_rate_payment_magnitude as i128)
        .checked_mul(funding_rate_payment_sign)
        .ok_or_else(|| (ContractError::MathError))?
        .checked_mul(funding_rate_delta_sign)
        .ok_or_else(|| (ContractError::MathError))?;

    return Ok(funding_rate_payment);
}

fn calculate_funding_rate_from_pnl_limit(
    pnl_limit: i128,
    base_asset_amount: i128,
) -> Result<i128, ContractError> {
    if base_asset_amount == 0 {
        return Ok(0);
    }

    let pnl_limit_biased = if pnl_limit < 0 {
        pnl_limit.checked_add(1).ok_or_else(|| (ContractError::MathError))?
    } else {
        pnl_limit
    };

    let funding_rate = pnl_limit_biased
        .checked_mul(QUOTE_TO_BASE_AMT_FUNDING_PRECISION.u128() as i128)
        .ok_or_else(|| (ContractError::MathError))?
        .checked_div(base_asset_amount)
        .ok_or_else(|| (ContractError::MathError));

    return funding_rate;
}

fn calculate_funding_payment_in_quote_precision(
    funding_rate_delta: i128,
    base_asset_amount: i128,
) -> Result<i128, ContractError> {
    let funding_payment = _calculate_funding_payment(funding_rate_delta, base_asset_amount)?;
    let funding_payment_collateral = funding_payment
        .checked_div(AMM_TO_QUOTE_PRECISION_RATIO.u128() as i128)
        .ok_or_else(|| (ContractError::MathError))?;

    Ok(funding_payment_collateral)
}
