use cosmwasm_std::{DepsMut, Uint128};

use crate::error::ContractError;

use crate::states::market::{MARKETS, Market};
use crate::states::state::ORACLEGUARDRAILS;

use crate::helpers::amm;
use crate::states::constants::{
    SHARE_OF_FEES_ALLOCATED_TO_CLEARING_HOUSE_NUMERATOR,SHARE_OF_FEES_ALLOCATED_TO_CLEARING_HOUSE_DENOMINATOR
};
use crate::helpers::position::_calculate_base_asset_value_and_pnl;

pub fn repeg(
    deps: &mut DepsMut,
    market_index: u64,
    new_peg_candidate: Uint128
) -> Result<i128, ContractError> {

    let mut market = MARKETS.load(deps.storage, market_index.to_string())?;

    let oracle_guard_rails = ORACLEGUARDRAILS.load(deps.storage)?;

    if new_peg_candidate == market.amm.peg_multiplier {
        return Err(ContractError::InvalidRepegRedundant.into());
    }

    let terminal_price_before = amm::calculate_terminal_price(&mut market)?;
    let adjustment_cost = adjust_peg_cost(&mut market, new_peg_candidate)?;

    
    market.amm.peg_multiplier = new_peg_candidate;

    let oracle_price_data = market.amm.get_oracle_price(deps, market_index)?;	
    let oracle_price = oracle_price_data.price.i128();	
    let oracle_conf = oracle_price_data.confidence;
    let oracle_is_valid =	
        amm::is_oracle_valid(&market.amm, &oracle_price_data, &oracle_guard_rails)?;	
    
    // if oracle is valid: check on size/direction of repeg
    if oracle_is_valid {
        let terminal_price_after = amm::calculate_terminal_price(&mut market)?;

        let mark_price_after = amm::calculate_price(
            market.amm.quote_asset_reserve,
            market.amm.base_asset_reserve,
            market.amm.peg_multiplier,
        )?;

        let oracle_conf_band_top = Uint128::from(oracle_price.unsigned_abs())
            .checked_add(oracle_conf)?;

        let oracle_conf_band_bottom = Uint128::from(oracle_price.unsigned_abs())
            .checked_sub(oracle_conf)?;

        if oracle_price.unsigned_abs() > terminal_price_after.u128() {
            // only allow terminal up when oracle is higher
            if terminal_price_after < terminal_price_before {
                return Err(ContractError::InvalidRepegDirection.into());
            }

            // only push terminal up to top of oracle confidence band
            if oracle_conf_band_bottom < terminal_price_after {
                return Err(ContractError::InvalidRepegProfitability.into());
            }

            // only push mark up to top of oracle confidence band
            if mark_price_after > oracle_conf_band_top {
                return Err(ContractError::InvalidRepegProfitability.into());
            }
        }

        if oracle_price.unsigned_abs() < terminal_price_after.u128() {
            // only allow terminal down when oracle is lower
            if terminal_price_after > terminal_price_before {
                return Err(ContractError::InvalidRepegDirection.into());
            }

            // only push terminal down to top of oracle confidence band
            if oracle_conf_band_top > terminal_price_after {
                return Err(ContractError::InvalidRepegProfitability.into());
            }

            // only push mark down to bottom of oracle confidence band
            if mark_price_after < oracle_conf_band_bottom {
                return Err(ContractError::InvalidRepegProfitability.into());
            }
        }
    }

    // Reduce pnl to quote asset precision and take the absolute value
    if adjustment_cost > 0 {
        market.amm.total_fee_minus_distributions = market
            .amm
            .total_fee_minus_distributions
            .checked_sub(Uint128::from(adjustment_cost.unsigned_abs()))?;

        // Only a portion of the protocol fees are allocated to repegging
        // This checks that the total_fee_minus_distributions does not decrease too much after repeg
        if market.amm.total_fee_minus_distributions
            < market
                .amm
                .total_fee
                .checked_mul(SHARE_OF_FEES_ALLOCATED_TO_CLEARING_HOUSE_NUMERATOR)?
                .checked_div(SHARE_OF_FEES_ALLOCATED_TO_CLEARING_HOUSE_DENOMINATOR)?
        {
            return Err(ContractError::InvalidRepegProfitability.into());
        }
    } else {
        market.amm.total_fee_minus_distributions = market
            .amm
            .total_fee_minus_distributions
            .checked_add(Uint128::from(adjustment_cost.unsigned_abs()))?;
    }

    MARKETS.update(deps.storage, market_index.to_string(), |_m| ->  Result<Market, ContractError>{
        Ok(market)
    })?;

    Ok(adjustment_cost)

}

pub fn adjust_peg_cost(market: &mut Market, new_peg: Uint128) -> Result<i128, ContractError> {
    // Find the net market value before adjusting peg
    let (current_net_market_value, _) =
        _calculate_base_asset_value_and_pnl(market.base_asset_amount.i128(), Uint128::zero(), &market.amm)?;

    market.amm.peg_multiplier = new_peg;

    let (_new_net_market_value, cost) = _calculate_base_asset_value_and_pnl(
        market.base_asset_amount.i128(),
        current_net_market_value,
        &market.amm,
    )?;

    Ok(cost)
}
