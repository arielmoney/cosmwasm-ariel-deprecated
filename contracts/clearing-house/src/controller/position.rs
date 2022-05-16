use crate::package::number::Number128;
use cosmwasm_std::{Addr, DepsMut, Uint128};

use crate::package::types::{PositionDirection, SwapDirection};

use crate::error::ContractError;

use crate::helpers::amm::should_round_trade;
use crate::helpers::order::calculate_quote_asset_amount_for_maker_order;
use crate::helpers::position::calculate_base_asset_value_and_pnl;
use crate::states::market::{Market, MARKETS};
use crate::states::user::{Position, User, POSITIONS, USERS};

use crate::helpers::position::{calculate_pnl, calculate_updated_collateral};

use crate::controller::amm;

pub fn increase(
    deps: &mut DepsMut,
    direction: PositionDirection,
    quote_asset_amount: Uint128,
    market_index: u64,
    user_addr: &Addr,
    position_index: u64,
    now: u64,
    precomputed_mark_price: Option<Uint128>,
) -> Result<i128, ContractError> {
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    let mut market_position = POSITIONS.load(deps.storage, (user_addr, position_index.to_string()))?;
    if quote_asset_amount.is_zero() {
        return Ok(0 as i128);
    }

    // Update funding rate if this is a new position
    if market_position.base_asset_amount.i128() == 0 {
        market_position.last_cumulative_funding_rate = match direction {
            PositionDirection::Long => market.amm.cumulative_funding_rate_long,
            PositionDirection::Short => market.amm.cumulative_funding_rate_short,
        };

        market.open_interest = market.open_interest.checked_add(Uint128::from(1 as u128))?;
    }

    market_position.quote_asset_amount = market_position
        .quote_asset_amount
        .checked_add(quote_asset_amount)?;

    let swap_direction = match direction {
        PositionDirection::Long => SwapDirection::Add,
        PositionDirection::Short => SwapDirection::Remove,
    };

    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market) },
    )?;

    let base_asset_acquired = amm::swap_quote_asset(
        deps,
        market_index,
        quote_asset_amount,
        swap_direction,
        now,
        precomputed_mark_price,
    )?;

    market = MARKETS.load(deps.storage, market_index.to_string())?;

    // update the position size on market and user
    market_position.base_asset_amount = Number128::new(
        market_position
            .base_asset_amount
            .i128()
            .checked_add(base_asset_acquired)
            .ok_or_else(|| (ContractError::MathError1))?,
    );
    market.base_asset_amount = Number128::new(
        market
            .base_asset_amount
            .i128()
            .checked_add(base_asset_acquired)
            .ok_or_else(|| (ContractError::MathError2))?,
    );

    if market_position.base_asset_amount.i128() > 0 {
        market.base_asset_amount_long = Number128::new(
            market
                .base_asset_amount_long
                .i128()
                .checked_add(base_asset_acquired)
                .ok_or_else(|| (ContractError::MathError3))?,
        );
    } else {
        market.base_asset_amount_short = Number128::new(
            market
                .base_asset_amount_short
                .i128()
                .checked_add(base_asset_acquired)
                .ok_or_else(|| (ContractError::MathError4))?,
        );
    }

    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market) },
    )?;

    POSITIONS.update(
        deps.storage,
        (user_addr, market_index.to_string()),
        |_p| -> Result<Position, ContractError> { Ok(market_position) },
    )?;

    Ok(base_asset_acquired)
}

pub fn reduce(
    deps: &mut DepsMut,
    direction: PositionDirection,
    quote_asset_swap_amount: Uint128,
    user_addr: &Addr,
    market_index: u64,
    position_index: u64,
    now: u64,
    precomputed_mark_price: Option<Uint128>,
) -> Result<i128, ContractError> {
    let mut user = USERS.load(deps.storage, user_addr)?;
    let mut market_position = POSITIONS.load(deps.storage, (user_addr, position_index.to_string()))?;
    let swap_direction = match direction {
        PositionDirection::Long => SwapDirection::Add,
        PositionDirection::Short => SwapDirection::Remove,
    };

    let base_asset_swapped = amm::swap_quote_asset(
        deps,
        market_index,
        quote_asset_swap_amount,
        swap_direction,
        now,
        precomputed_mark_price,
    )?;

    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    
    let base_asset_amount_before = market_position.base_asset_amount;
    market_position.base_asset_amount = Number128::new(
        market_position
            .base_asset_amount
            .i128()
            .checked_add(base_asset_swapped)
            .ok_or_else(|| (ContractError::MathError))?,
    );

    if market_position.base_asset_amount.i128() == 0 {
        market.open_interest = market.open_interest.checked_sub(Uint128::from(1 as u128))?;
    }

    market.base_asset_amount = Number128::new(
        market
            .base_asset_amount
            .i128()
            .checked_add(base_asset_swapped)
            .ok_or_else(|| (ContractError::MathError))?,
    );

    if market_position.base_asset_amount.i128() > 0 {
        market.base_asset_amount_long = Number128::new(
            market
                .base_asset_amount_long
                .i128()
                .checked_add(base_asset_swapped)
                .ok_or_else(|| (ContractError::MathError))?,
        );
    } else {
        market.base_asset_amount_short = Number128::new(
            market
                .base_asset_amount_short
                .i128()
                .checked_add(base_asset_swapped)
                .ok_or_else(|| (ContractError::MathError))?,
        );
    }

    let base_asset_amount_change = base_asset_amount_before
        .i128()
        .checked_sub(market_position.base_asset_amount.i128())
        .ok_or_else(|| (ContractError::MathError))?
        .abs();

    let initial_quote_asset_amount_closed = market_position
        .quote_asset_amount
        .checked_mul(Uint128::from(base_asset_amount_change.unsigned_abs()))?
        .checked_div(Uint128::from(
            base_asset_amount_before.i128().unsigned_abs(),
        ))?;

    market_position.quote_asset_amount = market_position
        .quote_asset_amount
        .checked_sub(initial_quote_asset_amount_closed)?;

    let pnl = if market_position.base_asset_amount.i128() > 0 {
        (quote_asset_swap_amount.u128() as i128)
            .checked_sub(initial_quote_asset_amount_closed.u128() as i128)
            .ok_or_else(|| (ContractError::MathError))?
    } else {
        (initial_quote_asset_amount_closed.checked_sub(quote_asset_swap_amount)?).u128() as i128
    };

    user.collateral = calculate_updated_collateral(user.collateral, pnl)?;

    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market) },
    )?;

    POSITIONS.update(
        deps.storage,
        (user_addr, position_index.to_string()),
        |_p| -> Result<Position, ContractError> { Ok(market_position) },
    )?;

    USERS.update(
        deps.storage,
        user_addr,
        |_u| -> Result<User, ContractError> { Ok(user) },
    )?;

    Ok(base_asset_swapped)
}

pub fn close(
    deps: &mut DepsMut,
    user_addr: &Addr,
    market_index: u64,
    position_index: u64,
    now: u64,
    maker_limit_price: Option<Uint128>,
    precomputed_mark_price: Option<Uint128>,
) -> Result<(Uint128, i128, Uint128), ContractError> {
    let mut user = USERS.load(deps.storage, user_addr)?;
    let mut market_position = POSITIONS.load(deps.storage, (user_addr, position_index.to_string()))?;
    // If user has no base asset, return early
    if market_position.base_asset_amount.i128() == 0 {
        return Ok((Uint128::zero(), 0, Uint128::zero()));
    }

    let swap_direction = if market_position.base_asset_amount.i128() > 0 {
        SwapDirection::Add
    } else {
        SwapDirection::Remove
    };

    let quote_asset_swapped = amm::swap_base_asset(
        deps,
        market_index,
        Uint128::from(market_position.base_asset_amount.i128().unsigned_abs()),
        swap_direction,
        now,
        precomputed_mark_price,
    )?;

    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    

    let (quote_asset_amount, quote_asset_amount_surplus) = match maker_limit_price {
        Some(limit_price) => calculate_quote_asset_amount_surplus(
            swap_direction,
            quote_asset_swapped,
            Uint128::from(market_position.base_asset_amount.i128().unsigned_abs()),
            limit_price,
        )?,
        None => (quote_asset_swapped, Uint128::zero()),
    };

    let pnl = calculate_pnl(
        quote_asset_swapped,
        market_position.quote_asset_amount,
        swap_direction,
    )?;

    user.collateral = calculate_updated_collateral(user.collateral, pnl)?;
    market_position.last_cumulative_funding_rate = Number128::zero();
    market_position.last_funding_rate_ts = 0;

    market.open_interest = market.open_interest.checked_sub(Uint128::from(1 as u128))?;

    market_position.quote_asset_amount = Uint128::zero();

    market.base_asset_amount = Number128::new(
        market
            .base_asset_amount
            .i128()
            .checked_sub(market_position.base_asset_amount.i128())
            .ok_or_else(|| (ContractError::MathError))?,
    );

    if market_position.base_asset_amount.i128() > 0 {
        market.base_asset_amount_long = Number128::new(
            market
                .base_asset_amount_long
                .i128()
                .checked_sub(market_position.base_asset_amount.i128())
                .ok_or_else(|| (ContractError::MathError))?,
        );
    } else {
        market.base_asset_amount_short = Number128::new(
            market
                .base_asset_amount_short
                .i128()
                .checked_sub(market_position.base_asset_amount.i128())
                .ok_or_else(|| (ContractError::MathError))?,
        );
    }

    let base_asset_amount = market_position.base_asset_amount.i128();
    market_position.base_asset_amount = Number128::zero();

    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market) },
    )?;

    POSITIONS.update(
        deps.storage,
        (user_addr, position_index.to_string()),
        |_p| -> Result<Position, ContractError> { Ok(market_position) },
    )?;

    USERS.update(
        deps.storage,
        user_addr,
        |_u| -> Result<User, ContractError> { Ok(user) },
    )?;

    Ok((
        quote_asset_amount,
        base_asset_amount,
        quote_asset_amount_surplus,
    ))
}

pub fn add_new_position(
    deps: &mut DepsMut,
    user_addr: &Addr,
    market_index: u64,
) -> Result<u64, ContractError> {

    let new_market_position = Position {
        market_index,
        base_asset_amount: Number128::zero(),
        quote_asset_amount: Uint128::zero(),
        last_cumulative_funding_rate: Number128::zero(),
        last_cumulative_repeg_rebate: Uint128::zero(),
        last_funding_rate_ts: 0,
        order_length: 0,
    };

    POSITIONS.update(
        deps.storage,
        (user_addr, market_index.to_string()),
        |_p| -> Result<Position, ContractError> { Ok(new_market_position) },
    )?;

    Ok(market_index)
}

pub fn increase_with_base_asset_amount(
    deps: &mut DepsMut,
    direction: PositionDirection,
    base_asset_amount: Uint128,
    user_addr: &Addr,
    position_index: u64,
    now: u64,
    maker_limit_price: Option<Uint128>,
    precomputed_mark_price: Option<Uint128>,
) -> Result<(Uint128, Uint128), ContractError> {
    let user = USERS.load(deps.storage, user_addr)?;
    let mut market_position = POSITIONS.load(deps.storage, (user_addr, position_index.to_string()))?;

    let market_index = position_index;

    if base_asset_amount.is_zero() {
        return Ok((Uint128::zero(), Uint128::zero()));
    }

    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;

    // Update funding rate if this is a new position
    if market_position.base_asset_amount.i128() == 0 {
        market_position.last_cumulative_funding_rate = match direction {
            PositionDirection::Long => market.amm.cumulative_funding_rate_long,
            PositionDirection::Short => market.amm.cumulative_funding_rate_short,
        };

        market.open_interest = market.open_interest.checked_add(Uint128::from(1 as u64))?;
    }

    let swap_direction = match direction {
        PositionDirection::Long => SwapDirection::Remove,
        PositionDirection::Short => SwapDirection::Add,
    };

    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market) },
    )?;

    let quote_asset_swapped = amm::swap_base_asset(
        deps,
        market_index,
        base_asset_amount,
        swap_direction,
        now,
        precomputed_mark_price,
    )?;

    market = MARKETS.load(deps.storage, market_index.to_string())?;

    let (quote_asset_amount, quote_asset_amount_surplus) = match maker_limit_price {
        Some(limit_price) => calculate_quote_asset_amount_surplus(
            swap_direction,
            quote_asset_swapped,
            base_asset_amount,
            limit_price,
        )?,
        None => (quote_asset_swapped, Uint128::zero()),
    };

    market_position.quote_asset_amount = market_position
        .quote_asset_amount
        .checked_add(quote_asset_amount)?;

    let base_asset_amount = match direction {
        PositionDirection::Long => (base_asset_amount.u128() as i128),
        PositionDirection::Short => -(base_asset_amount.u128() as i128),
    };

    market_position.base_asset_amount = Number128::new(
        market_position
            .base_asset_amount
            .i128()
            .checked_add(base_asset_amount)
            .ok_or_else(|| (ContractError::MathError))?,
    );
    market.base_asset_amount = Number128::new(
        market
            .base_asset_amount
            .i128()
            .checked_add(base_asset_amount)
            .ok_or_else(|| (ContractError::MathError))?,
    );

    if market_position.base_asset_amount.i128() > 0 {
        market.base_asset_amount_long = Number128::new(
            market
                .base_asset_amount_long
                .i128()
                .checked_add(base_asset_amount)
                .ok_or_else(|| (ContractError::MathError))?,
        );
    } else {
        market.base_asset_amount_short = Number128::new(
            market
                .base_asset_amount_short
                .i128()
                .checked_add(base_asset_amount)
                .ok_or_else(|| (ContractError::MathError))?,
        );
    }

    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market) },
    )?;

    POSITIONS.update(
        deps.storage,
        (user_addr, position_index.to_string()),
        |_p| -> Result<Position, ContractError> { Ok(market_position) },
    )?;

    USERS.update(
        deps.storage,
        user_addr,
        |_u| -> Result<User, ContractError> { Ok(user) },
    )?;

    Ok((quote_asset_amount, quote_asset_amount_surplus))
}

pub fn reduce_with_base_asset_amount(
    deps: &mut DepsMut,
    direction: PositionDirection,
    base_asset_amount: Uint128,
    user_addr: &Addr,
    position_index: u64,
    now: u64,
    maker_limit_price: Option<Uint128>,
    precomputed_mark_price: Option<Uint128>,
) -> Result<(Uint128, Uint128), ContractError> {
    let mut user = USERS.load(deps.storage, user_addr)?;
    let mut market_position = POSITIONS.load(deps.storage, (user_addr, position_index.to_string()))?;

    let market_index = position_index;
    
    let swap_direction = match direction {
        PositionDirection::Long => SwapDirection::Remove,
        PositionDirection::Short => SwapDirection::Add,
    };

    let quote_asset_swapped = amm::swap_base_asset(
        deps,
        market_index,
        base_asset_amount,
        swap_direction,
        now,
        precomputed_mark_price,
    )?;

    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;

    let (quote_asset_amount, quote_asset_amount_surplus) = match maker_limit_price {
        Some(limit_price) => calculate_quote_asset_amount_surplus(
            swap_direction,
            quote_asset_swapped,
            base_asset_amount,
            limit_price,
        )?,
        None => (quote_asset_swapped, Uint128::zero()),
    };

    let base_asset_amount = match direction {
        PositionDirection::Long => (base_asset_amount.u128() as i128),
        PositionDirection::Short => -(base_asset_amount.u128() as i128),
    };

    let base_asset_amount_before = market_position.base_asset_amount.i128();
    market_position.base_asset_amount = Number128::new(
        market_position
            .base_asset_amount
            .i128()
            .checked_add(base_asset_amount)
            .ok_or_else(|| (ContractError::MathError))?,
    );

    if market_position.base_asset_amount.i128() == 0 {
        market.open_interest = market.open_interest.checked_sub(Uint128::from(1 as u128))?;
    }

    market.base_asset_amount = Number128::new(
        market
            .base_asset_amount
            .i128()
            .checked_add(base_asset_amount)
            .ok_or_else(|| (ContractError::MathError))?,
    );

    if market_position.base_asset_amount.i128() > 0 {
        market.base_asset_amount_long = Number128::new(
            market
                .base_asset_amount_long
                .i128()
                .checked_add(base_asset_amount)
                .ok_or_else(|| (ContractError::MathError))?,
        );
    } else {
        market.base_asset_amount_short = Number128::new(
            market
                .base_asset_amount_short
                .i128()
                .checked_add(base_asset_amount)
                .ok_or_else(|| (ContractError::MathError))?,
        );
    }

    let base_asset_amount_change = base_asset_amount_before
        .checked_sub(market_position.base_asset_amount.i128())
        .ok_or_else(|| (ContractError::MathError))?
        .abs();

    let initial_quote_asset_amount_closed = market_position
        .quote_asset_amount
        .checked_mul(Uint128::from(base_asset_amount_change.unsigned_abs()))?
        .checked_div(Uint128::from(base_asset_amount_before.unsigned_abs()))?;

    market_position.quote_asset_amount = market_position
        .quote_asset_amount
        .checked_sub(initial_quote_asset_amount_closed)?;

    let pnl = if PositionDirection::Short == direction {
        (quote_asset_amount.u128() as i128)
            .checked_sub(initial_quote_asset_amount_closed.u128() as i128)
            .ok_or_else(|| (ContractError::MathError))?
    } else {
        (initial_quote_asset_amount_closed.u128() as i128)
            .checked_sub(quote_asset_amount.u128() as i128)
            .ok_or_else(|| (ContractError::MathError))?
    };

    user.collateral = calculate_updated_collateral(user.collateral, pnl)?;

    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market) },
    )?;

    POSITIONS.update(
        deps.storage,
        (user_addr, position_index.to_string()),
        |_p| -> Result<Position, ContractError> { Ok(market_position) },
    )?;

    USERS.update(
        deps.storage,
        user_addr,
        |_u| -> Result<User, ContractError> { Ok(user) },
    )?;

    Ok((quote_asset_amount, quote_asset_amount_surplus))
}

pub fn update_position_with_base_asset_amount(
    deps: &mut DepsMut,
    base_asset_amount: Uint128,
    direction: PositionDirection,
    user_addr: &Addr,
    position_index: u64,
    mark_price_before: Uint128,
    now: u64,
    maker_limit_price: Option<Uint128>,
) -> Result<(bool, bool, Uint128, Uint128, Uint128), ContractError> {
    let market_position = POSITIONS.load(deps.storage, (user_addr, position_index.to_string()))?;

    let market_index = position_index;

    // A trade is risk increasing if it increases the users leverage
    // If a trade is risk increasing and brings the user's margin ratio below initial requirement
    // the trade fails
    // If a trade is risk increasing and it pushes the mark price too far away from the oracle price
    // the trade fails
    let mut potentially_risk_increasing = true;
    let mut reduce_only = false;

    // The trade increases the the user position if
    // 1) the user does not have a position
    // 2) the trade is in the same direction as the user's existing position
    let quote_asset_amount;
    let quote_asset_amount_surplus;
    let increase_position = market_position.base_asset_amount.i128() == 0
        || market_position.base_asset_amount.i128() > 0 && direction == PositionDirection::Long
        || market_position.base_asset_amount.i128() < 0 && direction == PositionDirection::Short;
    if increase_position {
        let (_quote_asset_amount, _quote_asset_amount_surplus) = increase_with_base_asset_amount(
            deps,
            direction,
            base_asset_amount,
            user_addr,
            position_index,
            now,
            maker_limit_price,
            Some(mark_price_before),
        )?;
        quote_asset_amount = _quote_asset_amount;
        quote_asset_amount_surplus = _quote_asset_amount_surplus;
    } else if market_position.base_asset_amount.i128().unsigned_abs() > base_asset_amount.u128() {
        let (_quote_asset_amount, _quote_asset_amount_surplus) = reduce_with_base_asset_amount(
            deps,
            direction,
            base_asset_amount,
            user_addr,
            position_index,
            now,
            maker_limit_price,
            Some(mark_price_before),
        )?;
        quote_asset_amount = _quote_asset_amount;
        quote_asset_amount_surplus = _quote_asset_amount_surplus;

        reduce_only = true;
        potentially_risk_increasing = false;
    } else {
        // after closing existing position, how large should trade be in opposite direction
        let base_asset_amount_after_close = base_asset_amount.checked_sub(Uint128::from(
            market_position.base_asset_amount.i128().unsigned_abs(),
        ))?;

        // If the value of the new position is less than value of the old position, consider it risk decreasing
        if base_asset_amount_after_close.u128()
            < market_position.base_asset_amount.i128().unsigned_abs()
        {
            potentially_risk_increasing = false;
        }

        let (quote_asset_amount_closed, _, quote_asset_amount_surplus_closed) = close(
            deps,
            user_addr,
            market_index,
            position_index,
            now,
            maker_limit_price,
            Some(mark_price_before),
        )?;

        let (quote_asset_amount_opened, quote_asset_amount_surplus_opened) =
            increase_with_base_asset_amount(
                deps,
                direction,
                base_asset_amount_after_close,
                user_addr,
                position_index,
                now,
                maker_limit_price,
                Some(mark_price_before),
            )?;

        // means position was closed and it was reduce only
        if quote_asset_amount_opened.is_zero() {
            reduce_only = true;
        }

        quote_asset_amount = quote_asset_amount_closed.checked_add(quote_asset_amount_opened)?;

        quote_asset_amount_surplus =
            quote_asset_amount_surplus_closed.checked_add(quote_asset_amount_surplus_opened)?;
    }

    Ok((
        potentially_risk_increasing,
        reduce_only,
        base_asset_amount,
        quote_asset_amount,
        quote_asset_amount_surplus,
    ))
}

pub fn update_position_with_quote_asset_amount(
    deps: &mut DepsMut,
    quote_asset_amount: Uint128,
    direction: PositionDirection,
    user_addr: &Addr,
    position_index: u64,
    mark_price_before: Uint128,
    now: u64,
) -> Result<(bool, bool, Uint128, Uint128, Uint128), ContractError> {
    let market_position;
    let existing_position =
        POSITIONS.may_load(deps.storage, (&user_addr.clone(), position_index.to_string()))?;
    match existing_position {
        Some(exp) => {
            market_position = exp;
        }
        None => {
            market_position = Position {
                market_index: position_index,
                base_asset_amount: Number128::zero(),
                quote_asset_amount: Uint128::zero(),
                last_cumulative_funding_rate: Number128::zero(),
                last_cumulative_repeg_rebate: Uint128::zero(),
                last_funding_rate_ts: 0,
                order_length: 0,
            };
            POSITIONS.save(
                deps.storage,
                (&user_addr.clone(), position_index.to_string()),
                &market_position,
            )?;
        }
    }
    let market_index = market_position.market_index;
    let market = MARKETS.load(deps.storage, market_index.to_string())?;

    // A trade is risk increasing if it increases the users leverage
    // If a trade is risk increasing and brings the user's margin ratio below initial requirement
    // the trade fails
    // If a trade is risk increasing and it pushes the mark price too far away from the oracle price
    // the trade fails
    let mut potentially_risk_increasing = true;
    let mut reduce_only = false;

    let mut quote_asset_amount = quote_asset_amount;
    let base_asset_amount;
    // The trade increases the the user position if
    // 1) the user does not have a position
    // 2) the trade is in the same direction as the user's existing position
    let increase_position = market_position.base_asset_amount.i128() == 0
        || market_position.base_asset_amount.i128() > 0 && direction == PositionDirection::Long
        || market_position.base_asset_amount.i128() < 0 && direction == PositionDirection::Short;
    if increase_position {
        base_asset_amount = increase(
            deps,
            direction,
            quote_asset_amount,
            market_index,
            &user_addr.clone(),
            position_index,
            now,
            Some(mark_price_before),
        )?
        .unsigned_abs();
    } else {
        let (base_asset_value, _unrealized_pnl) =
            calculate_base_asset_value_and_pnl(&market_position, &market.amm)?;

        // if the quote_asset_amount is close enough in value to base_asset_value,
        // round the quote_asset_amount to be the same as base_asset_value
        if should_round_trade(&market.amm, quote_asset_amount, base_asset_value)? {
            quote_asset_amount = base_asset_value;
        }

        // we calculate what the user's position is worth if they closed to determine
        // if they are reducing or closing and reversing their position
        if base_asset_value > quote_asset_amount {
            base_asset_amount = reduce(
                deps,
                direction,
                quote_asset_amount,
                &user_addr.clone(),
                market_index,
                position_index,
                now,
                Some(mark_price_before),
            )?
            .unsigned_abs();

            potentially_risk_increasing = false;
            reduce_only = true;
        } else {
            // after closing existing position, how large should trade be in opposite direction
            let quote_asset_amount_after_close =
                quote_asset_amount.checked_sub(base_asset_value)?;

            // If the value of the new position is less than value of the old position, consider it risk decreasing
            if quote_asset_amount_after_close < base_asset_value {
                potentially_risk_increasing = false;
            }

            let (_, base_asset_amount_closed, _) = close(
                deps,
                &user_addr.clone(),
                market_index,
                position_index,
                now,
                None,
                Some(mark_price_before),
            )?;
            let base_asset_amount_closed = base_asset_amount_closed.unsigned_abs();

            let base_asset_amount_opened = increase(
                deps,
                direction,
                quote_asset_amount_after_close,
                market_index,
                &user_addr.clone(),
                position_index,
                now,
                Some(mark_price_before),
            )?
            .unsigned_abs();

            // means position was closed and it was reduce only
            if base_asset_amount_opened == 0 {
                reduce_only = true;
            }

            base_asset_amount = base_asset_amount_closed
                .checked_add(base_asset_amount_opened)
                .ok_or_else(|| (ContractError::MathError))?;
        }
    }

    Ok((
        potentially_risk_increasing,
        reduce_only,
        Uint128::from(base_asset_amount),
        quote_asset_amount,
        Uint128::zero(),
    ))
}

fn calculate_quote_asset_amount_surplus(
    swap_direction: SwapDirection,
    quote_asset_swapped: Uint128,
    base_asset_amount: Uint128,
    limit_price: Uint128,
) -> Result<(Uint128, Uint128), ContractError> {
    let quote_asset_amount =
        calculate_quote_asset_amount_for_maker_order(base_asset_amount, limit_price)?;

    let quote_asset_amount_surplus = match swap_direction {
        SwapDirection::Remove => quote_asset_amount.checked_sub(quote_asset_swapped)?,
        SwapDirection::Add => quote_asset_swapped.checked_sub(quote_asset_amount)?,
    };

    Ok((quote_asset_amount, quote_asset_amount_surplus))
}
