use std::cmp::max;

use crate::package::number::Number128;
use cosmwasm_std::Addr;
use cosmwasm_std::DepsMut;
use cosmwasm_std::Uint128;

use crate::error::ContractError;

use crate::helpers::amm::normalise_oracle_price;
use crate::states::history::{
    FundingPaymentRecord,
    FundingRateRecord,
};
use crate::states::market::{Market, MARKETS};
use crate::states::state::ORACLEGUARDRAILS;
use crate::states::state::STATE;
use crate::states::user::{Position, POSITIONS, User, USERS};

use crate::helpers::position::calculate_updated_collateral;
use crate::states::constants::{
    AMM_TO_QUOTE_PRECISION_RATIO_I128, FUNDING_PAYMENT_PRECISION, ONE_HOUR,
};
use crate::helpers::funding::{calculate_funding_payment, calculate_funding_rate_long_short};
use crate::helpers::oracle;

use crate::controller::amm;

/// Funding payments are settled lazily. The amm tracks its cumulative funding rate (for longs and shorts)
/// and the user's market position tracks how much funding the user been cumulatively paid for that market.
/// If the two values are not equal, the user owes/is owed funding.
pub fn settle_funding_payment(
    deps: &mut DepsMut,
    user_addr: &Addr,
    now: u64,
) -> Result<Vec<FundingPaymentRecord>, ContractError> {
    let mut fundingpay : Vec<FundingPaymentRecord> = Vec::new();
    let existing_user = USERS.may_load(deps.storage, &user_addr.clone())?;
    let mut funding_payment: i128 = 0;
    let mut user;
    if existing_user.is_some(){
        user = existing_user.unwrap();
    }
    else{
        return Ok(fundingpay);
    }
    let markets_length = STATE.load(deps.storage)?.markets_length;
    for n in 1..markets_length {
        let market_position = POSITIONS.load(deps.storage, (user_addr, n.to_string()));
        match market_position {
            Ok(mut m) => {
                if m.base_asset_amount.i128() == 0 {
                    continue;
                }
                let market = MARKETS.load(deps.storage, n.to_string())?;
                let amm_cumulative_funding_rate = if m.base_asset_amount.i128() > 0 {
                    market.amm.cumulative_funding_rate_long.i128()
                } else {
                    market.amm.cumulative_funding_rate_short.i128()
                };
                if amm_cumulative_funding_rate != m.last_cumulative_funding_rate.i128() {
                    let market_funding_rate_payment =
                        calculate_funding_payment(amm_cumulative_funding_rate, &m)?;

                    fundingpay.push(FundingPaymentRecord {
                            ts: now,
                            user: user_addr.clone(),
                            market_index: n,
                            funding_payment: Number128::new(market_funding_rate_payment), //10e13
                            user_last_cumulative_funding: m.last_cumulative_funding_rate, //10e14
                            user_last_funding_rate_ts: m.last_funding_rate_ts,
                            amm_cumulative_funding_long: market.amm.cumulative_funding_rate_long, //10e14
                            amm_cumulative_funding_short: market.amm.cumulative_funding_rate_short, //10e14
                            base_asset_amount: m.base_asset_amount,
                    });
                    funding_payment = funding_payment
                        .checked_add(market_funding_rate_payment)
                        .ok_or_else(|| (ContractError::MathError))?;
        
                    m.last_cumulative_funding_rate = Number128::new(amm_cumulative_funding_rate);
                    m.last_funding_rate_ts = market.amm.last_funding_rate_ts;
        
                    POSITIONS.update(
                        deps.storage,
                        (user_addr, n.to_string()),
                        |_p| -> Result<Position, ContractError> { Ok(m) },
                    )?;
                }
            }
            Err(_) => continue, 
        }
        
    }

    let funding_payment_collateral = funding_payment
        .checked_div(AMM_TO_QUOTE_PRECISION_RATIO_I128.u128() as i128)
        .ok_or_else(|| (ContractError::MathError))?;

    user.collateral = calculate_updated_collateral(user.collateral, funding_payment_collateral)?;

    USERS.update(
        deps.storage,
        user_addr,
        |_u| -> Result<User, ContractError> { Ok(user) },
    )?;

    Ok(fundingpay)
}

pub fn update_funding_rate(
    deps: &mut DepsMut,
    market_index: u64,
    now: u64,
    funding_paused: bool,
    precomputed_mark_price: Option<Uint128>,
) -> Result<Option<FundingRateRecord>, ContractError> {
    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
    let guard_rails = ORACLEGUARDRAILS.load(deps.storage)?;

    let time_since_last_update = now
        .checked_sub(market.amm.last_funding_rate_ts)
        .ok_or_else(|| (ContractError::MathError))?;

    // Pause funding if oracle is invalid or if mark/oracle spread is too divergent
    let (block_funding_rate_update, oracle_price_data) = oracle::block_operation(
        deps,
        &market.amm,
        &guard_rails,
        market_index,
        precomputed_mark_price,
    )?;

    let normalised_oracle_price =
        normalise_oracle_price(&market.amm, &oracle_price_data, precomputed_mark_price)?;

    // round next update time to be available on the hour
    let mut next_update_wait = market.amm.funding_period;
    if market.amm.funding_period > 1 {
        let last_update_delay = market
            .amm
            .last_funding_rate_ts
            .rem_euclid(market.amm.funding_period);
        if last_update_delay != 0 {
            let max_delay_for_next_period = market
                .amm
                .funding_period
                .checked_div(3)
                .ok_or_else(|| (ContractError::MathError1))?;
            if last_update_delay > max_delay_for_next_period {
                // too late for on the hour next period, delay to following period
                next_update_wait = market
                    .amm
                    .funding_period
                    .checked_mul(2)
                    .ok_or_else(|| (ContractError::MathError2))?
                    .checked_sub(last_update_delay)
                    .ok_or_else(|| (ContractError::MathError3))?;
            } else {
                // allow update on the hour
                next_update_wait = market
                    .amm
                    .funding_period
                    .checked_sub(last_update_delay)
                    .ok_or_else(|| (ContractError::MathError4))?;
            }
        }
    }

    if !funding_paused && !block_funding_rate_update && time_since_last_update >= next_update_wait {
        let oracle_price_twap =
            amm::update_oracle_price_twap(deps, market_index, now, normalised_oracle_price)?;
        let mark_price_twap = amm::update_mark_twap(deps, market_index, now, None)?;

        let one_hour_i64 = ONE_HOUR.u128() as i64;
        let period_adjustment = (24_i64)
            .checked_mul(one_hour_i64)
            .ok_or_else(|| (ContractError::MathError5))?
            .checked_div(max(one_hour_i64, market.amm.funding_period as i64))
            .ok_or_else(|| (ContractError::MathError6))?;

        // funding period = 1 hour, window = 1 day
        // low periodicity => quickly updating/settled funding rates => lower funding rate payment per interval
        let price_spread = (mark_price_twap.u128()  as i128)
            .checked_sub(oracle_price_twap).ok_or_else(|| (ContractError::MathError7))?;

        let funding_rate = price_spread
            .checked_mul(FUNDING_PAYMENT_PRECISION.u128() as i128)
            .ok_or_else(|| (ContractError::MathError8))?
            .checked_div(period_adjustment as i128)
            .ok_or_else(|| (ContractError::MathError9))?;

        let (funding_rate_long, funding_rate_short, new_total_fee_minus_distributions) =
            calculate_funding_rate_long_short(&market, funding_rate)?;

        market.amm.total_fee_minus_distributions = new_total_fee_minus_distributions;

        market.amm.cumulative_funding_rate_long = Number128::new(market
            .amm
            .cumulative_funding_rate_long.i128()
            .checked_add(funding_rate_long)
            .ok_or_else(|| (ContractError::MathError10))?);

        market.amm.cumulative_funding_rate_short = Number128::new(market
            .amm
            .cumulative_funding_rate_short.i128()
            .checked_add(funding_rate_short)
            .ok_or_else(|| (ContractError::MathError11))?);

        market.amm.last_funding_rate = Number128::new(funding_rate);
        market.amm.last_funding_rate_ts = now;

        MARKETS.update(
            deps.storage,
            market_index.to_string(),
            |_m| -> Result<Market, ContractError> { Ok(market.clone()) },
        )?;

        
        let f = FundingRateRecord {
                ts: now,
                market_index,
                funding_rate: Number128::new(funding_rate),
                cumulative_funding_rate_long: market.amm.cumulative_funding_rate_long,
                cumulative_funding_rate_short: market.amm.cumulative_funding_rate_short,
                mark_price_twap,
                oracle_price_twap: Number128::new(oracle_price_twap),
        };
        return Ok(Some(f));
    }
    Ok(None)
}
