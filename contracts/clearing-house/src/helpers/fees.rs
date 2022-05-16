use std::cmp::{max, min};

use cosmwasm_std::{Addr, Uint128, Fraction, Decimal};

use crate::states::state::OrderState;
use crate::{error::ContractError};


use integer_sqrt::IntegerSquareRoot;

use crate::package::types::{FeeStructure, OrderDiscountTier};

pub fn calculate_fee_for_trade(
    quote_asset_amount: Uint128,
    fee_structure: &FeeStructure,
    discount_token_amt: Uint128,
    referrer: &Option<Addr>,
) -> Result<(Uint128, Uint128, Uint128, Uint128, Uint128), ContractError> {
    let fee = quote_asset_amount
        .checked_mul(Uint128::from(fee_structure.fee.numerator()))?
        .checked_div(Uint128::from(fee_structure.fee.denominator()))?;

    let token_discount = calculate_token_discount(fee, fee_structure, discount_token_amt)?;

    let (referrer_reward, referee_discount) =
        calculate_referral_reward_and_referee_discount(fee, fee_structure, referrer)?;

    let user_fee = fee
        .checked_sub(token_discount)?
        .checked_sub(referee_discount)?;

    let fee_to_market = user_fee
        .checked_sub(referrer_reward)?;

    return Ok((
        user_fee,
        fee_to_market,
        token_discount,
        referrer_reward,
        referee_discount,
    ));
}

fn calculate_token_discount(
    fee: Uint128,
    fee_structure: &FeeStructure,
    discount_token_amt: Uint128,
) -> Result<Uint128, ContractError> {
    if discount_token_amt.is_zero() {
        return Ok(Uint128::zero());
    }

    if let Some(discount) =
        calculate_token_discount_for_tier(fee, fee_structure.first_tier_minimum_balance, fee_structure.first_tier_discount, discount_token_amt)?
    {
        return Ok(discount);
    }

    if let Some(discount) =
        calculate_token_discount_for_tier(fee, fee_structure.second_tier_minimum_balance, fee_structure.second_tier_discount, discount_token_amt)?
    {
        return Ok(discount);
    }

    if let Some(discount) =
        calculate_token_discount_for_tier(fee, fee_structure.third_tier_minimum_balance, fee_structure.third_tier_discount, discount_token_amt)?
    {
        return Ok(discount);
    }

    if let Some(discount) =
        calculate_token_discount_for_tier(fee, fee_structure.fourth_tier_minimum_balance, fee_structure.fourth_tier_discount, discount_token_amt)?
    {
        return Ok(discount);
    }


    Ok(Uint128::zero())
}

fn calculate_token_discount_for_tier(
    fee: Uint128,
    tier_minimum_balance: Uint128,
    discount : Decimal,
    discount_token_amt: Uint128,
) -> Result<Option<Uint128>, ContractError> {
    if belongs_to_tier(tier_minimum_balance, discount_token_amt) {
        return try_calculate_token_discount_for_tier(fee, discount);
    }
    Ok(None)
}

fn try_calculate_token_discount_for_tier(fee: Uint128, discount : Decimal) -> Result<Option<Uint128>, ContractError> {
    let res = fee.checked_mul(Uint128::from(discount.numerator()))?.checked_div(Uint128::from(discount.denominator()))?;
    Ok(Some(res))
}

fn belongs_to_tier(tier_minimum_balance: Uint128, discount_token_amt: Uint128) -> bool {
    discount_token_amt.ge(&tier_minimum_balance)
}

fn calculate_referral_reward_and_referee_discount(
    fee: Uint128,
    fee_structure: &FeeStructure,
    referrer: &Option<Addr>,
) -> Result<(Uint128, Uint128), ContractError> {
    if referrer.is_none() {
        return Ok((Uint128::zero(), Uint128::zero()));
    }

    let referrer_reward = fee
        .checked_mul(Uint128::from(fee_structure.referrer_reward.numerator()))?
        .checked_div(Uint128::from(fee_structure.referrer_reward.denominator()))?;

    let referee_discount = fee
        .checked_mul(Uint128::from(fee_structure.referee_discount.numerator()))?
        .checked_div(Uint128::from(fee_structure.referee_discount.denominator()))?;

    return Ok((referrer_reward, referee_discount));
}


pub fn calculate_order_fee_tier(
    fee_structure: &FeeStructure,
    discount_token_amt: Uint128,
) -> Result<OrderDiscountTier, ContractError> {
    if discount_token_amt.is_zero() {
        return Ok(OrderDiscountTier::None);
    }

    if belongs_to_tier(
        fee_structure.first_tier_minimum_balance,
        discount_token_amt,
    ) {
        return Ok(OrderDiscountTier::First);
    }

    if belongs_to_tier(
        fee_structure.second_tier_minimum_balance,
        discount_token_amt,
    ) {
        return Ok(OrderDiscountTier::Second);
    }

    if belongs_to_tier(
        fee_structure.third_tier_minimum_balance,
        discount_token_amt,
    ) {
        return Ok(OrderDiscountTier::Third);
    }

    if belongs_to_tier(
        fee_structure.fourth_tier_minimum_balance,
        discount_token_amt,
    ) {
        return Ok(OrderDiscountTier::Fourth);
    }

    Ok(OrderDiscountTier::None)
}

pub fn calculate_fee_for_order(
    quote_asset_amount: Uint128,
    fee_structure: &FeeStructure,
    filler_reward_structure: &OrderState,
    order_fee_tier: &OrderDiscountTier,
    order_ts: u64,
    now: u64,
    referrer: &Option<Addr>,
    filler_is_user: bool,
    quote_asset_amount_surplus: Uint128,
) -> Result<(Uint128, Uint128, Uint128, Uint128, Uint128, Uint128), ContractError> {
    // if there was a quote_asset_amount_surplus, the order was a maker order and fee_to_market comes from surplus
    if !quote_asset_amount_surplus.is_zero() {
        let fee = quote_asset_amount_surplus;
        let filler_reward: Uint128 = if filler_is_user {
            Uint128::zero()
        } else {
            calculate_filler_reward(fee, order_ts, now, filler_reward_structure)?
        };
        let fee_to_market = fee.checked_sub(filler_reward)?;

        Ok((Uint128::zero(), fee_to_market, Uint128::zero(), filler_reward, Uint128::zero(), Uint128::zero()))
    } else {
        let fee = quote_asset_amount
            .checked_mul(Uint128::from(fee_structure.fee.numerator()))?
            .checked_div(Uint128::from(fee_structure.fee.denominator()))?;

        let token_discount =
            calculate_token_discount_for_limit_order(fee, fee_structure, order_fee_tier)?;

        let (referrer_reward, referee_discount) =
            calculate_referral_reward_and_referee_discount(fee, fee_structure, referrer)?;

        let user_fee = fee
            .checked_sub(referee_discount)?
            .checked_sub(token_discount)?;

        let filler_reward: Uint128 = if filler_is_user {
            Uint128::zero()
        } else {
            calculate_filler_reward(user_fee, order_ts, now, filler_reward_structure)?
        };

        let fee_to_market = user_fee
            .checked_sub(filler_reward)?
            .checked_sub(referrer_reward)?;

        Ok((
            user_fee,
            fee_to_market,
            token_discount,
            filler_reward,
            referrer_reward,
            referee_discount,
        ))
    }
}

fn calculate_token_discount_for_limit_order(
    fee: Uint128,
    fee_structure: &FeeStructure,
    order_discount_tier: &OrderDiscountTier,
) -> Result<Uint128, ContractError> {
    match order_discount_tier {
        OrderDiscountTier::None => Ok(Uint128::zero()),
        OrderDiscountTier::First => {
            try_calculate_token_discount_for_tier(fee, fee_structure.first_tier_discount)?
                .ok_or_else(|| (ContractError::MathError))
        }
        OrderDiscountTier::Second => {
            try_calculate_token_discount_for_tier(fee, fee_structure.second_tier_discount)?
                .ok_or_else(|| (ContractError::MathError))
        }
        OrderDiscountTier::Third => {
            try_calculate_token_discount_for_tier(fee, fee_structure.third_tier_discount)?
                .ok_or_else(|| (ContractError::MathError))
        }
        OrderDiscountTier::Fourth => {
            try_calculate_token_discount_for_tier(fee, fee_structure.fourth_tier_discount)?
                .ok_or_else(|| (ContractError::MathError))
        }
    }
}

fn calculate_filler_reward(
    fee: Uint128,
    order_ts: u64,
    now: u64,
    filler_reward_structure: &OrderState,
) -> Result<Uint128, ContractError> {
    // incentivize keepers to prioritize filling older orders (rather than just largest orders)
    // for sufficiently small-sized order, reward based on fraction of fee paid

    let size_filler_reward = fee
        .checked_mul(Uint128::from(filler_reward_structure.reward.numerator()))?
        .checked_div(Uint128::from(filler_reward_structure.reward.denominator()))?;

    let min_time_filler_reward = filler_reward_structure.time_based_reward_lower_bound.u128();
    let time_since_order = max(
        1,
        (now as i64).checked_sub(order_ts as i64).ok_or_else(|| (ContractError::MathError))?,
    );
    let time_filler_reward = (time_since_order as u128)
        .checked_mul(100_000_000) // 1e8
        .ok_or_else(|| (ContractError::MathError))?
        .integer_sqrt().integer_sqrt()
        .checked_mul(min_time_filler_reward)
        .ok_or_else(|| (ContractError::MathError))?
        .checked_div(100) // 1e2 = sqrt(sqrt(1e8))
        .ok_or_else(|| (ContractError::MathError))?;

    // lesser of size-based and time-based reward
    let fee = min(size_filler_reward.u128(), time_filler_reward);

    Ok(Uint128::from(fee))
}
