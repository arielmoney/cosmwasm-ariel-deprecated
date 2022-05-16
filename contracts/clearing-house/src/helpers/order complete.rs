use crate::error::ContractError;
use crate::states::market::Market;
use crate::states::order::{get_limit_price, has_oracle_price_offset};
use crate::states::state::OrderState;

use std::cmp::min;
use std::ops::Div;
use crate::package::types::{Order, OrderType, OrderTriggerCondition, PositionDirection, OracleGuardRails};
use cosmwasm_std::{Addr, Uint128};

use crate::states::constants::{
    AMM_TO_QUOTE_PRECISION_RATIO, MARK_PRICE_PRECISION,
    MARK_PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO, AMM_RESERVE_PRECISION, QUOTE_PRECISION
};
use crate::helpers::amm;

use crate::helpers::amm::is_oracle_valid;

use super::position::asset_to_reserve_amount;

pub fn calculate_base_asset_amount_market_can_execute(
    order: &Order,
    market: &Market,
    precomputed_mark_price: Option<Uint128>,
    valid_oracle_price: Option<i128>,
) -> Result<Uint128, ContractError> {
    match order.order_type {
        OrderType::Limit => {
            calculate_base_asset_amount_to_trade_for_limit(order, market, valid_oracle_price)
        }
        OrderType::TriggerMarket => calculate_base_asset_amount_to_trade_for_trigger_market(
            order,
            market,
            precomputed_mark_price,
            valid_oracle_price,
        ),
        OrderType::TriggerLimit => calculate_base_asset_amount_to_trade_for_trigger_limit(
            order,
            market,
            precomputed_mark_price,
            valid_oracle_price,
        ),
        OrderType::Market => Err(ContractError::InvalidOrder),
    }
}

pub fn calculate_base_asset_amount_to_trade_for_limit(
    order: &Order,
    market: &Market,
    valid_oracle_price: Option<i128>,
) -> Result<Uint128, ContractError> {
    let base_asset_amount_to_fill = order
        .base_asset_amount
        .checked_sub(order.base_asset_amount_filled)?;

    let limit_price = get_limit_price(order, valid_oracle_price)?;

    let (max_trade_base_asset_amount, max_trade_direction) =
        amm::calculate_max_base_asset_amount_to_trade(&market.amm, limit_price)?;
    if max_trade_direction != order.direction || max_trade_base_asset_amount.is_zero() {
        return Ok(Uint128::zero());
    }

    let base_asset_amount_to_trade = min(base_asset_amount_to_fill, max_trade_base_asset_amount);

    Ok(base_asset_amount_to_trade)
}

fn calculate_base_asset_amount_to_trade_for_trigger_market(
    order: &Order,
    market: &Market,
    precomputed_mark_price: Option<Uint128>,
    valid_oracle_price: Option<i128>,
) -> Result<Uint128, ContractError> {
    let mark_price = match precomputed_mark_price {
        Some(mark_price) => mark_price,
        None => market.amm.mark_price()?,
    };

    match order.trigger_condition {
        OrderTriggerCondition::Above => {
            if mark_price <= order.trigger_price {
                return Ok(Uint128::zero());
            }

            // If there is a valid oracle, check that trigger condition is also satisfied by
            // oracle price (plus some additional buffer)
            if let Some(oracle_price) = valid_oracle_price {
                let oracle_price_101pct = oracle_price
                    .checked_mul(101)
                    .ok_or_else(|| (ContractError::MathError))?
                    .checked_div(100)
                    .ok_or_else(|| (ContractError::MathError))?;

                if oracle_price_101pct.le(&(order.trigger_price.u128() as i128)) {
                    return Ok(Uint128::zero());
                }
            }
        }
        OrderTriggerCondition::Below => {
            if mark_price >= order.trigger_price {
                return Ok(Uint128::zero());
            }

            // If there is a valid oracle, check that trigger condition is also satisfied by
            // oracle price (plus some additional buffer)
            if let Some(oracle_price) = valid_oracle_price {
                let oracle_price_99pct = oracle_price
                    .checked_mul(99)
                    .ok_or_else(|| (ContractError::MathError))?
                    .checked_div(100)
                    .ok_or_else(|| (ContractError::MathError))?;

                if Uint128::from(oracle_price_99pct.unsigned_abs()).ge(&order.trigger_price) {
                    return Ok(Uint128::zero());
                }
            }
        }
    }

    let res = order
        .base_asset_amount
        .checked_sub(order.base_asset_amount_filled)?;

    Ok(res)
}

fn calculate_base_asset_amount_to_trade_for_trigger_limit(
    order: &Order,
    market: &Market,
    precomputed_mark_price: Option<Uint128>,
    valid_oracle_price: Option<i128>,
) -> Result<Uint128, ContractError> {
    // if the order has not been filled yet, need to check that trigger condition is met
    if order.base_asset_amount_filled.is_zero() {
        let base_asset_amount = calculate_base_asset_amount_to_trade_for_trigger_market(
            order,
            market,
            precomputed_mark_price,
            valid_oracle_price,
        )?;
        if base_asset_amount.is_zero() {
            return Ok(Uint128::zero());
        }
    }

    calculate_base_asset_amount_to_trade_for_limit(order, market, None)
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

pub fn calculate_quote_asset_amount_for_maker_order(
    base_asset_amount: Uint128,
    limit_price: Uint128,
) -> Result<Uint128, ContractError> {
    let res = base_asset_amount
    .checked_mul(limit_price)?
    .checked_div(MARK_PRICE_TIMES_AMM_TO_QUOTE_PRECISION_RATIO)?;
    Ok(res)
}

pub fn get_valid_oracle_price(
    oracle: Option<&Addr>,
    market: &Market,
    order: &Order,
    validity_guardrails: &OracleGuardRails,
    _now: u64,
) -> Result<Option<i128>, ContractError> {
    let price = if let Some(_oracle) = oracle {
        let oracle_data = market.amm.get_oracle_price()?;
        let is_oracle_valid = is_oracle_valid(&market.amm, &oracle_data, validity_guardrails)?;
        if is_oracle_valid {
            Some(oracle_data.price.i128())
        } else if has_oracle_price_offset(order) {
            // msg!("Invalid oracle for order with oracle price offset");
            return Err(ContractError::InvalidOracle);
        } else {
            None
        }
    } else if has_oracle_price_offset(order) {
        // msg!("Oracle not found for order with oracle price offset");
        return Err(ContractError::OracleNotFound);
    } else {
        None
    };

    Ok(price)
}


pub fn validate_order(
    order: &Order,
    market: &Market,
    order_state: &OrderState,
    valid_oracle_price: Option<i128>,
) -> Result<bool, ContractError> {
    match order.order_type {
        OrderType::Market => validate_market_order(order, market)?,
        OrderType::Limit => validate_limit_order(order, market, order_state, valid_oracle_price)?,
        OrderType::TriggerMarket => validate_trigger_market_order(order, market, order_state)?,
        OrderType::TriggerLimit => validate_trigger_limit_order(order, market, order_state)?,
    };

    if order.immediate_or_cancel {
        // msg!("immediate_or_cancel not supported yet");
        return Err(ContractError::InvalidOrder);
    }

    Ok(true)
}

fn validate_market_order(
    order: &Order, 
    market: &Market
) -> Result<bool, ContractError> {
    if order.quote_asset_amount.gt(&Uint128::zero()) && order.base_asset_amount.gt(&Uint128::zero()) {
        // msg!("Market order should not have quote_asset_amount and base_asset_amount set");
        return Err(ContractError::InvalidOrder);
    }

    if order.base_asset_amount.gt(&Uint128::zero()) {
        validate_base_asset_amount(order, market)?;
    } else {
        validate_quote_asset_amount(order, market)?;
    }

    if order.trigger_price.gt(&Uint128::zero()) {
        // msg!("Market should not have trigger price");
        return Err(ContractError::InvalidOrder);
    }

    if order.post_only {
        // msg!("Market order can not be post only");
        return Err(ContractError::InvalidOrder);
    }

    if has_oracle_price_offset(order) {
        // msg!("Market order can not have oracle offset");
        return Err(ContractError::InvalidOrder);
    }

    Ok(true)
}

fn validate_limit_order(
    order: &Order,
    market: &Market,
    order_state: &OrderState,
    valid_oracle_price: Option<i128>,
) -> Result<bool, ContractError> {
    validate_base_asset_amount(order, market)?;

    if order.price.is_zero() && !has_oracle_price_offset(order) {
        // msg!("Limit order price == 0");
        return Err(ContractError::InvalidOrder);
    }

    if order.price.ne(&Uint128::zero()) && has_oracle_price_offset(order) {
        // msg!("Limit order price != 0 and oracle price offset is set");
        return Err(ContractError::InvalidOrder);
    }

    if order.trigger_price.gt(&Uint128::zero()) {
        // msg!("Limit order should not have trigger price");
        return Err(ContractError::InvalidOrder);
    }

    if order.quote_asset_amount.ne(&Uint128::zero()) {
        // msg!("Limit order should not have a quote asset amount");
        return Err(ContractError::InvalidOrder);
    }

    if order.post_only {
        validate_post_only_order(order, market, valid_oracle_price)?;
    }

    let limit_price = get_limit_price(order, valid_oracle_price)?;
    let approximate_market_value = limit_price.u128()
        .checked_mul(order.base_asset_amount.u128())
        .or(Some(u128::MAX))
        .unwrap()
        .div(AMM_RESERVE_PRECISION.u128())
        .div(MARK_PRICE_PRECISION.u128() / QUOTE_PRECISION.u128());

    if approximate_market_value < order_state.min_order_quote_asset_amount.u128() {
        // msg!("Order value < $0.50 ({:?})", approximate_market_value);
        return Err(ContractError::InvalidOrder);
    }

    Ok(true)
}

fn validate_post_only_order(
    order: &Order,
    market: &Market,
    valid_oracle_price: Option<i128>,
) -> Result<bool, ContractError> {
    let base_asset_amount_market_can_fill =
        calculate_base_asset_amount_to_trade_for_limit(order, market, valid_oracle_price)?;

    if base_asset_amount_market_can_fill.ne(&Uint128::zero()) {
        // msg!(
        //     "Post-only order can immediately fill {} base asset amount",
        //     base_asset_amount_market_can_fill
        // );
        return Err(ContractError::InvalidOrder);
    }

    Ok(true)
}

fn validate_trigger_limit_order(
    order: &Order,
    market: &Market,
    order_state: &OrderState,
) -> Result<bool, ContractError> {
    validate_base_asset_amount(order, market)?;

    if order.price.is_zero() {
        // msg!("Trigger limit order price == 0");
        return Err(ContractError::InvalidOrder);
    }

    if order.trigger_price.is_zero() {
        // msg!("Trigger price == 0");
        return Err(ContractError::InvalidOrder);
    }

    if !order.quote_asset_amount.is_zero() {
        // msg!("Trigger limit order should not have a quote asset amount");
        return Err(ContractError::InvalidOrder);
    }

    if order.post_only {
        // msg!("Trigger limit order can not be post only");
        return Err(ContractError::InvalidOrder);
    }

    if has_oracle_price_offset(order) {
        // msg!("Trigger limit can not have oracle offset");
        return Err(ContractError::InvalidOrder);
    }

    match order.trigger_condition {
        OrderTriggerCondition::Above => {
            if order.direction == PositionDirection::Long && order.price.lt(&order.trigger_price) {
                // msg!("If trigger condition is above and direction is long, limit price must be above trigger price");
                return Err(ContractError::InvalidOrder);
            }
        }
        OrderTriggerCondition::Below => {
            if order.direction == PositionDirection::Short && order.price.gt(&order.trigger_price) {
                // msg!("If trigger condition is below and direction is short, limit price must be below trigger price");
                return Err(ContractError::InvalidOrder);
            }
        }
    }

    let approximate_market_value = order
        .price.u128()
        .checked_mul(order.base_asset_amount.u128())
        .or(Some(u128::MAX))
        .unwrap()
        .div(AMM_RESERVE_PRECISION.u128())
        .div(MARK_PRICE_PRECISION.u128() / QUOTE_PRECISION.u128());

    if approximate_market_value < order_state.min_order_quote_asset_amount.u128() {
        // msg!("Order value < $0.50 ({:?})", approximate_market_value);
        return Err(ContractError::InvalidOrder);
    }

    Ok(true)
}

fn validate_trigger_market_order(
    order: &Order,
    market: &Market,
    order_state: &OrderState,
) -> Result<bool, ContractError> {
    validate_base_asset_amount(order, market)?;

    if order.price.gt(&Uint128::zero()) {
        // msg!("Trigger market order should not have price");
        return Err(ContractError::InvalidOrder);
    }

    if order.trigger_price.is_zero() {
        // msg!("Trigger market order trigger_price == 0");
        return Err(ContractError::InvalidOrder);
    }

    if !order.quote_asset_amount.is_zero() {
        // msg!("Trigger market order should not have a quote asset amount");
        return Err(ContractError::InvalidOrder);
    }

    if order.post_only {
        // msg!("Trigger market order can not be post only");
        return Err(ContractError::InvalidOrder);
    }

    if has_oracle_price_offset(order) {
        // msg!("Trigger market order can not have oracle offset");
        return Err(ContractError::InvalidOrder);
    }

    let approximate_market_value = order
        .trigger_price.u128()
        .checked_mul(order.base_asset_amount.u128())
        .or(Some(u128::MAX))
        .unwrap()
        .div(AMM_RESERVE_PRECISION.u128())
        .div(MARK_PRICE_PRECISION.u128() / QUOTE_PRECISION.u128());

    // decide min trade size ($10?)
    if approximate_market_value < order_state.min_order_quote_asset_amount.u128() {
        // msg!("Order value < $0.50 ({:?})", approximate_market_value);
        return Err(ContractError::InvalidOrder);
    }

    Ok(true)
}

fn validate_base_asset_amount(
    order: &Order, market: &Market
) -> Result<bool, ContractError> {
    if order.base_asset_amount.is_zero() {
        // msg!("Order base_asset_amount cant be 0");
        return Err(ContractError::InvalidOrder);
    }

    if order.base_asset_amount.lt(&market.amm.minimum_base_asset_trade_size) {
        // msg!("Order base_asset_amount smaller than market minimum_base_asset_trade_size");
        return Err(ContractError::InvalidOrder);
    }

    Ok(true)
}

fn validate_quote_asset_amount(
    order: &Order, market: &Market
) -> Result<bool, ContractError> {
    if order.quote_asset_amount.is_zero() {
        // msg!("Order quote_asset_amount cant be 0");
        return Err(ContractError::InvalidOrder);
    }

    let quote_asset_reserve_amount =
        asset_to_reserve_amount(order.quote_asset_amount, market.amm.peg_multiplier)?;

    if quote_asset_reserve_amount.lt(&market.amm.minimum_quote_asset_trade_size) {
        // msg!("Order quote_asset_reserve_amount smaller than market minimum_quote_asset_trade_size");
        return Err(ContractError::InvalidOrder);
    }

    Ok(true)
}

pub fn validate_order_can_be_canceled(
    order: &Order,
    market: &Market,
    valid_oracle_price: Option<i128>,
) -> Result<bool, ContractError> {
    if !order.post_only {
        return Ok(true);
    }

    let base_asset_amount_market_can_fill =
        calculate_base_asset_amount_to_trade_for_limit(order, market, valid_oracle_price)?;

    if base_asset_amount_market_can_fill.gt(&Uint128::zero()) {
        // msg!(
        //     "Cant cancel as post only order can be filled for {} base asset amount",
        //     base_asset_amount_market_can_fill
        // );
        return Err(ContractError::CantCancelPostOnlyOrder);
    }

    Ok(true)
}