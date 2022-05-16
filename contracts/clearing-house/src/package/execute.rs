use cosmwasm_std::{Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub collateral_vault: String,
    pub insurance_vault: String,
    pub history_contract: String,
    pub admin_controls_prices: bool,
    pub oracle: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    // market initializer updates AMM structure
    InitializeMarket {
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
    },
    //deposit collateral, updates user struct
    DepositCollateral {
        amount: u64,
        referrer: Option<String>
    },
    //user function withdraw collateral, updates user struct
    WithdrawCollateral {
        amount: u64,
    },
    OpenPosition {
        is_direction_long: bool,
        quote_asset_amount: Uint128,
        market_index: u64,
        limit_price: Option<Uint128>,
    },
    ClosePosition {
        market_index: u64,
    },

    // order related messages
    // PlaceOrder {
    //     order: OrderParams,
    // },
    // CancelOrder {
    //     market_index: u64,
    //     order_id: u64,
    // },
    // ExpireOrders {
    //     user_address: String,
    // },
    // FillOrder {
    //     order_id: u64,
    //     user_address: String,
    //     market_index: u64,
    // },
    Liquidate {
        user: String,
        market_index: u64,
    },
    MoveAMMPrice {
        base_asset_reserve: Uint128,
        quote_asset_reserve: Uint128,
        market_index: u64,
    },
    //user function
    WithdrawFees {
        market_index: u64,
        amount: u64,
    },

    // withdraw from insurance vault sends token but no logic

    //admin function
    WithdrawFromInsuranceVaultToMarket {
        market_index: u64,
        amount: u64,
    },
    //admin function
    RepegAMMCurve {
        new_peg_candidate: Uint128,
        market_index: u64,
    },

    UpdateAMMOracleTwap {
        market_index: u64,
    },

    ResetAMMOracleTwap {
        market_index: u64,
    },
    //user calls it we get the user identification from msg address sender
    SettleFundingPayment {},
    UpdateFundingRate {
        market_index: u64,
    },
    UpdateK {
        market_index: u64,
        sqrt_k: Uint128,
    },
    UpdateMarginRatio {
        market_index: u64,
        margin_ratio_initial: u32,
        margin_ratio_partial: u32,
        margin_ratio_maintenance: u32,
    },
    UpdatePartialLiquidationClosePercentage {
        value: Decimal,
    },
    UpdatePartialLiquidationPenaltyPercentage {
        value: Decimal,
    },
    UpdateFullLiquidationPenaltyPercentage {
        value: Decimal,
    },
    UpdatePartialLiquidationLiquidatorShareDenominator {
        denominator: u64,
    },
    UpdateFullLiquidationLiquidatorShareDenominator {
        denominator: u64,
    },
    UpdateFee {
        fee_: Decimal,
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
    },
    UpdateOraceGuardRails {
        use_for_liquidations: bool,
        mark_oracle_divergence: Decimal,
        slots_before_stale: i64,
        confidence_interval_max_size: Uint128,
        too_volatile_ratio: i128,
    },
    UpdateOrderState {
        min_order_quote_asset_amount: Uint128,
        reward: Decimal,
        time_based_reward_lower_bound: Uint128,
    },
    UpdateMarketOracle {
        market_index: u64,
        oracle: String,
        oracle_source_code: u8,
    },
    UpdateOracleAddress {
        oracle: String,
    },
    UpdateMarketMinimumQuoteAssetTradeSize {
        market_index: u64,
        minimum_trade_size: Uint128,
    },

    UpdateMarketMinimumBaseAssetTradeSize {
        market_index: u64,
        minimum_trade_size: Uint128,
    },
    // will move to admin controller
    UpdateAdmin {
        admin: String,
    },
    UpdateHistoryStore {
        history_contract: String,
    },
    UpdateMaxDeposit {
        max_deposit: Uint128,
    },
    UpdateExchangePaused {
        exchange_paused: bool,
    },
    DisableAdminControlsPrices {},
    UpdateFundingPaused {
        funding_paused: bool,
    },
    OracleFeeder {
        market_index: u64,
        price: Uint128,
    },
}
