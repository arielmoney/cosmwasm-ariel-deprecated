#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response};
use cw2::set_contract_version;
use cw_storage_plus::{Bound, PrimaryKey};

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, CurveHistoryResponse, DepositHistoryResponse, ExecuteMsg,
    FundingPaymentHistoryResponse, FundingRateHistoryResponse, InstantiateMsg, LengthResponse,
    LiquidationHistoryResponse, QueryMsg, TradeHistoryResponse,
};
use crate::package::validate::addr_validate_to_lower;
use crate::state::{
    CurveRecord, DepositRecord, FundingPaymentRecord, FundingRateRecord, Length, LiquidationRecord,
    State, TradeRecord, CURVEHISTORY, DEPOSIT_HISTORY, FUNDING_PAYMENT_HISTORY,
    FUNDING_RATE_HISTORY, LENGTH, LIQUIDATION_HISTORY, STATE, TRADE_HISTORY,
};

// iterator limits
pub const MAX_LIMIT: u32 = 20;
pub const DEFAULT_LIMIT: u32 = 10;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:my-first-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let state = State {
        owner: info.sender.clone(),
        clearing_house: Addr::unchecked(""),
    };
    STATE.save(deps.storage, &state)?;

    let length = Length {
        curve_history_length: 0,
        deposit_history_length: 0,
        funding_payment_history_length: 0,
        funding_rate_history_length: 0,
        liquidation_history_length: 0,
        trade_history_length: 0,
    };
    LENGTH.save(deps.storage, &length)?;
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateAdmin { new_admin } => try_new_admin(deps, info, new_admin),
        ExecuteMsg::UpdateClearingHouse { new_house } => {
            try_new_clearing_house(deps, info, new_house)
        }
        ExecuteMsg::RecordCurve { c } => try_record_curve(deps, info, c),
        ExecuteMsg::RecordFundingPayment { f } => try_record_funding_payment(deps, info, f),
        ExecuteMsg::RecordFundingRate { f } => try_record_funding_rate(deps, info, f),
        ExecuteMsg::RecordLiquidation { l } => try_record_liquidation(deps, info, l),
        ExecuteMsg::RecordTrade { t } => try_record_trade(deps, info, t),
        ExecuteMsg::RecordDeposit { d } => try_record_deposit(deps, info, d),
        ExecuteMsg::RecordFundingPaymentsMultiple { vecf } => {
            try_record_funding_payment_multiple(deps, info, vecf)
        } // ExecuteMsg::RecordOrder { o } => try_record_order(deps, info, o),
    }
}

fn try_record_funding_payment_multiple(
    deps: DepsMut,
    info: MessageInfo,
    vecf: Vec<FundingPaymentRecord>,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if info.sender != state.clearing_house {
        return Err(ContractError::UnauthorizedClearingHouse {});
    };

    for f in vecf.iter() {
        let mut len = LENGTH.load(deps.storage)?;
        let funding_payment_history_info_length = len
            .funding_payment_history_length
            .checked_add(1)
            .ok_or_else(|| (ContractError::MathError))?;
        len.funding_payment_history_length = funding_payment_history_info_length;
        LENGTH.update(deps.storage, |_l| -> Result<Length, ContractError> {
            Ok(len)
        })?;
        FUNDING_PAYMENT_HISTORY.save(
            deps.storage,
            (&f.user, funding_payment_history_info_length.to_string()),
            f,
        )?;
    }

    Ok(Response::new().add_attribute("method", "record_funding_payment_records_multple"))
}

// fn try_record_order(deps: DepsMut, info: MessageInfo, o: OrderRecord) -> Result<Response, ContractError>  {
//     todo!()
// }

fn try_record_deposit(
    deps: DepsMut,
    info: MessageInfo,
    d: DepositRecord,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if info.sender != state.clearing_house {
        return Err(ContractError::UnauthorizedClearingHouse {});
    };

    let mut len = LENGTH.load(deps.storage)?;
    let deposit_history_info_length = len
        .deposit_history_length
        .checked_add(1)
        .ok_or_else(|| (ContractError::MathError))?;
    len.deposit_history_length = deposit_history_info_length;
    LENGTH.update(deps.storage, |_l| -> Result<Length, ContractError> {
        Ok(len)
    })?;
    DEPOSIT_HISTORY.save(
        deps.storage,
        (&d.user, deposit_history_info_length.to_string()),
        &d,
    )?;

    Ok(Response::new().add_attribute("method", "record_deposit"))
}

fn try_record_trade(
    deps: DepsMut,
    info: MessageInfo,
    t: TradeRecord,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if info.sender != state.clearing_house {
        return Err(ContractError::UnauthorizedClearingHouse {});
    };

    let mut len = LENGTH.load(deps.storage)?;
    let trade_history_info_length = len
        .trade_history_length
        .checked_add(1)
        .ok_or_else(|| (ContractError::MathError))?;
    len.trade_history_length = trade_history_info_length;
    LENGTH.update(deps.storage, |_l| -> Result<Length, ContractError> {
        Ok(len)
    })?;
    TRADE_HISTORY.save(
        deps.storage,
        (&t.user, trade_history_info_length.to_string()),
        &t,
    )?;

    Ok(Response::new().add_attribute("method", "record_trade"))
}

fn try_record_liquidation(
    deps: DepsMut,
    info: MessageInfo,
    l: LiquidationRecord,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if info.sender != state.clearing_house {
        return Err(ContractError::UnauthorizedClearingHouse {});
    };

    let mut len = LENGTH.load(deps.storage)?;
    let liquidation_history_info_length = len
        .liquidation_history_length
        .checked_add(1)
        .ok_or_else(|| (ContractError::MathError))?;
    len.liquidation_history_length = liquidation_history_info_length;
    LENGTH.update(deps.storage, |_l| -> Result<Length, ContractError> {
        Ok(len)
    })?;
    LIQUIDATION_HISTORY.save(
        deps.storage,
        (&l.user, liquidation_history_info_length.to_string()),
        &l,
    )?;

    Ok(Response::new().add_attribute("method", "record_liquidation"))
}

fn try_record_funding_rate(
    deps: DepsMut,
    info: MessageInfo,
    f: FundingRateRecord,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if info.sender != state.clearing_house {
        return Err(ContractError::UnauthorizedClearingHouse {});
    };

    let mut len = LENGTH.load(deps.storage)?;
    let funding_rate_history_info_length = len
        .funding_rate_history_length
        .checked_add(1)
        .ok_or_else(|| (ContractError::MathError))?;
    len.funding_rate_history_length = funding_rate_history_info_length;
    LENGTH.update(deps.storage, |_l| -> Result<Length, ContractError> {
        Ok(len)
    })?;
    FUNDING_RATE_HISTORY.save(
        deps.storage,
        funding_rate_history_info_length.to_string(),
        &f,
    )?;

    Ok(Response::new().add_attribute("method", "record_funding_rate"))
}

fn try_record_funding_payment(
    deps: DepsMut,
    info: MessageInfo,
    f: FundingPaymentRecord,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if info.sender != state.clearing_house {
        return Err(ContractError::UnauthorizedClearingHouse {});
    };

    let mut len = LENGTH.load(deps.storage)?;
    let funding_payment_history_info_length = len
        .funding_payment_history_length
        .checked_add(1)
        .ok_or_else(|| (ContractError::MathError))?;
    len.funding_payment_history_length = funding_payment_history_info_length;
    LENGTH.update(deps.storage, |_l| -> Result<Length, ContractError> {
        Ok(len)
    })?;
    FUNDING_PAYMENT_HISTORY.save(
        deps.storage,
        (&f.user, funding_payment_history_info_length.to_string()),
        &f,
    )?;

    Ok(Response::new().add_attribute("method", "record_funding_payment"))
}

fn try_record_curve(
    deps: DepsMut,
    info: MessageInfo,
    c: CurveRecord,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if info.sender != state.clearing_house {
        return Err(ContractError::UnauthorizedClearingHouse {});
    };

    let mut len = LENGTH.load(deps.storage)?;
    let curve_history_info_length = len
        .curve_history_length
        .checked_add(1)
        .ok_or_else(|| (ContractError::MathError))?;
    len.curve_history_length = curve_history_info_length;
    LENGTH.update(deps.storage, |_l| -> Result<Length, ContractError> {
        Ok(len)
    })?;
    CURVEHISTORY.save(deps.storage, curve_history_info_length.to_string(), &c)?;

    Ok(Response::new().add_attribute("method", "record_curve"))
}

fn try_new_clearing_house(
    deps: DepsMut,
    info: MessageInfo,
    new_house: String,
) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage)?;
    if info.sender != state.owner {
        return Err(ContractError::UnauthorizedClearingHouse {});
    };

    state.clearing_house = deps.api.addr_validate(&new_house)?;

    STATE.update(deps.storage, |_s| -> Result<State, ContractError> {
        Ok(state)
    })?;

    Ok(Response::new().add_attribute("method", "new_clearing_house"))
}

fn try_new_admin(
    deps: DepsMut,
    info: MessageInfo,
    new_admin: String,
) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage)?;
    if info.sender != state.owner {
        return Err(ContractError::UnauthorizedClearingHouse {});
    };

    state.owner = deps.api.addr_validate(&new_admin)?;

    STATE.update(deps.storage, |_s| -> Result<State, ContractError> {
        Ok(state)
    })?;

    Ok(Response::new().add_attribute("method", "new_admin"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::GetConfig {} => Ok(to_binary(&get_config(deps)?)?),
        QueryMsg::GetLength {} => Ok(to_binary(&get_length(deps)?)?),
        QueryMsg::GetCurveHistory { start_after, limit } => {
            Ok(to_binary(&get_curve_history(deps, start_after, limit)?)?)
        }
        QueryMsg::GetDepositHistory {
            user_address,
            start_after,
            limit,
        } => Ok(to_binary(&get_deposit_history(
            deps,
            user_address,
            start_after,
            limit,
        )?)?),
        QueryMsg::GetFundingPaymentHistory {
            user_address,
            start_after,
            limit,
        } => Ok(to_binary(&get_funding_payment_history(
            deps,
            user_address,
            start_after,
            limit,
        )?)?),
        QueryMsg::GetFundingRateHistory { start_after, limit } => Ok(to_binary(
            &get_funding_rate_history(deps, start_after, limit)?,
        )?),
        QueryMsg::GetLiquidationHistory {
            user_address,
            start_after,
            limit,
        } => Ok(to_binary(&get_liquidation_history(
            deps,
            user_address,
            start_after,
            limit,
        )?)?),
        QueryMsg::GetTradeHistory { start_after, limit } => {
            Ok(to_binary(&get_trade_history(deps, start_after, limit)?)?)
        }
        QueryMsg::GetTradeHistoryByAddress {
            user_address,
            start_after,
            limit,
        } => Ok(to_binary(&get_trade_history_by_user(
            deps,
            user_address,
            start_after,
            limit,
        )?)?),
    }
}

fn get_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let state = STATE.load(deps.storage)?;
    let config = ConfigResponse {
        clearing_house: state.clearing_house,
        owner: state.owner,
    };
    Ok(config)
}

pub fn get_length(deps: Deps) -> Result<LengthResponse, ContractError> {
    let len = LENGTH.load(deps.storage)?;
    let length = LengthResponse {
        curve_history_length: len.curve_history_length,
        deposit_history_length: len.deposit_history_length,
        funding_payment_history_length: len.funding_payment_history_length,
        funding_rate_history_length: len.funding_rate_history_length,
        liquidation_history_length: len.liquidation_history_length,
        trade_history_length: len.trade_history_length,
    };
    Ok(length)
}

pub fn get_curve_history(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<CurveHistoryResponse>, ContractError> {
    let chl = LENGTH.load(deps.storage)?.curve_history_length;
    let mut curves: Vec<CurveHistoryResponse> = vec![];
    if chl > 0 {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
        let start = start_after
            .map(|start| start.joined_key())
            .map(Bound::Exclusive);

        curves = CURVEHISTORY
            .range(deps.storage, start, None, Order::Descending)
            .filter_map(|curve_record| {
                curve_record.ok().map(|curve| CurveHistoryResponse {
                    ts: curve.1.ts,
                    market_index: curve.1.market_index,
                    peg_multiplier_before: curve.1.peg_multiplier_before,
                    base_asset_reserve_before: curve.1.base_asset_reserve_before,
                    quote_asset_reserve_before: curve.1.quote_asset_reserve_before,
                    sqrt_k_before: curve.1.sqrt_k_before,
                    peg_multiplier_after: curve.1.peg_multiplier_after,
                    base_asset_reserve_after: curve.1.base_asset_reserve_after,
                    quote_asset_reserve_after: curve.1.quote_asset_reserve_after,
                    sqrt_k_after: curve.1.sqrt_k_after,
                    base_asset_amount_long: curve.1.base_asset_amount_long,
                    base_asset_amount_short: curve.1.base_asset_amount_short,
                    base_asset_amount: curve.1.base_asset_amount,
                    open_interest: curve.1.open_interest,
                    total_fee: curve.1.total_fee,
                    total_fee_minus_distributions: curve.1.total_fee_minus_distributions,
                    adjustment_cost: curve.1.adjustment_cost,
                    oracle_price: curve.1.oracle_price,
                })
            })
            .take(limit)
            .collect();
    }
    Ok(curves)
}

pub fn get_deposit_history(
    deps: Deps,
    user_address: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<DepositHistoryResponse>, ContractError> {
    let user_addr = addr_validate_to_lower(deps.api, &user_address.to_string())?;
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after
        .map(|start| start.joined_key())
        .map(Bound::Exclusive);
    let deposit_history = DEPOSIT_HISTORY
        .prefix(&user_addr)
        .range(deps.storage, start, None, Order::Descending)
        .filter_map(|records| {
            records.ok().map(|record| DepositHistoryResponse {
                ts: record.1.ts,
                user: record.1.user.to_string(),
                direction: record.1.direction,
                collateral_before: record.1.collateral_before,
                cumulative_deposits_before: record.1.cumulative_deposits_before,
                amount: record.1.amount,
            })
        })
        .take(limit)
        .collect();
    Ok(deposit_history)
}

pub fn get_funding_payment_history(
    deps: Deps,
    user_address: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<FundingPaymentHistoryResponse>, ContractError> {
    let user_addr = addr_validate_to_lower(deps.api, user_address.as_str())?;
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after
        .map(|start| start.joined_key())
        .map(Bound::Exclusive);
    let funding_payment_history = FUNDING_PAYMENT_HISTORY
        .prefix(&user_addr)
        .range(deps.storage, start, None, Order::Descending)
        .filter_map(|funding_payments| {
            funding_payments
                .ok()
                .map(|fp| FundingPaymentHistoryResponse {
                    ts: fp.1.ts,
                    user: fp.1.user.to_string(),
                    market_index: fp.1.market_index,
                    funding_payment: fp.1.funding_payment,
                    base_asset_amount: fp.1.base_asset_amount,
                    user_last_cumulative_funding: fp.1.user_last_cumulative_funding,
                    user_last_funding_rate_ts: fp.1.user_last_funding_rate_ts,
                    amm_cumulative_funding_long: fp.1.amm_cumulative_funding_long,
                    amm_cumulative_funding_short: fp.1.amm_cumulative_funding_short,
                })
        })
        .take(limit)
        .collect();
    Ok(funding_payment_history)
}

pub fn get_funding_rate_history(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<FundingRateHistoryResponse>, ContractError> {
    let mut fr_history: Vec<FundingRateHistoryResponse> = vec![];
    let length = LENGTH.load(deps.storage)?;
    if length.funding_rate_history_length > 0 {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
        let start = start_after
            .map(|start| start.joined_key())
            .map(Bound::Exclusive);
        fr_history = FUNDING_RATE_HISTORY
            .range(deps.storage, start, None, Order::Descending)
            .filter_map(|fr_records| {
                fr_records
                    .ok()
                    .map(|funding_record| FundingRateHistoryResponse {
                        ts: funding_record.1.ts,
                        market_index: funding_record.1.market_index,
                        funding_rate: funding_record.1.funding_rate,
                        cumulative_funding_rate_long: funding_record.1.cumulative_funding_rate_long,
                        cumulative_funding_rate_short: funding_record
                            .1
                            .cumulative_funding_rate_short,
                        oracle_price_twap: funding_record.1.oracle_price_twap,
                        mark_price_twap: funding_record.1.mark_price_twap,
                    })
            })
            .take(limit)
            .collect();
    }
    Ok(fr_history)
}

pub fn get_liquidation_history(
    deps: Deps,
    user_address: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<LiquidationHistoryResponse>, ContractError> {
    let user_addr = addr_validate_to_lower(deps.api, &user_address)?;

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after
        .map(|start| start.joined_key())
        .map(Bound::Exclusive);
    let liq_history = LIQUIDATION_HISTORY
        .prefix(&user_addr)
        .range(deps.storage, start, None, Order::Descending)
        .filter_map(|records| {
            records.ok().map(|record| LiquidationHistoryResponse {
                ts: record.1.ts,
                user: record.1.user.to_string(),
                partial: record.1.partial,
                base_asset_value: record.1.base_asset_value,
                base_asset_value_closed: record.1.base_asset_value_closed,
                liquidation_fee: record.1.liquidation_fee,
                fee_to_liquidator: record.1.fee_to_liquidator,
                fee_to_insurance_fund: record.1.fee_to_insurance_fund,
                liquidator: record.1.liquidator.to_string(),
                total_collateral: record.1.total_collateral,
                collateral: record.1.collateral,
                unrealized_pnl: record.1.unrealized_pnl,
                margin_ratio: record.1.margin_ratio,
            })
        })
        .take(limit)
        .collect();
    Ok(liq_history)
}

pub fn get_trade_history(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<TradeHistoryResponse>, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after
        .map(|start| start.joined_key())
        .map(Bound::Exclusive);
    let trade_history = TRADE_HISTORY
        .range(deps.storage, start, None, Order::Descending)
        .filter_map(|records| {
            records.ok().map(|record| TradeHistoryResponse {
                ts: record.1.ts,
                user: record.1.user.to_string(),
                direction: record.1.direction,
                base_asset_amount: record.1.base_asset_amount,
                quote_asset_amount: record.1.quote_asset_amount,
                mark_price_before: record.1.mark_price_before,
                mark_price_after: record.1.mark_price_after,
                fee: record.1.fee,
                referrer_reward: record.1.referrer_reward,
                referee_discount: record.1.referee_discount,
                token_discount: record.1.token_discount,
                liquidation: record.1.liquidation,
                market_index: record.1.market_index,
                oracle_price: record.1.oracle_price,
            })
        })
        .take(limit)
        .collect();
    Ok(trade_history)
}

pub fn get_trade_history_by_user(
    deps: Deps,
    user_address: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<TradeHistoryResponse>, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after
        .map(|start| start.joined_key())
        .map(Bound::Exclusive);
    let user_addr = addr_validate_to_lower(deps.api, &user_address)?;
    let trade_history = TRADE_HISTORY
        .prefix(&user_addr)
        .range(deps.storage, start, None, Order::Descending)
        .filter_map(|records| {
            records.ok().map(|record| TradeHistoryResponse {
                ts: record.1.ts,
                user: record.1.user.to_string(),
                direction: record.1.direction,
                base_asset_amount: record.1.base_asset_amount,
                quote_asset_amount: record.1.quote_asset_amount,
                mark_price_before: record.1.mark_price_before,
                mark_price_after: record.1.mark_price_after,
                fee: record.1.fee,
                referrer_reward: record.1.referrer_reward,
                referee_discount: record.1.referee_discount,
                token_discount: record.1.token_discount,
                liquidation: record.1.liquidation,
                market_index: record.1.market_index,
                oracle_price: record.1.oracle_price,
            })
        })
        .take(limit)
        .collect();
    Ok(trade_history)
}
