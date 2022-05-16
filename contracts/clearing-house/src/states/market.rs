use crate::package::number::Number128;
use crate::package::oracle::{OracleQueryMsg, PriceResponse};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128, DepsMut, WasmQuery, QueryRequest, to_binary};

use cw_storage_plus::Map;

use crate::package::types::{OracleSource, OracleStatus, OraclePriceData};

use crate::error::ContractError;

use crate::helpers::amm;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Market {
    pub market_name: String,
    pub initialized: bool,
    pub base_asset_amount_long: Number128,
    pub base_asset_amount_short: Number128,
    pub base_asset_amount: Number128, // net market bias
    pub open_interest: Uint128,     // number of users in a position
    pub amm: Amm,
    pub margin_ratio_initial: u32,
    pub margin_ratio_partial: u32,
    pub margin_ratio_maintenance: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Amm {
    pub oracle: Addr,
    pub oracle_source: OracleSource,
    pub base_asset_reserve: Uint128,
    pub quote_asset_reserve: Uint128,
    pub cumulative_repeg_rebate_long: Uint128,
    pub cumulative_repeg_rebate_short: Uint128,
    pub cumulative_funding_rate_long: Number128,
    pub cumulative_funding_rate_short: Number128,
    pub last_funding_rate: Number128,
    pub last_funding_rate_ts: u64,
    pub funding_period: u64,
    pub sqrt_k: Uint128,
    pub peg_multiplier: Uint128,
    pub total_fee: Uint128,
    pub last_mark_price_twap: Uint128,
    pub last_mark_price_twap_ts: u64,
    pub total_fee_minus_distributions: Uint128,
    pub total_fee_withdrawn: Uint128,
    pub minimum_quote_asset_trade_size: Uint128,
    pub last_oracle_price_twap_ts: u64,
    pub last_oracle_price: Number128,
    pub last_oracle_price_twap: Number128,
    pub minimum_base_asset_trade_size: Uint128,
}

pub const MARKETS: Map<String, Market> = Map::new("markets");

impl Amm {
    pub fn mark_price(&self) -> Result<Uint128, ContractError> {
        amm::calculate_price(
            self.quote_asset_reserve,
            self.base_asset_reserve,
            self.peg_multiplier,
        )
    }

    pub fn get_oracle_price(
        &self,
        deps: &mut DepsMut,
        market_index: u64
    ) -> Result<OraclePriceData, ContractError> {
        let x: PriceResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: self.oracle.to_string(),
            msg: to_binary(&OracleQueryMsg::Price { asset: "btc".to_string() })?,
        }))?;

        let price = x.price.u128();
        let mut market = MARKETS.load(deps.storage, market_index.to_string())?;
        market.amm.last_oracle_price = Number128::new(price as i128);
        MARKETS.update(deps.storage, market_index.to_string(), |_m| -> Result<Market,ContractError> {
            Ok(market)
        })?;

        Ok(OraclePriceData {
            // price: self.last_oracle_price,
            price: Number128::new(price as i128),
            confidence: Uint128::from(100 as u32),
            delay: 0,
            has_sufficient_number_of_data_points: true,
        })
    }

    pub fn get_oracle_twap(&self) -> Result<Option<i128>, ContractError> {
        // match self.oracle_source {
        //     OracleSource::Oracle => Ok(Some(self.fetch_oracle_twap()?)),
        //     // OracleSource::Bank => Ok(Some(self.fetch_bank_twap()?)),
        // }
        if self.last_mark_price_twap.ne(&Uint128::zero()) {
            Ok(Some(self.last_oracle_price_twap.i128()))
        } else {
            Ok(None)
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum LiquidationType {
    NONE,
    PARTIAL,
    FULL,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LiquidationStatus {
    pub liquidation_type: LiquidationType,
    pub margin_requirement: Uint128,
    pub total_collateral: Uint128,
    pub unrealized_pnl: i128,
    pub adjusted_total_collateral: Uint128,
    pub base_asset_value: Uint128,
    pub margin_ratio: Uint128,
    pub market_statuses: Vec<MarketStatus>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MarketStatus {
    pub market_index: u64,
    pub partial_margin_requirement: Uint128,
    pub maintenance_margin_requirement: Uint128,
    pub base_asset_value: Uint128,
    pub mark_price_before: Uint128,
    pub close_position_slippage: Option<i128>,
    pub oracle_status: OracleStatus,
}