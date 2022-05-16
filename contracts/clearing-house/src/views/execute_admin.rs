use crate::controller;
use crate::helpers;
use crate::package::history::HistoryExecuteMsg;
use crate::states::constants::*;
use crate::states::history::*;
use crate::ContractError;

use crate::states::market::{Amm, Market, MARKETS};
use crate::states::state::OrderState;
use crate::states::state::State;
use crate::states::state::FEESTRUCTURE;
use crate::states::state::ORACLEGUARDRAILS;
use crate::states::state::ORDERSTATE;
use crate::states::state::STATE;

use crate::package::helper::addr_validate_to_lower;
use crate::package::helper::VaultInterface;
use crate::package::number::Number128;
use crate::package::types::OraclePriceData;
use crate::package::types::{FeeStructure, OracleGuardRails, OracleSource};
use cosmwasm_std::{
    to_binary, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, Uint128, WasmMsg,
};

pub fn try_initialize_market(
    mut deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    market_index: u64,
    market_name: String,
    amm_base_asset_reserve: Uint128,
    amm_quote_asset_reserve: Uint128,
    amm_periodicity: u64,
    amm_peg_multiplier: Uint128,
    oracle_source_code: u8,
    margin_ratio_initial: u32,
    margin_ratio_partial: u32,
    margin_ratio_maintenance: u32,
) -> Result<Response, ContractError> {
    let now = env.block.time.seconds();

    let state = STATE.load(deps.storage)?;
    if state.admin != _info.sender.clone() {
        return Err(ContractError::Unauthorized {});
    }
    let existing_market = MARKETS.load(deps.storage, market_index.to_string());
    if existing_market.is_ok() {
        return Err(ContractError::MarketIndexAlreadyInitialized {});
    }
    if amm_base_asset_reserve != amm_quote_asset_reserve {
        return Err(ContractError::InvalidInitialPeg.into());
    }

    let init_mark_price = helpers::amm::calculate_price(
        amm_quote_asset_reserve,
        amm_base_asset_reserve,
        amm_peg_multiplier,
    )?;
    let oracle_source = match oracle_source_code {
        0 => OracleSource::Oracle,
        _ => OracleSource::Oracle,
    };

    let a = Amm {
        oracle: state.oracle,
        oracle_source,
        base_asset_reserve: amm_base_asset_reserve,
        quote_asset_reserve: amm_quote_asset_reserve,
        cumulative_repeg_rebate_long: Uint128::zero(),
        cumulative_repeg_rebate_short: Uint128::zero(),
        cumulative_funding_rate_long: Number128::zero(),
        cumulative_funding_rate_short: Number128::zero(),
        last_funding_rate: Number128::zero(),
        last_funding_rate_ts: now,
        funding_period: amm_periodicity,
        last_oracle_price_twap: Number128::zero(),
        last_mark_price_twap: init_mark_price,
        last_mark_price_twap_ts: now,
        sqrt_k: amm_base_asset_reserve,
        peg_multiplier: amm_peg_multiplier,
        total_fee: Uint128::zero(),
        total_fee_minus_distributions: Uint128::zero(),
        total_fee_withdrawn: Uint128::zero(),
        minimum_quote_asset_trade_size: Uint128::from(10000000 as u128),
        last_oracle_price_twap_ts: now,
        last_oracle_price: Number128::zero(),
        minimum_base_asset_trade_size: Uint128::from(10000000 as u128),
    };

    // Verify there's no overflow
    let _k = amm_base_asset_reserve.checked_mul(amm_quote_asset_reserve)?;

  

    // let last_oracle_price_twap = a.get_oracle_twap()?;

    controller::margin::validate_margin(
        margin_ratio_initial,
        margin_ratio_partial,
        margin_ratio_maintenance,
    )?;
    let market = Market {
        market_name: market_name,
        initialized: true,
        base_asset_amount_long: Number128::zero(),
        base_asset_amount_short: Number128::zero(),
        base_asset_amount: Number128::zero(),
        open_interest: Uint128::zero(),
        margin_ratio_initial, // unit is 20% (+2 decimal places)
        margin_ratio_partial,
        margin_ratio_maintenance,
        amm: a.clone(),
    };
    MARKETS.save(deps.storage, market_index.to_string(), &market)?;
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        state.markets_length += 1;
        Ok(state)
    })?;
    let OraclePriceData {
        // price: oracle_price,
        ..
    } = a.get_oracle_price(&mut deps, market_index)?;
    Ok(Response::new().add_attribute("method", "try_initialize_market"))
}

pub fn try_move_amm_price(
    mut deps: DepsMut,
    base_asset_reserve: Uint128,
    quote_asset_reserve: Uint128,
    market_index: u64,
) -> Result<Response, ContractError> {
    controller::amm::move_price(
        &mut deps,
        market_index,
        base_asset_reserve,
        quote_asset_reserve,
    )?;
    Ok(Response::new().add_attribute("method", "try_move_amm_price"))
}

pub fn try_withdraw_fees(
    deps: DepsMut,
    info: MessageInfo,
    market_index: u64,
    amount: u64,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if state.admin != info.sender.clone() {
        return Err(ContractError::Unauthorized {});
    }
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;

    // A portion of fees must always remain in protocol to be used to keep markets optimal
    let max_withdraw = market
        .amm
        .total_fee
        .checked_mul(SHARE_OF_FEES_ALLOCATED_TO_CLEARING_HOUSE_NUMERATOR)?
        .checked_div(SHARE_OF_FEES_ALLOCATED_TO_CLEARING_HOUSE_DENOMINATOR)?
        .checked_sub(market.amm.total_fee_withdrawn)?;

    if amount as u128 > max_withdraw.u128() {
        return Err(ContractError::AdminWithdrawTooLarge.into());
    }

    //todo recipient who? is it only admin function
    let message: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: state.collateral_vault.to_string(),
        msg: to_binary(&VaultInterface::Withdraw {
            to_address: info.sender.clone(),
            amount: Uint128::from(amount),
        })?,
        funds: vec![],
    });

    market.amm.total_fee_withdrawn = market
        .amm
        .total_fee_withdrawn
        .checked_add(Uint128::from(amount))?;

    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market) },
    )?;

    Ok(Response::new()
        .add_message(message)
        .add_attribute("method", "try_withdraw_fees"))
}

pub fn try_withdraw_from_insurance_vault_to_market(
    deps: DepsMut,
    info: MessageInfo,
    market_index: u64,
    amount: u64,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if state.admin != info.sender.clone() {
        return Err(ContractError::Unauthorized {});
    }

    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    market.amm.total_fee_minus_distributions = market
        .amm
        .total_fee_minus_distributions
        .checked_add(Uint128::from(amount))?;

    let message: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: state.insurance_vault.to_string(),
        msg: to_binary(&VaultInterface::Withdraw {
            to_address: state.collateral_vault.clone(),
            amount: Uint128::from(amount),
        })?,
        funds: vec![],
    });
    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market) },
    )?;
    Ok(Response::new()
        .add_message(message)
        .add_attribute("method", "try_withdraw_from_insurance_vault_to_market"))
}

pub fn try_repeg_amm_curve(
    mut deps: DepsMut,
    env: Env,
    new_peg_candidate: Uint128,
    market_index: u64,
) -> Result<Response, ContractError> {
    let now = env.block.time.seconds();
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    
    let OraclePriceData {
        price: oracle_price,
        ..
    } = market.amm.get_oracle_price(&mut deps, market_index)?;
    market = MARKETS.load(deps.storage, market_index.to_string())?;
    
    let peg_multiplier_before = market.amm.peg_multiplier;
    let base_asset_reserve_before = market.amm.base_asset_reserve;
    let quote_asset_reserve_before = market.amm.quote_asset_reserve;
    let sqrt_k_before = market.amm.sqrt_k;

    // let price_oracle = state.oracle;

    let adjustment_cost =
        controller::repeg::repeg(&mut deps, market_index, new_peg_candidate).unwrap();
    let peg_multiplier_after = market.amm.peg_multiplier;
    let base_asset_reserve_after = market.amm.base_asset_reserve;
    let quote_asset_reserve_after = market.amm.quote_asset_reserve;
    let sqrt_k_after = market.amm.sqrt_k;
    let c = CurveRecord {
        ts: now,
        market_index,
        peg_multiplier_before,
        base_asset_reserve_before,
        quote_asset_reserve_before,
        sqrt_k_before,
        peg_multiplier_after,
        base_asset_reserve_after,
        quote_asset_reserve_after,
        sqrt_k_after,
        base_asset_amount_long: Uint128::from(market.base_asset_amount_long.i128().unsigned_abs()),
        base_asset_amount_short: Uint128::from(
            market.base_asset_amount_short.i128().unsigned_abs(),
        ),
        base_asset_amount: market.base_asset_amount,
        open_interest: market.open_interest,
        total_fee: market.amm.total_fee,
        total_fee_minus_distributions: market.amm.total_fee_minus_distributions,
        adjustment_cost: Number128::new(adjustment_cost),
        oracle_price,
    };
    let state = STATE.load(deps.storage)?;
    let message: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: state.history_contract.clone().to_string(),
        msg: to_binary(&HistoryExecuteMsg::RecordCurve { c })?,
        funds: vec![],
    });
    Ok(Response::new()
        .add_message(message)
        .add_attribute("method", "try_repeg_amm_curve"))
}

pub fn try_update_amm_oracle_twap(
    deps: DepsMut,
    env: Env,
    market_index: u64,
) -> Result<Response, ContractError> {
    let now = env.block.time.seconds();
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    // todo get_oracle_twap is not defined yet
    let oracle_twap = market.amm.get_oracle_twap()?;

    if let Some(oracle_twap) = oracle_twap {
        let oracle_mark_gap_before = (market.amm.last_mark_price_twap.u128() as i128)
            .checked_sub(market.amm.last_oracle_price_twap.i128())
            .ok_or_else(|| (ContractError::MathError))?;

        let oracle_mark_gap_after = (market.amm.last_mark_price_twap.u128() as i128)
            .checked_sub(oracle_twap)
            .ok_or_else(|| (ContractError::MathError))?;

        if (oracle_mark_gap_after > 0 && oracle_mark_gap_before < 0)
            || (oracle_mark_gap_after < 0 && oracle_mark_gap_before > 0)
        {
            market.amm.last_oracle_price_twap =
                Number128::new(market.amm.last_mark_price_twap.u128() as i128);
            market.amm.last_oracle_price_twap_ts = now;
        } else if oracle_mark_gap_after.unsigned_abs() <= oracle_mark_gap_before.unsigned_abs() {
            market.amm.last_oracle_price_twap = Number128::new(oracle_twap);
            market.amm.last_oracle_price_twap_ts = now;
        } else {
            return Err(ContractError::OracleMarkSpreadLimit.into());
        }
    } else {
        return Err(ContractError::InvalidOracle.into());
    }

    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market) },
    )?;

    Ok(Response::new().add_attribute("method", "try_update_amm_oracle_twap"))
}

pub fn try_reset_amm_oracle_twap(
    mut deps: DepsMut,
    env: Env,
    market_index: u64,
) -> Result<Response, ContractError> {
    let now = env.block.time.seconds();
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    let oracle_guard_rails = ORACLEGUARDRAILS.load(deps.storage)?;
    let oracle_price_data = market.amm.get_oracle_price(&mut deps, market_index)?;
    market = MARKETS.load(deps.storage, market_index.to_string())?;

    let is_oracle_valid =
        helpers::amm::is_oracle_valid(&market.amm, &oracle_price_data, &oracle_guard_rails)?;

    if !is_oracle_valid {
        market.amm.last_oracle_price_twap =
            Number128::new(market.amm.last_mark_price_twap.u128() as i128);
        market.amm.last_oracle_price_twap_ts = now;
    }
    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market) },
    )?;
    Ok(Response::new().add_attribute("method", "try_reset_amm_oracle_twap"))
}

pub fn try_update_funding_rate(
    mut deps: DepsMut,
    env: Env,
    market_index: u64,
) -> Result<Response, ContractError> {
    let now = env.block.time.seconds();
    let funding_paused = STATE.load(deps.storage).unwrap().funding_paused;
    let state = STATE.load(deps.storage)?;
    let f = controller::funding::update_funding_rate(
        &mut deps,
        market_index,
        now,
        funding_paused,
        None,
    )?;

    let message: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: state.history_contract.clone().to_string(),
        msg: to_binary(&HistoryExecuteMsg::RecordFundingRate { f: f.unwrap() })?,
        funds: vec![],
    });
    Ok(Response::new()
        .add_message(message)
        .add_attribute("method", "try_update_funding_rate"))
}

pub fn try_update_k(
    mut deps: DepsMut,
    env: Env,
    market_index: u64,
    sqrt_k: Uint128,
) -> Result<Response, ContractError> {
    let now = env.block.time.seconds();
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    let state = STATE.load(deps.storage)?;

    let base_asset_amount_long = Uint128::from(market.base_asset_amount_long.i128().unsigned_abs());
    let base_asset_amount_short =
        Uint128::from(market.base_asset_amount_short.i128().unsigned_abs());
    let base_asset_amount = market.base_asset_amount.i128().clone();
    let open_interest = market.open_interest.clone();

    let price_before = helpers::amm::calculate_price(
        market.amm.quote_asset_reserve,
        market.amm.base_asset_reserve,
        market.amm.peg_multiplier,
    )?;

    let peg_multiplier_before = market.amm.peg_multiplier;
    let base_asset_reserve_before = market.amm.base_asset_reserve;
    let quote_asset_reserve_before = market.amm.quote_asset_reserve;
    let sqrt_k_before = market.amm.sqrt_k;

    let adjustment_cost = controller::amm::adjust_k_cost(&mut deps, market_index, sqrt_k)?;

    if adjustment_cost > 0 {
        let max_cost = market
            .amm
            .total_fee_minus_distributions
            .checked_sub(market.amm.total_fee_withdrawn)?;
        if adjustment_cost.unsigned_abs() > max_cost.u128() {
            return Err(ContractError::InvalidUpdateK.into());
        } else {
            market.amm.total_fee_minus_distributions = market
                .amm
                .total_fee_minus_distributions
                .checked_sub(Uint128::from(adjustment_cost.unsigned_abs()))?;
        }
    } else {
        market.amm.total_fee_minus_distributions = market
            .amm
            .total_fee_minus_distributions
            .checked_add(Uint128::from(adjustment_cost.unsigned_abs()))?;
    }

    let amm = &market.amm;
    let price_after = helpers::amm::calculate_price(
        amm.quote_asset_reserve,
        amm.base_asset_reserve,
        amm.peg_multiplier,
    )?;

    let price_change_too_large = (price_before.u128() as i128)
        .checked_sub(price_after.u128() as i128)
        .ok_or_else(|| ContractError::MathError {})?
        .unsigned_abs()
        .gt(&UPDATE_K_ALLOWED_PRICE_CHANGE.u128());

    if price_change_too_large {
        return Err(ContractError::InvalidUpdateK.into());
    }

    let peg_multiplier_after = amm.peg_multiplier;
    let base_asset_reserve_after = amm.base_asset_reserve;
    let quote_asset_reserve_after = amm.quote_asset_reserve;
    let sqrt_k_after = amm.sqrt_k;

    let total_fee = amm.total_fee;
    let total_fee_minus_distributions = amm.total_fee_minus_distributions;

    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market.clone()) },
    )?;

    let OraclePriceData {
        price: oracle_price,
        ..
    } = market.amm.get_oracle_price(&mut deps, market_index)?;

    let c = CurveRecord {
        ts: now,
        market_index,
        peg_multiplier_before,
        base_asset_reserve_before,
        quote_asset_reserve_before,
        sqrt_k_before,
        peg_multiplier_after,
        base_asset_reserve_after,
        quote_asset_reserve_after,
        sqrt_k_after,
        base_asset_amount_long,
        base_asset_amount_short,
        base_asset_amount: Number128::new(base_asset_amount),
        open_interest,
        adjustment_cost: Number128::new(adjustment_cost),
        total_fee,
        total_fee_minus_distributions,
        oracle_price,
    };

    let message = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: state.history_contract.to_string(),
        funds: vec![],
        msg: to_binary(&HistoryExecuteMsg::RecordCurve { c: c })?,
    });

    Ok(Response::new()
        .add_message(message)
        .add_attribute("method", "try_update_k"))
}

pub fn try_update_margin_ratio(
    deps: DepsMut,
    info: MessageInfo,
    market_index: u64,
    margin_ratio_initial: u32,
    margin_ratio_partial: u32,
    margin_ratio_maintenance: u32,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if state.admin != info.sender.clone() {
        return Err(ContractError::Unauthorized {});
    }

    controller::margin::validate_margin(
        margin_ratio_initial,
        margin_ratio_partial,
        margin_ratio_maintenance,
    )?;
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> {
            market.margin_ratio_initial = margin_ratio_initial;
            market.margin_ratio_partial = margin_ratio_partial;
            market.margin_ratio_maintenance = margin_ratio_maintenance;
            Ok(market)
        },
    )?;
    Ok(Response::new().add_attribute("method", "try_update_margin_ratio"))
}

pub fn try_update_partial_liquidation_close_percentage(
    deps: DepsMut,
    info: MessageInfo,
    value: Decimal,
) -> Result<Response, ContractError> {
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        if state.admin != info.sender.clone() {
            return Err(ContractError::Unauthorized {});
        }
        state.partial_liquidation_close_percentage = value;
        Ok(state)
    })?;

    Ok(Response::new().add_attribute("method", "try_update_partial_liquidation_close_percentage"))
}

pub fn try_update_partial_liquidation_penalty_percentage(
    deps: DepsMut,
    info: MessageInfo,
    value: Decimal,
) -> Result<Response, ContractError> {
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        if state.admin != info.sender.clone() {
            return Err(ContractError::Unauthorized {});
        }

        state.partial_liquidation_penalty_percentage = value;
        Ok(state)
    })?;
    Ok(Response::new().add_attribute(
        "method",
        "try_update_partial_liquidation_penalty_percentage",
    ))
}

pub fn try_update_full_liquidation_penalty_percentage(
    deps: DepsMut,
    info: MessageInfo,
    value: Decimal,
) -> Result<Response, ContractError> {
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        if state.admin != info.sender.clone() {
            return Err(ContractError::Unauthorized {});
        }
        state.full_liquidation_penalty_percentage = value;
        Ok(state)
    })?;
    Ok(Response::new().add_attribute("method", "try_update_full_liquidation_penalty_percentage"))
}

pub fn try_update_partial_liquidation_liquidator_share_denominator(
    deps: DepsMut,
    info: MessageInfo,
    denominator: u64,
) -> Result<Response, ContractError> {
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        if state.admin != info.sender.clone() {
            return Err(ContractError::Unauthorized {});
        }
        state.partial_liquidation_liquidator_share_denominator = denominator;
        Ok(state)
    })?;
    Ok(Response::new().add_attribute(
        "method",
        "try_update_partial_liquidation_liquidator_share_denominator",
    ))
}

pub fn try_update_full_liquidation_liquidator_share_denominator(
    deps: DepsMut,
    info: MessageInfo,
    denominator: u64,
) -> Result<Response, ContractError> {
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        if state.admin != info.sender.clone() {
            return Err(ContractError::Unauthorized {});
        }
        state.full_liquidation_liquidator_share_denominator = denominator;
        Ok(state)
    })?;
    Ok(Response::new().add_attribute(
        "method",
        "try_update_full_liquidation_liquidator_share_denominator",
    ))
}

pub fn try_update_fee(
    deps: DepsMut,
    info: MessageInfo,
    fee: Decimal,
    first_tier_minimum_balance: Uint128,
    first_tier_discount: Decimal,
    second_tier_minimum_balance: Uint128,
    second_tier_discount: Decimal,
    third_tier_minimum_balance: Uint128,
    third_tier_discount: Decimal,
    fourth_tier_minimum_balance: Uint128,
    fourth_tier_discount: Decimal,
    referrer_reward: Decimal,
    referee_discount: Decimal,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if state.admin != info.sender.clone() {
        return Err(ContractError::Unauthorized {});
    }
    let fee_structure = FeeStructure {
        fee,
        first_tier_minimum_balance,
        second_tier_minimum_balance,
        third_tier_minimum_balance,
        fourth_tier_minimum_balance,
        first_tier_discount,
        second_tier_discount,
        third_tier_discount,
        fourth_tier_discount,
        referrer_reward,
        referee_discount,
    };
    FEESTRUCTURE.update(
        deps.storage,
        |mut _f| -> Result<FeeStructure, ContractError> { Ok(fee_structure) },
    )?;
    Ok(Response::new().add_attribute("method", "try_update_fee"))
}

pub fn try_update_order_state_structure(
    deps: DepsMut,
    info: MessageInfo,
    min_order_quote_asset_amount: Uint128,
    reward: Decimal,
    time_based_reward_lower_bound: Uint128,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if state.admin != info.sender.clone() {
        return Err(ContractError::Unauthorized {});
    }
    let order_state = OrderState {
        min_order_quote_asset_amount,
        reward,
        time_based_reward_lower_bound,
    };
    ORDERSTATE.update(
        deps.storage,
        |mut _s| -> Result<OrderState, ContractError> { Ok(order_state) },
    )?;
    Ok(Response::new().add_attribute("method", "try_update_order_filler_reward_structure"))
}

pub fn try_update_market_oracle(
    deps: DepsMut,
    info: MessageInfo,
    market_index: u64,
    oracle: String,
    oracle_source_code: u8,
) -> Result<Response, ContractError> {
    let oracle_source = match oracle_source_code {
        0 => OracleSource::Oracle,
        _ => OracleSource::Oracle,
    };
    let state = STATE.load(deps.storage)?;
    if state.admin != info.sender.clone() {
        return Err(ContractError::Unauthorized {});
    }
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    market.amm.oracle = addr_validate_to_lower(deps.api, &oracle)?;
    market.amm.oracle_source = oracle_source;
    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market) },
    )?;
    Ok(Response::new().add_attribute("method", "try_update_market_oracle"))
}

pub fn try_update_oracle_guard_rails(
    deps: DepsMut,
    info: MessageInfo,
    use_for_liquidations: bool,
    mark_oracle_divergence: Decimal,
    slots_before_stale: i64,
    confidence_interval_max_size: Uint128,
    too_volatile_ratio: i128,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if state.admin != info.sender.clone() {
        return Err(ContractError::Unauthorized {});
    }

    let oracle_gr = OracleGuardRails {
        use_for_liquidations,
        mark_oracle_divergence,
        slots_before_stale,
        confidence_interval_max_size,
        too_volatile_ratio: Number128::new(too_volatile_ratio),
    };
    ORACLEGUARDRAILS.update(
        deps.storage,
        |mut _o| -> Result<OracleGuardRails, ContractError> { Ok(oracle_gr) },
    )?;

    Ok(Response::new().add_attribute("method", "try_update_oracle_guard_rails"))
}

pub fn try_update_max_deposit(
    deps: DepsMut,
    info: MessageInfo,
    max_deposit: Uint128,
) -> Result<Response, ContractError> {
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        if state.admin != info.sender.clone() {
            return Err(ContractError::Unauthorized {});
        }
        state.max_deposit = max_deposit;
        Ok(state)
    })?;
    Ok(Response::new().add_attribute("method", "try_max_deposit"))
}

pub fn try_update_admin(
    deps: DepsMut,
    info: MessageInfo,
    admin: String,
) -> Result<Response, ContractError> {
    let api = deps.api;
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        if state.admin != info.sender.clone() {
            return Err(ContractError::Unauthorized {});
        }
        state.admin = addr_validate_to_lower(api, &admin)?;
        Ok(state)
    })?;
    Ok(Response::new().add_attribute("method", "try_max_deposit"))
}

pub fn try_update_exchange_paused(
    deps: DepsMut,
    info: MessageInfo,
    exchange_paused: bool,
) -> Result<Response, ContractError> {
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        if state.admin != info.sender.clone() {
            return Err(ContractError::Unauthorized {});
        }

        state.exchange_paused = exchange_paused;
        Ok(state)
    })?;
    Ok(Response::new().add_attribute("method", "try_exchange_paused"))
}

pub fn try_disable_admin_control_prices(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        if state.admin != info.sender.clone() {
            return Err(ContractError::Unauthorized {});
        }
        state.admin_controls_prices = false;
        Ok(state)
    })?;
    Ok(Response::new().add_attribute("method", "try_disable_admin_control_prices"))
}
pub fn try_update_funding_paused(
    deps: DepsMut,
    info: MessageInfo,
    funding_paused: bool,
) -> Result<Response, ContractError> {
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        if state.admin != info.sender.clone() {
            return Err(ContractError::Unauthorized {});
        }
        state.funding_paused = funding_paused;
        Ok(state)
    })?;
    Ok(Response::new().add_attribute("method", "try_update_funding_paused"))
}

pub fn try_update_market_minimum_quote_asset_trade_size(
    deps: DepsMut,
    info: MessageInfo,
    market_index: u64,
    minimum_trade_size: Uint128,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if state.admin != info.sender.clone() {
        return Err(ContractError::Unauthorized {});
    }
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> {
            market.amm.minimum_quote_asset_trade_size = minimum_trade_size;
            Ok(market)
        },
    )?;
    Ok(Response::new().add_attribute("method", "try_update_market_minimum_quote_asset_trade_size"))
}

pub fn try_update_market_minimum_base_asset_trade_size(
    deps: DepsMut,
    info: MessageInfo,
    market_index: u64,
    minimum_trade_size: Uint128,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if state.admin != info.sender.clone() {
        return Err(ContractError::Unauthorized {});
    }

    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |m| -> Result<_, ContractError> {
            match m {
                Some(mut mr) => {
                    mr.amm.minimum_base_asset_trade_size = minimum_trade_size;
                    Ok(mr)
                }
                None => return Err(ContractError::UserMaxDeposit),
            }
        },
    )?;
    Ok(Response::new().add_attribute("method", "try_update_market_minimum_base_asset_trade_size"))
}

pub fn try_update_oracle_address(
    deps: DepsMut,
    info: MessageInfo,
    oracle: String,
) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage)?;
    if state.admin != info.sender.clone() {
        return Err(ContractError::Unauthorized {});
    }

    state.oracle = addr_validate_to_lower(deps.api, &oracle)?;
    STATE.update(deps.storage, |_state| -> Result<State, ContractError> {
        Ok(state)
    })?;
    Ok(Response::new().add_attribute("method", "try_update_oracle_address"))
}

pub fn try_feeding_price(
    deps: DepsMut,
    info: MessageInfo,
    market_index: u64,
    price: Uint128,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if state.admin != info.sender.clone() {
        return Err(ContractError::Unauthorized {});
    }

    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    market.amm.last_oracle_price = Number128::new(price.u128() as i128);
    market.amm.last_oracle_price_twap = Number128::new(price.u128() as i128);
    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market) },
    )?;
    Ok(Response::new().add_attribute("method", "try_update_oracle_address"))
}

pub fn try_update_history_contract(
    deps: DepsMut,
    info: MessageInfo,
    history_contract: String,
) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage)?;
    if state.admin != info.sender.clone() {
        return Err(ContractError::Unauthorized {});
    }

    let new_history_contract = deps.api.addr_validate(&history_contract)?;
    state.history_contract = new_history_contract;
    STATE.update(deps.storage, |_s| -> Result<State, ContractError> {
        Ok(state)
    })?;
    Ok(Response::new().add_attribute("method", "try_update_history_contract"))
}
