use cosmwasm_std::{Addr, DepsMut, Uint128};

use crate::error::ContractError;
use crate::helpers::position::{calculate_updated_collateral, calculate_slippage};
use crate::states::constants::{MARGIN_PRECISION, MAXIMUM_MARGIN_RATIO, MINIMUM_MARGIN_RATIO};
use crate::helpers::position::{
    calculate_base_asset_value_and_pnl, calculate_base_asset_value_and_pnl_with_oracle_price,
};
use crate::states::market::{LiquidationStatus, LiquidationType, MarketStatus, MARKETS};
use crate::states::state::{ORACLEGUARDRAILS, STATE};
use crate::states::user::{POSITIONS, USERS};

use crate::helpers::amm::use_oracle_price_for_margin_calculation;
use crate::helpers::oracle::get_oracle_status;

use std::ops::Div;

pub fn meets_initial_margin_requirement(
    deps: &mut DepsMut,
    user_addr: &Addr,
) -> Result<bool, ContractError> {
    let user = USERS.load(deps.storage, user_addr)?;

    let mut initial_margin_requirement: Uint128 = Uint128::zero();
    let mut unrealized_pnl: i128 = 0;

    let markets_length = STATE.load(deps.storage)?.markets_length;
    for n in 1..markets_length {
        let market_position = POSITIONS.load(deps.storage, (user_addr, n.to_string()));
        match market_position {
            Ok(m) => {
                if m.base_asset_amount.i128() == 0 {
                    continue;
                }
                let market = MARKETS.load(deps.storage, n.to_string())?;
                let a = &market.amm;
                let (position_base_asset_value, position_unrealized_pnl) =
                    calculate_base_asset_value_and_pnl(&m, a)?;
                initial_margin_requirement = initial_margin_requirement
                    .checked_add(
                        position_base_asset_value
                            .checked_mul(market.margin_ratio_initial.into())?,
                    )?;

                unrealized_pnl = unrealized_pnl
                    .checked_add(position_unrealized_pnl)
                    .ok_or_else(|| (ContractError::HelpersError))?;
            },
            Err(_) => continue,
        }
    }

    initial_margin_requirement = initial_margin_requirement
        .checked_div(MARGIN_PRECISION)?;

    let total_collateral = calculate_updated_collateral(user.collateral, unrealized_pnl)?;

    Ok(total_collateral.u128() >= initial_margin_requirement.u128())
}

pub fn meets_partial_margin_requirement(
    deps: &DepsMut,
    user_addr: &Addr,
) -> Result<bool, ContractError> {
    let user = USERS.load(deps.storage, user_addr)?;

    let mut partial_margin_requirement: Uint128 = Uint128::zero();
    let mut unrealized_pnl: i128 = 0;

    let markets_length = STATE.load(deps.storage)?.markets_length;
    for n in 1..markets_length {
        let market_position = POSITIONS.load(deps.storage, (user_addr, n.to_string()));
        match market_position {
            Ok(m) => {
                if m.base_asset_amount.i128() == 0 {
                    continue;
                }
                let market = MARKETS.load(deps.storage, n.to_string())?;
                let a = &market.amm;

                let (position_base_asset_value, position_unrealized_pnl) =
                    calculate_base_asset_value_and_pnl(&m, a)?;
                partial_margin_requirement = partial_margin_requirement
                    .checked_add(
                        position_base_asset_value
                            .checked_mul(market.margin_ratio_partial.into())?,
                    )?;

                unrealized_pnl = unrealized_pnl
                    .checked_add(position_unrealized_pnl)
                    .ok_or_else(|| (ContractError::HelpersError))?;
            }
            Err(_) => continue,
        }
    }

    partial_margin_requirement = partial_margin_requirement
        .checked_div(MARGIN_PRECISION)?;

    let total_collateral = calculate_updated_collateral(user.collateral, unrealized_pnl)?;

    Ok(total_collateral >= partial_margin_requirement)
}

pub fn calculate_free_collateral(
    deps: &DepsMut,
    user_addr: &Addr,
    market_to_close: Option<u64>,
) -> Result<(Uint128, Uint128), ContractError> {
    let mut closed_position_base_asset_value: Uint128 = Uint128::zero();
    let mut initial_margin_requirement: Uint128 = Uint128::zero();
    let mut unrealized_pnl: i128 = 0;

    let user = USERS.load(deps.storage, user_addr)?;

    let markets_length = STATE.load(deps.storage)?.markets_length;
    for n in 1..markets_length {
        let market_position = POSITIONS.load(deps.storage, (user_addr, n.to_string()));
        match market_position {
            Ok(m) => {
                if m.base_asset_amount.i128() == 0 {
                    continue;
                }

                let market = MARKETS.load(deps.storage, n.to_string())?;
                let a = &market.amm;
                let (position_base_asset_value, position_unrealized_pnl) =
                    calculate_base_asset_value_and_pnl(&m, a)?;

                if market_to_close.is_some() && market_to_close.unwrap() == n
                {
                    closed_position_base_asset_value = position_base_asset_value;
                } else {
                    initial_margin_requirement = initial_margin_requirement
                        .checked_add(
                            position_base_asset_value
                                .checked_mul(market.margin_ratio_initial.into())?,
                        )?;
                }

                unrealized_pnl = unrealized_pnl
                    .checked_add(position_unrealized_pnl)
                    .ok_or_else(|| (ContractError::HelpersError))?;
            }
            Err(_) => continue,
        }
    }

    initial_margin_requirement = initial_margin_requirement
        .checked_div(MARGIN_PRECISION)?;

    let total_collateral = calculate_updated_collateral(user.collateral, unrealized_pnl)?;

    let free_collateral = if initial_margin_requirement < total_collateral {
        total_collateral
            .checked_sub(initial_margin_requirement)?
    } else {
        Uint128::zero()
    };

    Ok((free_collateral, closed_position_base_asset_value))
}

pub fn calculate_liquidation_status(
    deps: &mut DepsMut,
    user_addr: &Addr,
) -> Result<LiquidationStatus, ContractError> {
    let user = USERS.load(deps.storage, user_addr)?;
    let oracle_guard_rails = ORACLEGUARDRAILS.load(deps.storage)?;

    let mut partial_margin_requirement: Uint128 = Uint128::zero();
    let mut maintenance_margin_requirement: Uint128 = Uint128::zero();
    let mut base_asset_value: Uint128 = Uint128::zero();
    let mut unrealized_pnl: i128 = 0;
    let mut adjusted_unrealized_pnl: i128 = 0;
    let mut market_statuses: Vec<MarketStatus> = Vec::new();

    let markets_length = STATE.load(deps.storage)?.markets_length;
    for n in 1..markets_length {
        let market_position = POSITIONS.load(deps.storage, (user_addr, n.to_string()));
        match market_position {
            Ok(m) => {
                if m.base_asset_amount.i128() == 0 {
                    continue;
                }

                let market = MARKETS.load(deps.storage, n.to_string())?;
                let a = &market.amm;
                let (amm_position_base_asset_value, amm_position_unrealized_pnl) =
                    calculate_base_asset_value_and_pnl(&m, a)?;

                base_asset_value = base_asset_value
                    .checked_add(amm_position_base_asset_value)?;
                unrealized_pnl = unrealized_pnl
                    .checked_add(amm_position_unrealized_pnl)
                    .ok_or_else(|| (ContractError::HelpersError))?;

                // Block the liquidation if the oracle is invalid or the oracle and mark are too divergent
                let mark_price_before = market.amm.mark_price()?;

                let oracle_status = get_oracle_status(
                    deps,
                    &market.amm,
                    &oracle_guard_rails,
                    n,
                    Some(mark_price_before),
                )?;

                let market_partial_margin_requirement: Uint128;
                let market_maintenance_margin_requirement: Uint128;
                let mut close_position_slippage = None;
                if oracle_status.is_valid
                    && use_oracle_price_for_margin_calculation(
                        oracle_status.oracle_mark_spread_pct.i128(),
                        &oracle_guard_rails,
                    )?
                {
                    let exit_slippage = calculate_slippage(
                        amm_position_base_asset_value,
                        Uint128::from( m.base_asset_amount.i128().unsigned_abs()),
                        mark_price_before.u128() as i128,
                    )?;
                    close_position_slippage = Some(exit_slippage);

                    let oracle_exit_price = oracle_status
                        .price_data
                        .price.i128()
                        .checked_add(exit_slippage)
                        .ok_or_else(|| (ContractError::HelpersError))?;

                    let (oracle_position_base_asset_value, oracle_position_unrealized_pnl) =
                        calculate_base_asset_value_and_pnl_with_oracle_price(
                            &m,
                            oracle_exit_price,
                        )?;

                    let oracle_provides_better_pnl =
                        oracle_position_unrealized_pnl > amm_position_unrealized_pnl;
                    if oracle_provides_better_pnl {
                        adjusted_unrealized_pnl = adjusted_unrealized_pnl
                            .checked_add(oracle_position_unrealized_pnl)
                            .ok_or_else(|| (ContractError::HelpersError))?;

                        market_partial_margin_requirement = (oracle_position_base_asset_value)
                            .checked_mul(market.margin_ratio_partial.into())?;

                        partial_margin_requirement = partial_margin_requirement
                            .checked_add(market_partial_margin_requirement)?;

                        market_maintenance_margin_requirement = oracle_position_base_asset_value
                            .checked_mul(market.margin_ratio_maintenance.into())?;

                        maintenance_margin_requirement = maintenance_margin_requirement
                            .checked_add(market_maintenance_margin_requirement)?;
                    } else {
                        adjusted_unrealized_pnl = adjusted_unrealized_pnl
                            .checked_add(amm_position_unrealized_pnl)
                            .ok_or_else(|| (ContractError::HelpersError))?;

                        market_partial_margin_requirement = (amm_position_base_asset_value)
                            .checked_mul(market.margin_ratio_partial.into())?;

                        partial_margin_requirement = partial_margin_requirement
                            .checked_add(market_partial_margin_requirement)?;

                        market_maintenance_margin_requirement = amm_position_base_asset_value
                            .checked_mul(market.margin_ratio_maintenance.into())?;

                        maintenance_margin_requirement = maintenance_margin_requirement
                            .checked_add(market_maintenance_margin_requirement)?;
                    }
                } else {
                    adjusted_unrealized_pnl = adjusted_unrealized_pnl
                        .checked_add(amm_position_unrealized_pnl)
                        .ok_or_else(|| (ContractError::HelpersError))?;

                    market_partial_margin_requirement = (amm_position_base_asset_value)
                        .checked_mul(market.margin_ratio_partial.into())?;

                    partial_margin_requirement = partial_margin_requirement
                        .checked_add(market_partial_margin_requirement)?;

                    market_maintenance_margin_requirement = amm_position_base_asset_value
                        .checked_mul(market.margin_ratio_maintenance.into())?;

                    maintenance_margin_requirement = maintenance_margin_requirement
                        .checked_add(market_maintenance_margin_requirement)?;
                }

                market_statuses.push(MarketStatus {
                    market_index: n,
                    partial_margin_requirement: market_partial_margin_requirement.div(MARGIN_PRECISION),
                    maintenance_margin_requirement: market_maintenance_margin_requirement
                        .div(MARGIN_PRECISION),
                    base_asset_value: amm_position_base_asset_value,
                    mark_price_before,
                    oracle_status,
                    close_position_slippage,
                });
            }
            Err(_) => todo!(),
        }
    }

    partial_margin_requirement = partial_margin_requirement
        .checked_div(MARGIN_PRECISION)?;

    maintenance_margin_requirement = maintenance_margin_requirement
        .checked_div(MARGIN_PRECISION)?;

    let total_collateral = calculate_updated_collateral(user.collateral, unrealized_pnl)?;
    let adjusted_total_collateral =
        calculate_updated_collateral(user.collateral, adjusted_unrealized_pnl)?;

    let requires_partial_liquidation = adjusted_total_collateral < partial_margin_requirement;
    let requires_full_liquidation = adjusted_total_collateral < maintenance_margin_requirement;

    let liquidation_type = if requires_full_liquidation {
        LiquidationType::FULL
    } else if requires_partial_liquidation {
        LiquidationType::PARTIAL
    } else {
        LiquidationType::NONE
    };

    let margin_requirement = match liquidation_type {
        LiquidationType::FULL => maintenance_margin_requirement,
        LiquidationType::PARTIAL => partial_margin_requirement,
        LiquidationType::NONE => partial_margin_requirement,
    };

    // Sort the market statuses such that we close the markets with biggest margin requirements first
    if liquidation_type == LiquidationType::FULL {
        market_statuses.sort_by(|a, b| {
            b.maintenance_margin_requirement
                .cmp(&a.maintenance_margin_requirement)
        });
    } else if liquidation_type == LiquidationType::PARTIAL {
        market_statuses.sort_by(|a, b| {
            b.partial_margin_requirement
                .cmp(&a.partial_margin_requirement)
        });
    }

    let margin_ratio = if base_asset_value.is_zero() {
        Uint128::MAX
    } else {
        total_collateral
            .checked_mul(MARGIN_PRECISION)?
            .checked_div(base_asset_value)?
    };

    Ok(LiquidationStatus {
        liquidation_type,
        margin_requirement,
        total_collateral,
        unrealized_pnl,
        adjusted_total_collateral,
        base_asset_value,
        market_statuses,
        margin_ratio,
    })
}

pub fn validate_margin(
    margin_ratio_initial: u32,
    margin_ratio_partial: u32,
    margin_ratio_maintenance: u32,
) -> Result<bool, ContractError> {
    if !(MINIMUM_MARGIN_RATIO.u128()..=MAXIMUM_MARGIN_RATIO.u128()).contains(&(margin_ratio_initial as u128)) {
        return Err(ContractError::InvalidMarginRatio);
    }

    if margin_ratio_initial < margin_ratio_partial {
        return Err(ContractError::InvalidMarginRatio);
    }

    if !(MINIMUM_MARGIN_RATIO.u128()..=MAXIMUM_MARGIN_RATIO.u128()).contains(&(margin_ratio_partial as u128)) {
        return Err(ContractError::InvalidMarginRatio);
    }

    if margin_ratio_partial < margin_ratio_maintenance {
        return Err(ContractError::InvalidMarginRatio);
    }

    if !(MINIMUM_MARGIN_RATIO.u128()..=MAXIMUM_MARGIN_RATIO.u128()).contains(&(margin_ratio_maintenance as u128)) {
        return Err(ContractError::InvalidMarginRatio);
    }

    Ok(true)
}
