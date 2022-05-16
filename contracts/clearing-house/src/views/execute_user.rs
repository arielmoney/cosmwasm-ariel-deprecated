use std::ops::Div;

use crate::controller;
use crate::helpers;
use crate::helpers::position::calculate_withdrawal_amounts;
use crate::package::history::HistoryExecuteMsg;
use crate::states::constants::*;
use crate::states::history::*;
use crate::ContractError;

use crate::states::market::LiquidationStatus;
use crate::states::market::LiquidationType;
use crate::states::market::{Market, MARKETS};
use crate::states::state::FEESTRUCTURE;
use crate::states::state::ORACLEGUARDRAILS;
use crate::states::state::STATE;
use crate::states::user::{User, POSITIONS, USERS};

use crate::package::helper::addr_validate_to_lower;
use crate::package::helper::assert_sent_uusd_balance;
use crate::package::helper::query_balance;
use crate::package::helper::VaultInterface;
use crate::package::number::Number128;
use crate::package::types::{DepositDirection, PositionDirection};
use cosmwasm_std::{
    coins, to_binary, CosmosMsg, DepsMut, Env, Fraction, MessageInfo, Response, Uint128, WasmMsg,
};

pub fn try_deposit_collateral(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: u64,
    referrer: Option<String>,
) -> Result<Response, ContractError> {
    let user_address = info.sender.clone();
    let existing_user = USERS.may_load(deps.storage, &user_address)?;
    let now = env.block.time.seconds();
    let mut user: User;
    if existing_user.is_some() {
        // user = existing_user.unwrap();
        user = existing_user.unwrap();
    } else {
        if referrer.is_some() {
            user = User {
                collateral: Uint128::zero(),
                cumulative_deposits: Uint128::zero(),
                total_fee_paid: Uint128::zero(),
                total_token_discount: Uint128::zero(),
                total_referral_reward: Uint128::zero(),
                total_referee_discount: Uint128::zero(),
                referrer: Some(addr_validate_to_lower(deps.api, &referrer.unwrap())?),
            };
        } else {
            user = User {
                collateral: Uint128::zero(),
                cumulative_deposits: Uint128::zero(),
                total_fee_paid: Uint128::zero(),
                total_token_discount: Uint128::zero(),
                total_referral_reward: Uint128::zero(),
                total_referee_discount: Uint128::zero(),
                referrer: None,
            };
        }
    }

    if amount == 0 {
        return Err(ContractError::InsufficientDeposit.into());
    }

    assert_sent_uusd_balance(&info.clone(), amount as u128)?;
    let state = STATE.load(deps.storage)?;

    let collateral_before = user.collateral;
    let cumulative_deposits_before = user.cumulative_deposits;
    user.collateral = user.collateral.checked_add(Uint128::from(amount as u128))?;
    user.cumulative_deposits = user.cumulative_deposits.checked_add(amount.into())?;
    if state.max_deposit.u128() > 0 && user.cumulative_deposits.u128() > state.max_deposit.u128() {
        return Err(ContractError::UserMaxDeposit.into());
    }
    USERS.update(
        deps.storage,
        &user_address.clone(),
        |_m| -> Result<User, ContractError> { Ok(user) },
    )?;

    let f = controller::funding::settle_funding_payment(&mut deps, &user_address, now)?;
    let mut messages: Vec<CosmosMsg> = vec![];
    let fm: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: state.history_contract.clone().to_string(),
        msg: to_binary(&HistoryExecuteMsg::RecordFundingPaymentsMultiple { vecf: f })?,
        funds: vec![],
    });
    messages.push(fm);
    let message: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: state.collateral_vault.to_string(),
        msg: to_binary(&VaultInterface::Deposit {})?,
        funds: coins(amount.into(), "uusd"),
    });
    messages.push(message);

    let message_h = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: state.history_contract.clone().to_string(),
        msg: to_binary(&HistoryExecuteMsg::RecordDeposit {
            d: DepositRecord {
                ts: now,
                user: user_address.clone(),
                direction: DepositDirection::DEPOSIT,
                collateral_before,
                cumulative_deposits_before,
                amount: amount,
            },
        })?,
        funds: vec![],
    });
    messages.push(message_h);
    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("method", "try_deposit_collateral"))
}

pub fn try_withdraw_collateral(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: u64,
) -> Result<Response, ContractError> {
    let user_address = info.sender.clone();
    let existing_user = USERS.may_load(deps.storage, &user_address)?;
    let now = env.block.time.seconds();
    let mut user;
    if existing_user.is_none() {
        return Err(ContractError::UserDoesNotExist);
    } else {
        user = existing_user.unwrap();
    }
    let collateral_before = user.collateral;
    let cumulative_deposits_before = user.cumulative_deposits;
    let state = STATE.load(deps.storage)?;
    let f = controller::funding::settle_funding_payment(&mut deps, &user_address, now)?;
    let mut messages: Vec<CosmosMsg> = vec![];
    let message: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: state.history_contract.clone().to_string(),
        msg: to_binary(&HistoryExecuteMsg::RecordFundingPaymentsMultiple { vecf: f })?,
        funds: vec![],
    });
    messages.push(message);
    user = USERS.may_load(deps.storage, &user_address)?.unwrap();

    if (amount as u128) > user.collateral.u128() {
        return Err(ContractError::InsufficientCollateral.into());
    }

    let collateral_balance = query_balance(&deps.querier, state.collateral_vault.clone())?;
    let insurance_balance = query_balance(&deps.querier, state.insurance_vault.clone())?;
    let (collateral_account_withdrawal, insurance_account_withdrawal) =
        calculate_withdrawal_amounts(
            Uint128::from(amount as u128),
            Uint128::from(collateral_balance),
            Uint128::from(insurance_balance),
        )?;

    // amount_withdrawn can be less than amount if there is an insufficient balance in collateral and insurance vault
    let amount_withdraw =
        collateral_account_withdrawal.checked_add(insurance_account_withdrawal)?;

    user.cumulative_deposits = user
        .cumulative_deposits
        .checked_sub(Uint128::from(amount_withdraw))?;

    user.collateral = user
        .collateral
        .checked_sub(Uint128::from(collateral_account_withdrawal))?
        .checked_sub(Uint128::from(insurance_account_withdrawal))?;

    if !controller::margin::meets_initial_margin_requirement(&mut deps, &info.sender.clone())? {
        return Err(ContractError::InsufficientCollateral.into());
    }

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: state.collateral_vault.clone().to_string(),
        msg: to_binary(&VaultInterface::Withdraw {
            to_address: info.sender.clone(),
            amount: collateral_account_withdrawal,
        })?,
        funds: vec![],
    }));

    if insurance_account_withdrawal.gt(&Uint128::zero()) {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: state.insurance_vault.to_string(),
            msg: to_binary(&VaultInterface::Withdraw {
                to_address: info.sender.clone(),
                amount: insurance_account_withdrawal,
            })?,
            funds: vec![],
        }));
    }

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: state.history_contract.clone().to_string(),
        msg: to_binary(&HistoryExecuteMsg::RecordDeposit {
            d: DepositRecord {
                ts: now,
                user: user_address.clone(),
                direction: DepositDirection::WITHDRAW,
                collateral_before,
                cumulative_deposits_before,
                amount: amount_withdraw.u128() as u64,
            },
        })?,
        funds: vec![],
    }));
    USERS.update(
        deps.storage,
        &user_address.clone(),
        |_u| -> Result<User, ContractError> { Ok(user) },
    )?;
    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("method", "try_withdraw_collateral"))
}

pub fn try_open_position(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    is_direction_long: bool,
    quote_asset_amount: Uint128,
    market_index: u64,
    limit_price: Option<Uint128>,
) -> Result<Response, ContractError> {
    let user_address = info.sender.clone();

    let now = env.block.time.seconds();
    let state = STATE.load(deps.storage)?;
    let oracle_guard_rails = ORACLEGUARDRAILS.load(deps.storage)?;
    let fee_structure = FEESTRUCTURE.load(deps.storage)?;

    let direction = match is_direction_long {
        true => PositionDirection::Long,
        false => PositionDirection::Short,
    };

    if quote_asset_amount.is_zero() {
        return Err(ContractError::TradeSizeTooSmall.into());
    }
    let f = controller::funding::settle_funding_payment(&mut deps, &user_address, now)?;
    let mut messages: Vec<CosmosMsg> = vec![];
    let message: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: state.history_contract.clone().to_string(),
        msg: to_binary(&HistoryExecuteMsg::RecordFundingPaymentsMultiple { vecf: f })?,
        funds: vec![],
    });
    messages.push(message);
    let position_index = market_index.clone();
    let mark_price_before: Uint128;
    let oracle_mark_spread_pct_before: i128;
    let is_oracle_valid: bool;

    {
        let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
        mark_price_before = market.amm.mark_price()?;
        let oracle_price_data = market.amm.get_oracle_price(&mut deps, market_index)?;
        market = MARKETS.load(deps.storage, market_index.to_string())?;
        oracle_mark_spread_pct_before = helpers::amm::calculate_oracle_mark_spread_pct(
            &market.amm,
            &oracle_price_data,
            Some(mark_price_before),
        )?;
        is_oracle_valid =
            helpers::amm::is_oracle_valid(&market.amm, &oracle_price_data, &oracle_guard_rails)?;
        if is_oracle_valid {
            let normalised_oracle_price = helpers::amm::normalise_oracle_price(
                &market.amm,
                &oracle_price_data,
                Some(mark_price_before),
            )?;
            controller::amm::update_oracle_price_twap(
                &mut deps,
                market_index,
                now,
                normalised_oracle_price,
            )?;
        }
    }

    let potentially_risk_increasing;
    let base_asset_amount;
    let mut quote_asset_amount = quote_asset_amount;

    {
        let (_potentially_risk_increasing, _, _base_asset_amount, _quote_asset_amount, _) =
            controller::position::update_position_with_quote_asset_amount(
                &mut deps,
                quote_asset_amount,
                direction,
                &user_address,
                position_index,
                mark_price_before,
                now,
            )?;

        potentially_risk_increasing = _potentially_risk_increasing;
        base_asset_amount = _base_asset_amount;
        quote_asset_amount = _quote_asset_amount;
    }
    let mut user = USERS.load(deps.storage, &user_address)?;
    let mark_price_after: Uint128;
    let oracle_price_after: i128;
    let oracle_mark_spread_pct_after: i128;
    {
        let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
        mark_price_after = market.amm.mark_price()?;
        let oracle_price_data = market.amm.get_oracle_price(&mut deps, market_index)?;
        market = MARKETS.load(deps.storage, market_index.to_string())?;
        oracle_mark_spread_pct_after = helpers::amm::calculate_oracle_mark_spread_pct(
            &market.amm,
            &oracle_price_data,
            Some(mark_price_after),
        )?;
        oracle_price_after = oracle_price_data.price.i128();
    }

    let meets_initial_margin_requirement =
        controller::margin::meets_initial_margin_requirement(&mut deps, &user_address)?;
    if !meets_initial_margin_requirement && potentially_risk_increasing {
        return Err(ContractError::InsufficientCollateral.into());
    }

    // todo add referrer and discount token
    let referrer = user.referrer.clone();
    let discount_token = Uint128::zero();
    let (user_fee, fee_to_market, token_discount, referrer_reward, referee_discount) =
        helpers::fees::calculate_fee_for_trade(
            quote_asset_amount,
            &fee_structure,
            discount_token,
            &referrer,
        )?;

    {
        let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
        market.amm.total_fee = market.amm.total_fee.checked_add(fee_to_market)?;
        market.amm.total_fee_minus_distributions = market
            .amm
            .total_fee_minus_distributions
            .checked_add(fee_to_market)?;
        MARKETS.update(
            deps.storage,
            market_index.to_string(),
            |_m| -> Result<Market, ContractError> { Ok(market) },
        )?;
    }

    if user.collateral.ge(&user_fee) {
        user.collateral = user.collateral.checked_sub(user_fee)?;
    } else {
        user.collateral = Uint128::zero();
    }

    // Increment the user's total fee variables
    user.total_fee_paid = user.total_fee_paid.checked_add(user_fee)?;
    user.total_token_discount = user.total_token_discount.checked_add(token_discount)?;
    user.total_referee_discount = user.total_referee_discount.checked_add(referee_discount)?;

    // Update the referrer's collateral with their reward
    if referrer.is_some() {
        let mut _referrer = USERS.load(deps.storage, &referrer.clone().unwrap())?;
        _referrer.total_referral_reward = _referrer
            .total_referral_reward
            .checked_add(referrer_reward)?;
        // todo what this signifies
        // referrer.exit(ctx.program_id)?;
        USERS.update(
            deps.storage,
            &referrer.unwrap().clone(),
            |_m| -> Result<User, ContractError> { Ok(_referrer) },
        )?;
    }

    let is_oracle_mark_too_divergent_before = helpers::amm::is_oracle_mark_too_divergent(
        oracle_mark_spread_pct_before,
        &oracle_guard_rails,
    )?;
    let is_oracle_mark_too_divergent_after = helpers::amm::is_oracle_mark_too_divergent(
        oracle_mark_spread_pct_after,
        &oracle_guard_rails,
    )?;

    if is_oracle_mark_too_divergent_after && !is_oracle_mark_too_divergent_before && is_oracle_valid
    {
        return Err(ContractError::OracleMarkSpreadLimit.into());
    }

    let message: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: state.history_contract.to_string(),
        msg: to_binary(&HistoryExecuteMsg::RecordTrade {
            t: TradeRecord {
                ts: now,
                user: user_address.clone(),
                direction,
                base_asset_amount,
                quote_asset_amount,
                mark_price_before,
                mark_price_after,
                fee: user_fee,
                referrer_reward,
                referee_discount,
                token_discount,
                liquidation: false,
                market_index,
                oracle_price: Number128::new(oracle_price_after),
            },
        })?,
        funds: vec![],
    });
    messages.push(message);
    if limit_price.is_some()
        && !helpers::order::limit_price_satisfied(
            limit_price.unwrap(),
            quote_asset_amount,
            base_asset_amount,
            direction,
        )?
    {
        return Err(ContractError::SlippageOutsideLimit.into());
    }

    {
        let f = controller::funding::update_funding_rate(
            &mut deps,
            market_index,
            now,
            state.funding_paused,
            Some(mark_price_before),
        )?;
        match f {
            Some(fr) => {
                let message: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: state.history_contract.clone().to_string(),
                    msg: to_binary(&HistoryExecuteMsg::RecordFundingRate { f: fr })?,
                    funds: vec![],
                });
                messages.push(message);
            }
            None => {}
        }
    }

    USERS.update(
        deps.storage,
        &user_address.clone(),
        |_m| -> Result<User, ContractError> { Ok(user) },
    )?;

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("method", "try_open_position"))
}

pub fn try_close_position(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    market_index: u64,
) -> Result<Response, ContractError> {
    let user_address = info.sender.clone();
    let now = env.block.time.seconds();
    let state = STATE.load(deps.storage)?;
    let oracle_guard_rails = ORACLEGUARDRAILS.load(deps.storage)?;
    let fee_structure = FEESTRUCTURE.load(deps.storage)?;
    let f = controller::funding::settle_funding_payment(&mut deps, &user_address, now)?;
    let mut messages: Vec<CosmosMsg> = vec![];
    let message: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: state.history_contract.clone().to_string(),
        msg: to_binary(&HistoryExecuteMsg::RecordFundingPaymentsMultiple { vecf: f })?,
        funds: vec![],
    });
    messages.push(message);
    let position_index = market_index.clone();
    let market_position = POSITIONS.load(
        deps.storage,
        (&user_address.clone(), market_index.to_string()),
    )?;
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    let mark_price_before = market.amm.mark_price()?;
    let oracle_price_data = market.amm.get_oracle_price(&mut deps, market_index)?;
    market = MARKETS.load(deps.storage, market_index.to_string())?;
    let oracle_mark_spread_pct_before = helpers::amm::calculate_oracle_mark_spread_pct(
        &market.amm,
        &oracle_price_data,
        Some(mark_price_before),
    )?;
    let direction_to_close =
        helpers::position::direction_to_close_position(market_position.base_asset_amount.i128());

    let (quote_asset_amount, base_asset_amount, _) = controller::position::close(
        &mut deps,
        &user_address,
        market_index,
        position_index,
        now,
        None,
        Some(mark_price_before),
    )?;

    let mut user = USERS.load(deps.storage, &user_address)?;

    market = MARKETS.load(deps.storage, market_index.to_string())?;
    let base_asset_amount = Uint128::from(base_asset_amount.unsigned_abs());
    let referrer = user.referrer.clone();
    let discount_token = Uint128::zero();

    let (user_fee, fee_to_market, token_discount, referrer_reward, referee_discount) =
        helpers::fees::calculate_fee_for_trade(
            quote_asset_amount,
            &fee_structure,
            discount_token,
            &referrer,
        )?;

    market.amm.total_fee = market.amm.total_fee.checked_add(fee_to_market)?;
    market.amm.total_fee_minus_distributions = market
        .amm
        .total_fee_minus_distributions
        .checked_add(fee_to_market)?;

    if user.collateral.gt(&user_fee) {
        user.collateral = user.collateral.checked_sub(user_fee)?;
    } else {
        user.collateral = Uint128::zero();
    }

    user.total_fee_paid = user.total_fee_paid.checked_add(user_fee)?;
    user.total_token_discount = user.total_token_discount.checked_add(token_discount)?;
    user.total_referee_discount = user.total_referee_discount.checked_add(referee_discount)?;

    if referrer.is_some() {
        let mut _referrer = USERS.load(deps.storage, &referrer.clone().unwrap())?;
        _referrer.total_referral_reward = _referrer
            .total_referral_reward
            .checked_add(referrer_reward)?;
        USERS.update(
            deps.storage,
            &referrer.unwrap().clone(),
            |_m| -> Result<User, ContractError> { Ok(_referrer) },
        )?;
    }

    let mark_price_after = market.amm.mark_price()?;

    let oracle_mark_spread_pct_after = helpers::amm::calculate_oracle_mark_spread_pct(
        &market.amm,
        &oracle_price_data,
        Some(mark_price_after),
    )?;

    let oracle_price_after = oracle_price_data.price;

    let is_oracle_valid =
        helpers::amm::is_oracle_valid(&market.amm, &oracle_price_data, &oracle_guard_rails)?;

    MARKETS.update(
        deps.storage,
        market_index.to_string(),
        |_m| -> Result<Market, ContractError> { Ok(market.clone()) },
    )?;

    USERS.update(
        deps.storage,
        &user_address.clone(),
        |_m| -> Result<User, ContractError> { Ok(user) },
    )?;

    if is_oracle_valid {
        let normalised_oracle_price = helpers::amm::normalise_oracle_price(
            &market.amm,
            &oracle_price_data,
            Some(mark_price_before),
        )?;
        controller::amm::update_oracle_price_twap(
            &mut deps,
            market_index,
            now,
            normalised_oracle_price,
        )?;
    }

    let is_oracle_mark_too_divergent_before = helpers::amm::is_oracle_mark_too_divergent(
        oracle_mark_spread_pct_before,
        &oracle_guard_rails,
    )?;
    let is_oracle_mark_too_divergent_after = helpers::amm::is_oracle_mark_too_divergent(
        oracle_mark_spread_pct_after,
        &oracle_guard_rails,
    )?;

    if (is_oracle_mark_too_divergent_after && !is_oracle_mark_too_divergent_before)
        && is_oracle_valid
    {
        return Err(ContractError::OracleMarkSpreadLimit.into());
    }
    let mut messages: Vec<CosmosMsg> = vec![];
    let t = TradeRecord {
        ts: now,
        user: user_address.clone(),
        direction: direction_to_close,
        base_asset_amount,
        quote_asset_amount,
        mark_price_before,
        mark_price_after,
        fee: user_fee,
        referrer_reward,
        referee_discount,
        token_discount,
        liquidation: false,
        market_index,
        oracle_price: oracle_price_after,
    };
    let message: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: state.history_contract.clone().to_string(),
        msg: to_binary(&HistoryExecuteMsg::RecordTrade { t })?,
        funds: vec![],
    });
    messages.push(message);
    let f = controller::funding::update_funding_rate(
        &mut deps,
        market_index,
        now,
        state.funding_paused,
        Some(mark_price_before),
    )?;
    match f {
        Some(fr) => {
            let message: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: state.history_contract.clone().to_string(),
                msg: to_binary(&HistoryExecuteMsg::RecordFundingRate { f: fr })?,
                funds: vec![],
            });
            messages.push(message);
        }
        None => {},
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("method", "try_close_position"))
}

//new limit order interfaces
// pub fn try_place_order(
//     mut deps: DepsMut,
//     env: Env,
//     info: MessageInfo,
//     order: OrderParams,
// ) -> Result<Response, ContractError> {
//     let now = env.block.time.seconds();
//     let user_address = info.sender.clone();
//     let state = STATE.load(deps.storage)?;
//     let oracle = state.oracle;
//     if order.order_type == OrderType::Market {
//         return Err(ContractError::MarketOrderMustBeInPlaceAndFill.into());
//     }

//     controller::order::place_order(&mut deps, &user_address, now, order, &oracle)?;
//     Ok(Response::new().add_attribute("method", "try_place_order"))
// }

// pub fn try_cancel_order(
//     mut deps: DepsMut,
//     env: Env,
//     info: MessageInfo,
//     market_index: u64,
//     order_id: u64,
// ) -> Result<Response, ContractError> {
//     let now = env.block.time.seconds();
//     let state = STATE.load(deps.storage)?;
//     let oracle = state.oracle;
//     controller::order::cancel_order(
//         &mut deps,
//         &info.sender.clone(),
//         market_index,
//         order_id,
//         &oracle,
//         now,
//     )?;
//     Ok(Response::new().add_attribute("method", "try_cancel_order"))
// }

//todo who is filler? is sender is filler and passing the user address?
// pub fn try_expire_orders(
//     mut deps: DepsMut,
//     env: Env,
//     info: MessageInfo,
//     user_address: String,
// ) -> Result<Response, ContractError> {
//     let now = env.block.time.seconds();
//     let user_address = addr_validate_to_lower(deps.api, &user_address.to_string())?;
//     controller::order::expire_orders(&mut deps, &user_address, now, &info.sender.clone())?;
//     Ok(Response::new().add_attribute("method", "try_expire_orders"))
// }

// pub fn try_fill_order(
//     mut deps: DepsMut,
//     env: Env,
//     info: MessageInfo,
//     order_id: u64,
//     user_address: String,
//     market_index: u64,
// ) -> Result<Response, ContractError> {
//     let now = env.block.time.seconds();
//     let user_address = addr_validate_to_lower(deps.api, &user_address.to_string())?;
//     let base_asset_amount = controller::order::fill_order(
//         &mut deps,
//         &user_address,
//         &info.sender.clone(),
//         market_index,
//         order_id,
//         now,
//     )?;
//     if base_asset_amount.is_zero() {
//         return Err(ContractError::CouldNotFillOrder);
//     }
//     Ok(Response::new().add_attribute("method", "try_fill_order"))
// }

//todo later

pub fn try_liquidate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user: String,
    market_index: u64,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    let user_address = addr_validate_to_lower(deps.api, &user)?;
    let now = env.block.time.seconds();
    let mut messages: Vec<CosmosMsg> = vec![];
    let f = controller::funding::settle_funding_payment(&mut deps, &user_address, now)?;

    let mut user = USERS.load(deps.storage, &user_address)?;

    let LiquidationStatus {
        liquidation_type,
        total_collateral,
        adjusted_total_collateral,
        unrealized_pnl,
        base_asset_value,
        market_statuses,
        mut margin_requirement,
        margin_ratio,
    } = controller::margin::calculate_liquidation_status(&mut deps, &user_address)?;

    let res: Response = Response::new().add_attribute("method", "try_liquidate");
    let collateral = user.collateral;
    if liquidation_type == LiquidationType::NONE {
        res.clone()
            .add_attribute("total_collateral {}", total_collateral.to_string());
        res.clone().add_attribute(
            "adjusted_total_collateral {}",
            adjusted_total_collateral.to_string(),
        );
        res.clone()
            .add_attribute("margin_requirement {}", margin_requirement.to_string());
        return Err(ContractError::SufficientCollateral.into());
    }

    let is_dust_position = adjusted_total_collateral <= QUOTE_PRECISION;

    let mut base_asset_value_closed: Uint128 = Uint128::zero();
    let mut liquidation_fee = Uint128::zero();

    let is_full_liquidation = liquidation_type == LiquidationType::FULL || is_dust_position;

    if is_full_liquidation {
        let maximum_liquidation_fee = total_collateral
            .checked_mul(Uint128::from(
                state.full_liquidation_penalty_percentage.numerator(),
            ))?
            .checked_div(Uint128::from(
                state.full_liquidation_penalty_percentage.denominator(),
            ))?;

        for market_status in market_statuses.iter() {
            if market_status.base_asset_value.is_zero() {
                continue;
            }

            let market = MARKETS.load(deps.storage, market_status.market_index.to_string())?;
            let mark_price_before = market_status.mark_price_before;
            let oracle_status = &market_status.oracle_status;

            // if the oracle is invalid and the mark moves too far from twap, dont liquidate
            let oracle_is_valid = oracle_status.is_valid;
            if !oracle_is_valid {
                let mark_twap_divergence =
                    helpers::amm::calculate_mark_twap_spread_pct(&market.amm, mark_price_before)?;
                let mark_twap_too_divergent =
                    mark_twap_divergence.unsigned_abs() >= MAX_MARK_TWAP_DIVERGENCE.u128();

                if mark_twap_too_divergent {
                    res.clone().add_attribute(
                        "mark_twap_divergence {} for market {}",
                        mark_twap_divergence.to_string(),
                    );
                    continue;
                }
            }

            let market_position =
                POSITIONS.load(deps.storage, (&user_address, market_index.to_string()))?;
            // todo initialize position

            let mark_price_before_i128 = mark_price_before.u128() as i128;
            let close_position_slippage = match market_status.close_position_slippage {
                Some(close_position_slippage) => close_position_slippage,
                None => helpers::position::calculate_slippage(
                    market_status.base_asset_value,
                    Uint128::from(market_position.base_asset_amount.i128().unsigned_abs()),
                    mark_price_before_i128,
                )?,
            };
            let close_position_slippage_pct = helpers::position::calculate_slippage_pct(
                close_position_slippage,
                mark_price_before_i128,
            )?;

            let close_slippage_pct_too_large = close_position_slippage_pct
                > MAX_LIQUIDATION_SLIPPAGE.u128() as i128
                || close_position_slippage_pct < -(MAX_LIQUIDATION_SLIPPAGE.u128() as i128);

            let oracle_mark_divergence_after_close = if !close_slippage_pct_too_large {
                oracle_status
                    .oracle_mark_spread_pct
                    .i128()
                    .checked_add(close_position_slippage_pct)
                    .ok_or_else(|| (ContractError::MathError))?
            } else if close_position_slippage_pct > 0 {
                oracle_status
                    .oracle_mark_spread_pct
                    .i128()
                    // approximates price impact based on slippage
                    .checked_add((MAX_LIQUIDATION_SLIPPAGE.u128() as i128) * 2)
                    .ok_or_else(|| (ContractError::MathError))?
            } else {
                oracle_status
                    .oracle_mark_spread_pct
                    .i128()
                    // approximates price impact based on slippage
                    .checked_sub((MAX_LIQUIDATION_SLIPPAGE.u128() as i128) * 2)
                    .ok_or_else(|| (ContractError::MathError))?
            };

            let oracle_guard_rails = ORACLEGUARDRAILS.load(deps.storage)?;

            let oracle_mark_too_divergent_after_close = helpers::amm::is_oracle_mark_too_divergent(
                oracle_mark_divergence_after_close,
                &oracle_guard_rails,
            )?;

            // if closing pushes outside the oracle mark threshold, don't liquidate
            if oracle_is_valid && oracle_mark_too_divergent_after_close {
                // but only skip the liquidation if it makes the divergence worse
                if oracle_status.oracle_mark_spread_pct.i128().unsigned_abs()
                    < oracle_mark_divergence_after_close.unsigned_abs()
                {
                    res.clone().add_attribute(
                        "oracle_mark_divergence_after_close ",
                        oracle_mark_divergence_after_close.to_string(),
                    );
                    continue;
                }
            }

            let direction_to_close = helpers::position::direction_to_close_position(
                market_position.base_asset_amount.i128(),
            );

            // just reduce position if position is too big
            let (quote_asset_amount, base_asset_amount) = if close_slippage_pct_too_large {
                let quote_asset_amount = market_status
                    .base_asset_value
                    .checked_mul(MAX_LIQUIDATION_SLIPPAGE)?
                    .checked_div(Uint128::from(close_position_slippage_pct.unsigned_abs()))?;

                let base_asset_amount = controller::position::reduce(
                    &mut deps,
                    direction_to_close,
                    quote_asset_amount,
                    &user_address,
                    market_index,
                    market_index,
                    now,
                    Some(mark_price_before),
                )?;

                (quote_asset_amount, base_asset_amount)
            } else {
                let (quote_asset_amount, base_asset_amount, _) = controller::position::close(
                    &mut deps,
                    &user_address,
                    market_index,
                    market_index,
                    now,
                    None,
                    Some(mark_price_before),
                )?;

                (quote_asset_amount, base_asset_amount)
            };

            let base_asset_amount = Uint128::from(base_asset_amount.unsigned_abs());
            base_asset_value_closed = base_asset_value_closed.checked_add(quote_asset_amount)?;
            let mark_price_after = market.amm.mark_price()?;
            let message_h = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: state.history_contract.clone().to_string(),
                msg: to_binary(&HistoryExecuteMsg::RecordTrade {
                    t: TradeRecord {
                        ts: now,
                        user: user_address.clone(),
                        direction: direction_to_close,
                        base_asset_amount,
                        quote_asset_amount,
                        mark_price_before,
                        mark_price_after,
                        fee: Uint128::zero(),
                        referrer_reward: Uint128::zero(),
                        referee_discount: Uint128::zero(),
                        token_discount: Uint128::zero(),
                        liquidation: true,
                        market_index,
                        oracle_price: market_status.oracle_status.price_data.price,
                    },
                })?,
                funds: vec![],
            });
            messages.push(message_h);
            margin_requirement = margin_requirement.checked_sub(
                market_status
                    .maintenance_margin_requirement
                    .checked_mul(quote_asset_amount)?
                    .checked_div(market_status.base_asset_value)?,
            )?;

            let market_liquidation_fee = maximum_liquidation_fee
                .checked_mul(quote_asset_amount)?
                .checked_div(base_asset_value)?;

            liquidation_fee = liquidation_fee.checked_add(market_liquidation_fee)?;

            let adjusted_total_collateral_after_fee =
                adjusted_total_collateral.checked_sub(liquidation_fee)?;

            if !is_dust_position && margin_requirement < adjusted_total_collateral_after_fee {
                break;
            }
        }
    } else {
        let maximum_liquidation_fee = total_collateral
            .checked_mul(Uint128::from(
                state.partial_liquidation_penalty_percentage.numerator(),
            ))?
            .checked_div(Uint128::from(
                state.partial_liquidation_penalty_percentage.denominator(),
            ))?;
        let maximum_base_asset_value_closed = base_asset_value
            .checked_mul(Uint128::from(
                state.partial_liquidation_close_percentage.numerator(),
            ))?
            .checked_div(Uint128::from(
                state.partial_liquidation_close_percentage.denominator(),
            ))?;
        for market_status in market_statuses.iter() {
            if market_status.base_asset_value.is_zero() {
                continue;
            }

            let oracle_status = &market_status.oracle_status;
            let market = MARKETS.load(deps.storage, market_index.to_string())?;
            let mark_price_before = market_status.mark_price_before;

            let oracle_is_valid = oracle_status.is_valid;
            if !oracle_is_valid {
                let mark_twap_divergence =
                    helpers::amm::calculate_mark_twap_spread_pct(&market.amm, mark_price_before)?;
                let mark_twap_too_divergent =
                    mark_twap_divergence.unsigned_abs() >= MAX_MARK_TWAP_DIVERGENCE.u128();

                if mark_twap_too_divergent {
                    res.clone()
                        .add_attribute("mark_twap_divergence", mark_twap_divergence.to_string());
                    continue;
                }
            }

            let market_position =
                POSITIONS.load(deps.storage, (&user_address, market_index.to_string()))?;

            let mut quote_asset_amount = market_status
                .base_asset_value
                .checked_mul(Uint128::from(
                    state.partial_liquidation_close_percentage.numerator(),
                ))?
                .checked_div(Uint128::from(
                    state.partial_liquidation_close_percentage.denominator(),
                ))?;

            let mark_price_before_i128 = mark_price_before.u128() as i128;
            let reduce_position_slippage = match market_status.close_position_slippage {
                Some(close_position_slippage) => close_position_slippage.div(4),
                None => helpers::position::calculate_slippage(
                    market_status.base_asset_value,
                    Uint128::from(market_position.base_asset_amount.i128().unsigned_abs()),
                    mark_price_before_i128,
                )?
                .div(4),
            };

            let reduce_position_slippage_pct = helpers::position::calculate_slippage_pct(
                reduce_position_slippage,
                mark_price_before_i128,
            )?;

            res.clone().add_attribute(
                "reduce_position_slippage_pct",
                reduce_position_slippage_pct.to_string(),
            );

            let reduce_slippage_pct_too_large = reduce_position_slippage_pct
                > (MAX_LIQUIDATION_SLIPPAGE.u128() as i128)
                || reduce_position_slippage_pct < -(MAX_LIQUIDATION_SLIPPAGE.u128() as i128);

            let oracle_mark_divergence_after_reduce = if !reduce_slippage_pct_too_large {
                oracle_status
                    .oracle_mark_spread_pct
                    .i128()
                    .checked_add(reduce_position_slippage_pct)
                    .ok_or_else(|| (ContractError::MathError))?
            } else if reduce_position_slippage_pct > 0 {
                oracle_status
                    .oracle_mark_spread_pct
                    .i128()
                    // approximates price impact based on slippage
                    .checked_add((MAX_LIQUIDATION_SLIPPAGE.u128() as i128) * 2)
                    .ok_or_else(|| (ContractError::MathError))?
            } else {
                oracle_status
                    .oracle_mark_spread_pct
                    .i128()
                    // approximates price impact based on slippage
                    .checked_sub((MAX_LIQUIDATION_SLIPPAGE.u128() as i128) * 2)
                    .ok_or_else(|| (ContractError::MathError))?
            };

            let oracle_guard_rails = ORACLEGUARDRAILS.load(deps.storage)?;
            let oracle_mark_too_divergent_after_reduce =
                helpers::amm::is_oracle_mark_too_divergent(
                    oracle_mark_divergence_after_reduce,
                    &oracle_guard_rails,
                )?;

            // if reducing pushes outside the oracle mark threshold, don't liquidate
            if oracle_is_valid && oracle_mark_too_divergent_after_reduce {
                // but only skip the liquidation if it makes the divergence worse
                if oracle_status.oracle_mark_spread_pct.i128().unsigned_abs()
                    < oracle_mark_divergence_after_reduce.unsigned_abs()
                {
                    res.clone().add_attribute(
                        "oracle_mark_spread_pct_after_reduce",
                        oracle_mark_divergence_after_reduce.to_string(),
                    );
                    return Err(ContractError::OracleMarkSpreadLimit.into());
                }
            }

            if reduce_slippage_pct_too_large {
                quote_asset_amount = quote_asset_amount
                    .checked_mul(MAX_LIQUIDATION_SLIPPAGE)?
                    .checked_div(Uint128::from(reduce_position_slippage_pct.unsigned_abs()))?;
            }

            base_asset_value_closed = base_asset_value_closed.checked_add(quote_asset_amount)?;

            let direction_to_reduce = helpers::position::direction_to_close_position(
                market_position.base_asset_amount.i128(),
            );

            let base_asset_amount = controller::position::reduce(
                &mut deps,
                direction_to_reduce,
                quote_asset_amount,
                &user_address,
                market_index,
                market_index,
                now,
                Some(mark_price_before),
            )?
            .unsigned_abs();

            let mark_price_after = market.amm.mark_price()?;
            let message_h = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: state.history_contract.clone().to_string(),
                msg: to_binary(&HistoryExecuteMsg::RecordTrade {
                    t: TradeRecord {
                        ts: now,
                        user: user_address.clone(),
                        direction: direction_to_reduce,
                        base_asset_amount: Uint128::from(base_asset_amount),
                        quote_asset_amount,
                        mark_price_before,
                        mark_price_after,
                        fee: Uint128::zero(),
                        referrer_reward: Uint128::zero(),
                        referee_discount: Uint128::zero(),
                        token_discount: Uint128::zero(),
                        liquidation: true,
                        market_index,
                        oracle_price: market_status.oracle_status.price_data.price,
                    },
                })?,
                funds: vec![],
            });
            messages.push(message_h);

            margin_requirement = margin_requirement.checked_sub(
                market_status
                    .partial_margin_requirement
                    .checked_mul(quote_asset_amount)?
                    .checked_div(market_status.base_asset_value)?,
            )?;

            let market_liquidation_fee = maximum_liquidation_fee
                .checked_mul(quote_asset_amount)?
                .checked_div(maximum_base_asset_value_closed)?;

            liquidation_fee = liquidation_fee.checked_add(market_liquidation_fee)?;

            let adjusted_total_collateral_after_fee =
                adjusted_total_collateral.checked_sub(liquidation_fee)?;

            if margin_requirement < adjusted_total_collateral_after_fee {
                break;
            }
        }
    }
    if base_asset_value_closed.is_zero() {
        return Err(ContractError::NoPositionsLiquidatable);
    }

    let balance_collateral = query_balance(&deps.querier, state.collateral_vault.clone())?;

    let balance_insurance = query_balance(&deps.querier, state.insurance_vault.clone())?;

    let (withdrawal_amount, _) = calculate_withdrawal_amounts(
        liquidation_fee,
        Uint128::from(balance_collateral),
        Uint128::from(balance_insurance),
    )?;

    user = USERS.load(deps.storage, &user_address)?;
    user.collateral = user.collateral.checked_sub(liquidation_fee)?;
    USERS.update(
        deps.storage,
        &user_address,
        |_u| -> Result<User, ContractError> { Ok(user) },
    )?;

    let fee_to_liquidator = if is_full_liquidation {
        withdrawal_amount.checked_div(Uint128::from(
            state.full_liquidation_liquidator_share_denominator,
        ))?
    } else {
        withdrawal_amount.checked_div(Uint128::from(
            state.partial_liquidation_liquidator_share_denominator,
        ))?
    };

    let fee_to_insurance_fund = withdrawal_amount.checked_sub(fee_to_liquidator)?;

    if fee_to_liquidator.gt(&Uint128::zero()) {
        let mut liquidator = USERS.load(deps.storage, &info.sender.clone())?;
        liquidator.collateral = liquidator
            .collateral
            .checked_add(Uint128::from(fee_to_liquidator))?;

        USERS.update(
            deps.storage,
            &info.sender.clone(),
            |_m| -> Result<User, ContractError> { Ok(liquidator) },
        )?;
    }

    if fee_to_insurance_fund.gt(&Uint128::zero()) {
        let message = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: state.collateral_vault.to_string(),
            msg: to_binary(&VaultInterface::Withdraw {
                to_address: state.insurance_vault.clone(),
                amount: fee_to_insurance_fund,
            })?,
            funds: vec![],
        });
        messages.push(message);
    }

    let message: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: state.history_contract.clone().to_string(),
        msg: to_binary(&HistoryExecuteMsg::RecordFundingPaymentsMultiple { vecf: f })?,
        funds: vec![],
    });
    messages.push(message);

    LiquidationRecord {
        ts: now,
        user: user_address,
        partial: !is_full_liquidation,
        base_asset_value,
        base_asset_value_closed,
        liquidation_fee,
        liquidator: info.sender.clone(),
        total_collateral,
        collateral,
        unrealized_pnl: Number128::new(unrealized_pnl),
        margin_ratio,
        fee_to_liquidator: fee_to_liquidator.u128() as u64,
        fee_to_insurance_fund: fee_to_insurance_fund.u128() as u64,
    };
    Ok(res
        .add_messages(messages))
}

pub fn try_settle_funding_payment(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let now = env.block.time.seconds();
    let user_address = info.sender;

    let f = controller::funding::settle_funding_payment(&mut deps, &user_address, now)?;
    let state = STATE.load(deps.storage)?;
    let mut messages: Vec<CosmosMsg> = vec![];
    let message: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: state.history_contract.clone().to_string(),
        msg: to_binary(&HistoryExecuteMsg::RecordFundingPaymentsMultiple { vecf: f })?,
        funds: vec![],
    });
    messages.push(message);

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("method", "try_settle_funding_payment"))
}
