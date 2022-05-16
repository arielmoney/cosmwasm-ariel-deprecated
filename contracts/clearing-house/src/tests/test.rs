use crate::contract::{execute, instantiate, query};
use crate::states::constants::{DEFAULT_FEE_DENOMINATOR, DEFAULT_FEE_NUMERATOR};
use crate::states::market::Market;
use crate::views::execute_admin::{
    try_feeding_price, try_initialize_market, try_update_exchange_paused,
    try_update_market_minimum_base_asset_trade_size,
    try_update_market_minimum_quote_asset_trade_size,
};
use crate::views::execute_user::{
    try_close_position, try_deposit_collateral, try_liquidate, try_open_position,
    try_settle_funding_payment, try_withdraw_collateral,
};
use crate::views::query;

use crate::package::execute::InstantiateMsg;
use crate::package::number::Number128;
use crate::package::queries::QueryMsg;
use crate::package::response::*;

use crate::package::types::{DepositDirection, OracleSource, PositionDirection};
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockQuerier, MOCK_CONTRACT_ADDR,
};
use cosmwasm_std::{
    coins, from_binary, Addr, BalanceResponse, BankQuery, Decimal, QueryRequest, StdResult, Uint128,
};

const ADMIN_ACCOUNT: &str = "admin_account";

// #[test]
pub fn test_initialize_state() {
    let mut deps = mock_dependencies(&coins(0, "token"));

    let msg = InstantiateMsg {
        collateral_vault: String::from(MOCK_CONTRACT_ADDR),
        insurance_vault: String::from(MOCK_CONTRACT_ADDR),
        admin_controls_prices: true,
        oracle: String::from(MOCK_CONTRACT_ADDR),
        history_contract: String::from(MOCK_CONTRACT_ADDR),
    };

    let info = mock_info(ADMIN_ACCOUNT, &coins(0, "earth"));

    // we can just call .unwrap() to assert this was a success
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // let res = query(deps.as_ref(), mock_env(), QueryMsg::GetAdmin {}).unwrap();
    // let value: AdminResponse = from_binary(&res).unwrap();
    // assert_eq!(String::from(ADMIN_ACCOUNT), value.admin);
    //query market_length
    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetMarketLength {});
    match res {
        Ok(_res) => {
            let value: MarketLengthResponse = from_binary(&_res).unwrap();
            assert_eq!(0, value.length);
        }
        Err(err) => {
            println!("{} days", err);
        }
    }
    //query vault address
    // let res = query(deps.as_ref(), mock_env(), QueryMsg::GetVaults {}).unwrap();
    // let value: VaultsResponse = from_binary(&res).unwrap();
    // assert_eq!(String::from(MOCK_CONTRACT_ADDR), value.collateral_vault); //collateral vault set
    // assert_eq!(String::from(MOCK_CONTRACT_ADDR), value.insurance_vault); // insurance vault setassert_eq!(String::from(MOCK_CONTRACT_ADDR), value.insurance_vault); // insurance vault set
                                                                         // query admin

    //query isexchangepaused

    // let res = query(deps.as_ref(), mock_env(), QueryMsg::IsExchangePaused {}).unwrap();
    // let value: IsExchangePausedResponse = from_binary(&res).unwrap();
    // assert_eq!(false, value.exchange_paused);

    // //query funding paused

    // let res = query(deps.as_ref(), mock_env(), QueryMsg::IsFundingPaused {}).unwrap();
    // let value: IsFundingPausedResponse = from_binary(&res).unwrap();
    // assert_eq!(false, value.funding_paused);
    // //query admin controls prices

    // let res = query(deps.as_ref(), mock_env(), QueryMsg::AdminControlsPrices {}).unwrap();
    // let value: AdminControlsPricesResponse = from_binary(&res).unwrap();
    // assert_eq!(true, value.admin_controls_prices);

    // //query margin ratio

    // let res = query(deps.as_ref(), mock_env(), QueryMsg::GetMarginRatio {}).unwrap();
    // let value: MarginRatioResponse = from_binary(&res).unwrap();
    // assert_eq!(Uint128::from(2000u128), value.margin_ratio_initial);
    // assert_eq!(Uint128::from(500u128), value.margin_ratio_maintenance);
    // assert_eq!(Uint128::from(625u128), value.margin_ratio_partial);

    // //query oracle
    // let res = query(deps.as_ref(), mock_env(), QueryMsg::GetOracle {}).unwrap();
    // let value: OracleResponse = from_binary(&res).unwrap();
    // assert_eq!(MOCK_CONTRACT_ADDR, value.oracle);

    //query oracle guard rails
    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetOracleGuardRails {}).unwrap();
    let value: OracleGuardRailsResponse = from_binary(&res).unwrap();
    assert_eq!(true, value.use_for_liquidations);
    assert_eq!(1000, value.slots_before_stale.i128());

    // query order state
    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetOrderState {}).unwrap();
    let value: OrderStateResponse = from_binary(&res).unwrap();
    assert_eq!(Uint128::zero(), value.min_order_quote_asset_amount);

    //query partial liq close
    // let res = query(
    //     deps.as_ref(),
    //     mock_env(),
    //     QueryMsg::GetPartialLiquidationClosePercentage {},
    // )
    // .unwrap();
    // let value: PartialLiquidationClosePercentageResponse = from_binary(&res).unwrap();
    // // 25/100
    // assert_eq!(Decimal::percent(25), value.value);

    //query partial liq penalty
    //query full liq penalty
    //query partial liq share perc
    //query full liq share perc
    //query max deposit
    // let res = query(deps.as_ref(), mock_env(), QueryMsg::GetMaxDepositLimit {}).unwrap();
    // let value: MaxDepositLimitResponse = from_binary(&res).unwrap();
    // assert_eq!(Uint128::zero(), value.max_deposit);

    // query fee structure
    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetFeeStructure {}).unwrap();
    let value: FeeStructureResponse = from_binary(&res).unwrap();
    assert_eq!(
        Decimal::from_ratio(DEFAULT_FEE_NUMERATOR, DEFAULT_FEE_DENOMINATOR),
        value.fee
    );
}

// #[test]
pub fn test_deposit_withdraw() {
    let mut deps = mock_dependencies(&coins(0, "token"));

    let msg = InstantiateMsg {
        collateral_vault: String::from(MOCK_CONTRACT_ADDR),
        insurance_vault: String::from(MOCK_CONTRACT_ADDR),
        admin_controls_prices: true,
        oracle: String::from(MOCK_CONTRACT_ADDR),
        history_contract: String::from(MOCK_CONTRACT_ADDR),
    };

    let info = mock_info(ADMIN_ACCOUNT, &coins(0, "earth"));

    // we can just call .unwrap() to assert this was a success
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    // intialize market
    let market_init_info = mock_info(ADMIN_ACCOUNT, &coins(0, "earth"));
    let amm_base_asset_reserve = Uint128::from(5000_000_000u128);
    let amm_quote_asset_reserve = Uint128::from(5000_000_000u128);
    let amm_periodicity = 10;
    let oracle_source_code = 0;
    let amm_peg_multiplier = Uint128::from(92_19_0000u128);
    let margin_ratio_initial = 2000;
    let margin_ratio_partial = 625;
    let margin_ratio_maintenance = 500;
    try_initialize_market(
        deps.as_mut(),
        mock_env(),
        market_init_info,
        1,
        "LUNA-UST".to_string(),
        amm_base_asset_reserve,
        amm_quote_asset_reserve,
        amm_periodicity,
        amm_peg_multiplier,
        oracle_source_code,
        margin_ratio_initial,
        margin_ratio_partial,
        margin_ratio_maintenance,
    )
    .unwrap();

    // println!("{} contract error", err);
    //test market params
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetMarketInfo { market_index: 1 },
    )
    .unwrap();
    let value: MarketInfoResponse = from_binary(&res).unwrap();
    assert_eq!("LUNA-UST".to_string(), value.market_name);
    assert_eq!(amm_base_asset_reserve, value.sqrt_k);

    let res = query(deps.as_ref(), mock_env(), QueryMsg::GetMarketLength {}).unwrap();
    let value: MarketLengthResponse = from_binary(&res).unwrap();
    assert_eq!(1, value.length);

    let deposit_info = mock_info("geekybot", &coins(100_000_000, "uusd"));

    try_deposit_collateral(deps.as_mut(), mock_env(), deposit_info, 100_000_000, None).unwrap();
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetUser {
            user_address: "geekybot".to_string(),
        },
    )
    .unwrap();
    let value: UserResponse = from_binary(&res).unwrap();
    assert_eq!(Uint128::from(100_000_000u128), value.collateral);
    assert_eq!(Uint128::zero(), value.total_fee_paid);

    // let res = query(
    //     deps.as_ref(),
    //     mock_env(),
    //     QueryMsg::GetLength {  },
    // )
    // .unwrap();
    // let value: LengthResponse = from_binary(&res).unwrap();
    // assert_eq!(1, value.deposit_history_length);

    // let res = query(
    //     deps.as_ref(),
    //     mock_env(),
    //     QueryMsg::GetDepositHistory {
    //         user_address: "geekybot".to_string(),
    //         start_after: None,
    //         limit: None,
    //     },
    // )
    // .unwrap();
    // let value: Vec<DepositHistoryResponse> = from_binary(&res).unwrap();
    // assert_eq!(100000000, value[0].amount);
    // assert_eq!(DepositDirection::DEPOSIT, value[0].direction);

    // //test withdraw
    // let withdraw_info = mock_info("geekybot", &coins(0, "uusd"));
    // try_withdraw_collateral(deps.as_mut(), mock_env(), withdraw_info, 100_000_000).unwrap();

    // let res = query(
    //     deps.as_ref(),
    //     mock_env(),
    //     QueryMsg::GetUser {
    //         user_address: "geekybot".to_string(),
    //     },
    // )
    // .unwrap();
    // let value: UserResponse = from_binary(&res).unwrap();
    // assert_eq!(Uint128::from(100_000_000u128), value.collateral);
    // assert_eq!(Uint128::zero(), value.total_fee_paid);

    // let res = query(
    //     deps.as_ref(),
    //     mock_env(),
    //     QueryMsg::GetLength {},
    // )
    // .unwrap();
    // let value: LengthResponse = from_binary(&res).unwrap();
    // assert_eq!(2, value.deposit_history_length);

    // let res = query(
    //     deps.as_ref(),
    //     mock_env(),
    //     QueryMsg::GetDepositHistory {
    //         user_address: "geekybot".to_string(),
    //         start_after: None,
    //         limit: None,
    //     },
    // )
    // .unwrap();
    // let value: Vec<DepositHistoryResponse> = from_binary(&res).unwrap();
    // assert_eq!(100000000, value[1].amount);
    // assert_eq!(0, value[0].amount);
    // assert_eq!(DepositDirection::WITHDRAW, value[0].direction);
}

// #[test]
// pub fn test_open_position() {
//     let mut deps = mock_dependencies(&coins(0, "token"));

//     let msg = InstantiateMsg {
//         collateral_vault: String::from(MOCK_CONTRACT_ADDR),
//         insurance_vault: String::from(MOCK_CONTRACT_ADDR),
//         admin_controls_prices: true,
//         oracle: String::from(MOCK_CONTRACT_ADDR),
//         history_contract: String::from(MOCK_CONTRACT_ADDR),
//     };

//     let info = mock_info(ADMIN_ACCOUNT, &coins(0, "earth"));

//     // we can just call .unwrap() to assert this was a success
//     instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
//     // intialize market
//     let market_init_info = mock_info(ADMIN_ACCOUNT, &coins(0, "earth"));
//     let amm_base_asset_reserve = Uint128::from(5000_000_000u128);
//     let amm_quote_asset_reserve = Uint128::from(5000_000_000u128);
//     let amm_periodicity = 10;
//     let oracle_source_code = 0;
//     let amm_peg_multiplier = Uint128::from(92_19_0000u128);
//     let margin_ratio_initial = 2000;
//     let margin_ratio_partial = 625;
//     let margin_ratio_maintenance = 500;
//     try_initialize_market(
//         deps.as_mut(),
//         mock_env(),
//         market_init_info,
//         1,
//         "LUNA-UST".to_string(),
//         amm_base_asset_reserve,
//         amm_quote_asset_reserve,
//         amm_periodicity,
//         amm_peg_multiplier,
//         oracle_source,
//         margin_ratio_initial,
//         margin_ratio_partial,
//         margin_ratio_maintenance,
//     )
//     .unwrap();

//     try_feeding_price(
//         deps.as_mut(),
//         mock_info(ADMIN_ACCOUNT, &coins(0, "tt")),
//         1,
//         92_45_00_00,
//     )
//     .unwrap();
//     let deposit_info = mock_info("geekybot", &coins(100_000_000, "uusd"));

//     try_deposit_collateral(deps.as_mut(), mock_env(), deposit_info, 100_000_000, None).unwrap();

//     let open_position_info = mock_info("geekybot", &coins(0, "denom"));
//     let quote_asset_amount = Uint128::from(20_000_000u128);
//     let limit_price = Uint128::zero();
//     try_open_position(
//         deps.as_mut(),
//         mock_env(),
//         open_position_info,
//         PositionDirection::Short,
//         quote_asset_amount,
//         1,
//         None,
//     )
//     .unwrap();
//     let res = query(
//         deps.as_ref(),
//         mock_env(),
//         QueryMsg::GetUser {
//             user_address: "geekybot".to_string(),
//         },
//     )
//     .unwrap();
//     let value: UserResponse = from_binary(&res).unwrap();
//     // assert_eq!(Uint128::from(90_000_000u128), value.collateral);
//     // assert_eq!(Uint128::from(5_000u128), value.total_fee_paid);

//     let res = query(
//         deps.as_ref(),
//         mock_env(),
//         QueryMsg::GetUserPositions {
//             user_address: "geekybot".to_string(),
//             start_after: None,
//             limit: None,
//         },
//     )
//     .unwrap();
//     let value: Vec<PositionResponse> = from_binary(&res).unwrap();
//     assert_eq!(PositionDirection::Short, value[0].direction);
//     // println!("test {}", err);

//     //########## opening another position of long
//     let open_position_info = mock_info("geekybot", &coins(0, "denom"));
//     let quote_asset_amount = Uint128::from(40_000_000u128);
//     let limit_price = Uint128::zero();
//     try_open_position(
//         deps.as_mut(),
//         mock_env(),
//         open_position_info,
//         PositionDirection::Long,
//         quote_asset_amount,
//         1,
//         None,
//     )
//     .unwrap();
//     let res = query(
//         deps.as_ref(),
//         mock_env(),
//         QueryMsg::GetUser {
//             user_address: "geekybot".to_string(),
//         },
//     )
//     .unwrap();
//     let value: UserResponse = from_binary(&res).unwrap();
//     // assert_eq!(Uint128::from(90_000_000u128), value.collateral);
//     assert_eq!(Uint128::from(45_000u128), value.total_fee_paid);

//     let res = query(
//         deps.as_ref(),
//         mock_env(),
//         QueryMsg::GetUserPositions {
//             user_address: "geekybot".to_string(),
//             start_after: None,
//             limit: None,
//         },
//     )
//     .unwrap();
//     let value: Vec<PositionResponse> = from_binary(&res).unwrap();
//     assert_eq!(PositionDirection::Long, value[0].direction);

//     // trying to reduce long position by opening a short of 45ust
//     let open_position_info = mock_info("geekybot", &coins(0, "denom"));
//     let quote_asset_amount = Uint128::from(45_000_000u128);
//     let limit_price = Uint128::zero();
//     try_open_position(
//         deps.as_mut(),
//         mock_env(),
//         open_position_info,
//         PositionDirection::Short,
//         quote_asset_amount,
//         1,
//         None,
//     )
//     .unwrap();
//     let res = query(
//         deps.as_ref(),
//         mock_env(),
//         QueryMsg::GetUser {
//             user_address: "geekybot".to_string(),
//         },
//     )
//     .unwrap();
//     let value: UserResponse = from_binary(&res).unwrap();
//     // assert_eq!(Uint128::from(90_000_000u128), value.collateral);
//     // assert_eq!(Uint128::from(45_000u128), value.total_fee_paid);

//     let res = query(
//         deps.as_ref(),
//         mock_env(),
//         QueryMsg::GetUserPositions {
//             user_address: "geekybot".to_string(),
//             start_after: None,
//             limit: None,
//         },
//     )
//     .unwrap();
//     let value: Vec<PositionResponse> = from_binary(&res).unwrap();
//     assert_eq!(PositionDirection::Short, value[0].direction);

//     let res = query(
//         deps.as_ref(),
//         mock_env(),
//         QueryMsg::GetLength {},
//     )
//     .unwrap();
//     let value: LengthResponse = from_binary(&res).unwrap();
//     // assert_eq!(1, value.length);

//     // get trade_history
//     let res = query(
//         deps.as_ref(),
//         mock_env(),
//         QueryMsg::GetTradeHistory {
//             start_after: None,
//             limit: None,
//         },
//     )
//     .unwrap();
//     let value: Vec<TradeHistoryResponse> = from_binary(&res).unwrap();
//     assert_eq!(PositionDirection::Long, value[0].direction);
//     assert_eq!(PositionDirection::Long, value[1].direction);
//     assert_eq!(PositionDirection::Short, value[2].direction);

//     //###### get Position of the user
//     let res = query(
//         deps.as_ref(),
//         mock_env(),
//         QueryMsg::GetUserMarketPosition {
//             user_address: "geekybot".to_string(),
//             index: 1,
//         },
//     )
//     .unwrap();
//     let value: UserPositionResponse = from_binary(&res).unwrap();
//     assert_eq!(Uint128::from(80_000_000u128), value.quote_asset_amount);
// }

// #[test]
// pub fn test_short_position() {
//     let mut deps = mock_dependencies(&coins(0, "token"));

//     let msg = InstantiateMsg {
//         collateral_vault: String::from("collateral_vault"),
//         insurance_vault: String::from("insurance_vault"),
//         admin_controls_prices: true,
//         oracle: String::from(MOCK_CONTRACT_ADDR),
//     };

//     let info = mock_info(ADMIN_ACCOUNT, &coins(0, "earth"));

//     // we can just call .unwrap() to assert this was a success
//     instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
//     // intialize market
//     let market_init_info = mock_info(ADMIN_ACCOUNT, &coins(0, "earth"));
//     let amm_base_asset_reserve = Uint128::from(1_000_000_000_000_000_0000u128); // 5M 13 precision
//     let amm_quote_asset_reserve = Uint128::from(1_000_000_000_000_000_0000u128); //5M 13 precision
//     let amm_periodicity = 10;
//     let oracle_source = OracleSource::Oracle;
//     let amm_peg_multiplier = Uint128::from(92_190u128);
//     let margin_ratio_initial = 2000;
//     let margin_ratio_partial = 625;
//     let margin_ratio_maintenance = 500;
//     try_initialize_market(
//         deps.as_mut(),
//         mock_env(),
//         market_init_info,
//         1,
//         "LUNA-UST".to_string(),
//         amm_base_asset_reserve,
//         amm_quote_asset_reserve,
//         amm_periodicity,
//         amm_peg_multiplier,
//         oracle_source,
//         margin_ratio_initial,
//         margin_ratio_partial,
//         margin_ratio_maintenance,
//     )
//     .unwrap();

//     try_feeding_price(
//         deps.as_mut(),
//         mock_info(ADMIN_ACCOUNT, &coins(0, "tt")),
//         1,
//         92_450_000_000_0,
//     )
//     .unwrap();
//     let deposit_info = mock_info("geekybot", &coins(100_000_000, "uusd"));

//     try_deposit_collateral(deps.as_mut(), mock_env(), deposit_info, 100_000_000, None).unwrap();

//     let open_position_info = mock_info("geekybot", &coins(0, "denom"));
//     let quote_asset_amount = Uint128::from(50000_000_000u128);
//     let limit_price = Uint128::zero();
//     try_open_position(
//         deps.as_mut(),
//         mock_env(),
//         open_position_info,
//         PositionDirection::Long,
//         quote_asset_amount,
//         1,
//         None,
//     )
//     .unwrap();
//     let res = query(
//         deps.as_ref(),
//         mock_env(),
//         QueryMsg::GetUser {
//             user_address: "geekybot".to_string(),
//         },
//     )
//     .unwrap();
//     let value: UserResponse = from_binary(&res).unwrap();
//     // assert_eq!(Uint128::from(80_000_000u128), value.collateral);
//     // assert_eq!(Uint128::from(5_000u128), value.total_fee_paid);

//     let res = query(
//         deps.as_ref(),
//         mock_env(),
//         QueryMsg::GetUserPositions {
//             user_address: "geekybot".to_string(),
//             start_after: None,
//             limit: None,
//         },
//     )
//     .unwrap();
//     let value: Vec<PositionResponse> = from_binary(&res).unwrap();
//     assert_eq!(PositionDirection::Long, value[0].direction);
//     // assert_eq!(Uint128::zero(), value[0].entry_price);
//     assert_eq!(Number128::zero(), value[0].entry_notional);
// }

#[test]
pub fn user_functions_test() {
    let mut deps = mock_dependencies(&coins(0, "token"));

    let msg = InstantiateMsg {
        collateral_vault: String::from("collateral_vault"),
        insurance_vault: String::from("insurance_vault"),
        admin_controls_prices: true,
        oracle: String::from(MOCK_CONTRACT_ADDR),
        history_contract: String::from(MOCK_CONTRACT_ADDR),
    };

    let info = mock_info(ADMIN_ACCOUNT, &coins(0, "earth"));

    // we can just call .unwrap() to assert this was a success
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    // intialize market
    let market_init_info = mock_info(ADMIN_ACCOUNT, &coins(0, "earth"));
    let amm_base_asset_reserve = Uint128::from(5_000_000_000_000_000_000u128);
    let amm_quote_asset_reserve = Uint128::from(5_000_000_000_000_000_000u128);
    let amm_periodicity = 3600;
    let oracle_source_code = 0;
    let amm_peg_multiplier = Uint128::from(1000u128);
    let margin_ratio_initial = 2000;
    let margin_ratio_partial = 625;
    let margin_ratio_maintenance = 500;
   let err= try_initialize_market(
        deps.as_mut(),
        mock_env(),
        market_init_info,
        1,
        "LUNA-UST".to_string(),
        amm_base_asset_reserve,
        amm_quote_asset_reserve,
        amm_periodicity,
        amm_peg_multiplier,
        oracle_source_code,
        margin_ratio_initial,
        margin_ratio_partial,
        margin_ratio_maintenance,
    )
    .unwrap_err();
    println!("{}", err);

    let deposit_info = mock_info("geekybot", &coins(10_000_000, "uusd"));

    try_deposit_collateral(deps.as_mut(), mock_env(), deposit_info, 10_000_000, None).unwrap();

    try_feeding_price(
        deps.as_mut(),
        mock_info(ADMIN_ACCOUNT, &coins(0, "tt")),
        1,
        Uint128::from(92_450_000_000_0u128),
    )
    .unwrap();

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetMarketInfo { market_index: 1 },
    )
    .unwrap();
    let value: MarketInfoResponse = from_binary(&res).unwrap();
    assert_eq!(Number128::new(92_450_000_000_0), value.last_oracle_price);

    let mut trade_amount = calculate_trade_amount(10_000_000).unwrap();

    let open_position_info = mock_info("geekybot", &coins(0, "denom"));
    let quote_asset_amount = trade_amount;
    try_open_position(
        deps.as_mut(),
        mock_env(),
        open_position_info.clone(),
        true,
        quote_asset_amount,
        1,
        None,
    )
    .unwrap();
    try_open_position(
        deps.as_mut(),
        mock_env(),
        open_position_info.clone(),
        false,
        quote_asset_amount,
        1,
        None,
    )
    .unwrap();
    try_open_position(
        deps.as_mut(),
        mock_env(),
        open_position_info.clone(),
        true,
        quote_asset_amount,
        1,
        None,
    )
    .unwrap();
    try_open_position(
        deps.as_mut(),
        mock_env(),
        open_position_info.clone(),
        true,
        quote_asset_amount,
        1,
        None,
    )
    .unwrap();
    try_open_position(
        deps.as_mut(),
        mock_env(),
        open_position_info.clone(),
        true,
        quote_asset_amount,
        1,
        None,
    )
    .unwrap();

    try_deposit_collateral(deps.as_mut(), mock_env(), mock_info("whocares", &coins(10_000_000, "uusd")), 10_000_000, None).unwrap();
    try_open_position(
        deps.as_mut(),
        mock_env(),
        mock_info("whocares", &coins(235, "test")),
        true,
        quote_asset_amount,
        1,
        None,
    )
    .unwrap();
    try_open_position(
        deps.as_mut(),
        mock_env(),
        mock_info("whocares", &coins(235, "test")),
        false,
        quote_asset_amount,
        1,
        None,
    )
    .unwrap();
    try_open_position(
        deps.as_mut(),
        mock_env(),
        open_position_info.clone(),
        true,
        quote_asset_amount,
        1,
        None,
    )
    .unwrap();
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetUser {
            user_address: "geekybot".to_string(),
        },
    )
    .unwrap();
    let value: UserResponse = from_binary(&res).unwrap();
    // assert_eq!(Uint128::from(9_950_250u128), value.collateral);
    // assert_eq!(Uint128::from(49_750u128), value.total_fee_paid);
    // assert_eq!(Uint128::from(10_000_000u128), value.cumulative_deposits);

    // try_feeding_price(
    //     deps.as_mut(),
    //     mock_info(ADMIN_ACCOUNT, &coins(0, "tt")),
    //     1,
    //     Uint128::from(1u128),
    // )
    // .unwrap();
    let liquidate_position_info = mock_info("geekybot", &coins(0, "denom"));

    // let err =try_liquidate(deps.as_mut(), mock_env(), liquidate_position_info, "geekybot".to_string(), 1).unwrap();
    // println!("{}", err);
    let close_position_info = mock_info("geekybot", &coins(0, "denom"));
    try_close_position(deps.as_mut(), mock_env(), close_position_info, 1).unwrap();
    // let res = query(
    //     deps.as_ref(),
    //     mock_env(),
    //     QueryMsg::GetUserPositions {
    //         user_address: "geekybot".to_string(),
    //         start_after: None,
    //         limit: None,
    //     },
    // )
    // .unwrap();
    // let value: Vec<PositionResponse> = from_binary(&res).unwrap();
    // assert_eq!(PositionDirection::Long, value[0].direction);
    // assert_eq!(
    //     Number128::new(497450503674885i128),
    //     value[0].base_asset_amount
    // );
    // assert_eq!(Uint128::from(49750000u128), value[0].quote_asset_amount);
    // // assert_eq!(Uint128::zero(), value[0].entry_price);
    // // assert_eq!(Number128::zero(), value[0].entry_notional);
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetMarketInfo { market_index: 1 },
    )
    .unwrap();
    let value: MarketInfoResponse = from_binary(&res).unwrap();
    assert_eq!("LUNA-UST".to_string(), value.market_name);
    assert_eq!(amm_base_asset_reserve, value.sqrt_k);

    // assert_eq!(Number128::new(497450503674885i128), value.base_asset_amount);
    // assert_eq!(Uint128::from(49_750u128), value.total_fee);
    // assert_eq!(
    //     Uint128::from(49_750u128),
    //     value.total_fee_minus_distributions
    // );
    // // assert_eq!("1".to_string(), value.last_mark_price_twap.to_string());

    // // reduce long position
    // trade_amount = trade_amount.checked_div(Uint128::from(2u128)).unwrap();
    // try_open_position(
    //     deps.as_mut(),
    //     mock_env(),
    //     mock_info("geekybot", &coins(0, "denom")),
    //     false,
    //     trade_amount,
    //     1,
    //     None,
    // )
    // .unwrap();

    // let res = query(
    //     deps.as_ref(),
    //     mock_env(),
    //     QueryMsg::GetUser {
    //         user_address: "geekybot".to_string(),
    //     },
    // )
    // .unwrap();
    // let value: UserResponse = from_binary(&res).unwrap();
    // assert_eq!(Uint128::from(9_921_663u128), value.collateral);
    // assert_eq!(Uint128::from(74_625u128), value.total_fee_paid);
    // assert_eq!(Uint128::from(10_000_000u128), value.cumulative_deposits);

    // let res = query(
    //     deps.as_ref(),
    //     mock_env(),
    //     QueryMsg::GetUserPositions {
    //         user_address: "geekybot".to_string(),
    //         start_after: None,
    //         limit: None,
    //     },
    // )
    // .unwrap();
    // let value: Vec<PositionResponse> = from_binary(&res).unwrap();
    // assert_eq!(PositionDirection::Long, value[0].direction);
    // assert_eq!(Number128::new(248688127746683), value[0].base_asset_amount);
    // assert_eq!(Uint128::from(24871288u128), value[0].quote_asset_amount);
    // // assert_eq!(Uint128::zero(), value[0].entry_price);
    // // assert_eq!(Number128::zero(), value[0].entry_notional);
    // let res = query(
    //     deps.as_ref(),
    //     mock_env(),
    //     QueryMsg::GetMarketInfo { market_index: 1 },
    // )
    // .unwrap();
    // let value: MarketInfoResponse = from_binary(&res).unwrap();
    // assert_eq!("LUNA-UST".to_string(), value.market_name);
    // assert_eq!(amm_base_asset_reserve, value.sqrt_k);
    // assert_eq!(Number128::new(248688127746683i128), value.base_asset_amount);
    // assert_eq!(Uint128::from(74_625u128), value.total_fee);
    // assert_eq!(
    //     Uint128::from(74_625u128),
    //     value.total_fee_minus_distributions
    // );

    // //#### reverse long position
    // let trade_amount = calculate_trade_amount(10_000_000).unwrap();
    // try_open_position(
    //     deps.as_mut(),
    //     mock_env(),
    //     mock_info("geekybot", &coins(0, "denom")),
    //     PositionDirection::Short,
    //     trade_amount,
    //     1,
    //     None,
    // )
    // .unwrap();

    // let res = query(
    //     deps.as_ref(),
    //     mock_env(),
    //     QueryMsg::GetUser {
    //         user_address: "geekybot".to_string(),
    //     },
    // )
    // .unwrap();
    // let value: UserResponse = from_binary(&res).unwrap();
    // assert_eq!(Uint128::from(9_868_200u128), value.collateral);
    // assert_eq!(Uint128::from(124_375u128), value.total_fee_paid);
    // assert_eq!(Uint128::from(10_000_000u128), value.cumulative_deposits);

    // let res = query(
    //     deps.as_ref(),
    //     mock_env(),
    //     QueryMsg::GetUserPositions {
    //         user_address: "geekybot".to_string(),
    //         start_after: None,
    //         limit: None,
    //     },
    // )
    // .unwrap();
    // let value: Vec<PositionResponse> = from_binary(&res).unwrap();
    // assert_eq!(PositionDirection::Short, value[0].direction);
    // assert_eq!(Number128::new(-248836633317731), value[0].base_asset_amount);
    // assert_eq!(Uint128::from(24882425u128), value[0].quote_asset_amount);
    // // assert_eq!(Uint128::zero(), value[0].entry_price);
    // // assert_eq!(Number128::zero(), value[0].entry_notional);
    // let res = query(
    //     deps.as_ref(),
    //     mock_env(),
    //     QueryMsg::GetMarketInfo { market_index: 1 },
    // )
    // .unwrap();
    // let value: MarketInfoResponse = from_binary(&res).unwrap();
    // assert_eq!(amm_base_asset_reserve, value.sqrt_k);
    // assert_eq!(
    //     Number128::new(-248836633317731i128),
    //     value.base_asset_amount
    // );
    // assert_eq!(Uint128::from(124_375u128), value.total_fee);
    // assert_eq!(
    //     Uint128::from(124_375u128),
    //     value.total_fee_minus_distributions
    // );

    // let res = query(
    //     deps.as_ref(),
    //     mock_env(),
    //     QueryMsg::GetTradeHistory {
    //         start_after: None,
    //         limit: None,
    //     },
    // )
    // .unwrap();
    // let value: Vec<TradeHistoryResponse> = from_binary(&res).unwrap();
    // assert_eq!(PositionDirection::Short, value[0].direction);
    // assert_eq!(PositionDirection::Short, value[1].direction);
    // assert_eq!(PositionDirection::Long, value[2].direction);
    // assert_eq!(
    //     Uint128::new(497524761064414u128),
    //     value[0].base_asset_amount
    // );
    // assert_eq!(
    //     Uint128::new(248762375928202u128),
    //     value[1].base_asset_amount
    // );
    // assert_eq!(
    //     Uint128::new(497450503674885u128),
    //     value[2].base_asset_amount
    // );

    // //close position

    // try_close_position(
    //     deps.as_mut(),
    //     mock_env(),
    //     mock_info("geekybot", &coins(0, "denom")),
    //     1
    // )
    // .unwrap();

    // let res = query(
    //     deps.as_ref(),
    //     mock_env(),
    //     QueryMsg::GetUser {
    //         user_address: "geekybot".to_string(),
    //     },
    // )
    // .unwrap();
    // let value: UserResponse = from_binary(&res).unwrap();
    // // assert_eq!(Uint128::from(9_843_316u128), value.collateral);
    // assert_eq!(Uint128::from(149_259u128), value.total_fee_paid);
    // assert_eq!(Uint128::from(10_000_000u128), value.cumulative_deposits);

    // let res = query(
    //     deps.as_ref(),
    //     mock_env(),
    //     QueryMsg::GetUserPositions {
    //         user_address: "geekybot".to_string(),
    //         start_after: None,
    //         limit: None,
    //     },
    // )
    // .unwrap();
    // let value: Vec<PositionResponse> = from_binary(&res).unwrap();
    // // assert_eq!(PositionDirection::Short, value[0].direction);
    // // assert_eq!(Number128::new(-248836633317731), value[0].base_asset_amount);
    // // assert_eq!(Uint128::from(24882425u128), value[0].quote_asset_amount);
    // // assert_eq!(Uint128::zero(), value[0].entry_price);
    // // assert_eq!(Number128::zero(), value[0].entry_notional);
    // let res = query(
    //     deps.as_ref(),
    //     mock_env(),
    //     QueryMsg::GetMarketInfo { market_index: 1 },
    // )
    // .unwrap();
    // let value: MarketInfoResponse = from_binary(&res).unwrap();
    // assert_eq!(amm_base_asset_reserve, value.sqrt_k);
    // assert_eq!(
    //     0,
    //     value.base_asset_amount.i128()
    // );
    // assert_eq!(Uint128::from(149_259u128), value.total_fee);
    // assert_eq!(
    //     Uint128::from(149_259u128),
    //     value.total_fee_minus_distributions
    // );

    // let res = query(
    //     deps.as_ref(),
    //     mock_env(),
    //     QueryMsg::GetTradeHistory {
    //         start_after: None,
    //         limit: None,
    //     },
    // )
    // .unwrap();
    // let value: Vec<TradeHistoryResponse> = from_binary(&res).unwrap();
    // assert_eq!(PositionDirection::Short, value[1].direction);
    // assert_eq!(PositionDirection::Short, value[2].direction);
    // assert_eq!(PositionDirection::Long, value[3].direction);
    // assert_eq!(
    //     Uint128::new(497524761064414u128),
    //     value[1].base_asset_amount
    // );
    // assert_eq!(
    //     Uint128::new(248762375928202u128),
    //     value[2].base_asset_amount
    // );
    // assert_eq!(
    //     Uint128::new(497450503674885u128),
    //     value[3].base_asset_amount
    // );
}

pub fn calculate_trade_amount(amount_collateral: u128) -> StdResult<Uint128> {
    let trade_amount = Uint128::from(amount_collateral)
        .checked_mul(Uint128::from(5u128))?
        .checked_mul(Uint128::from(100000u128).checked_sub(Uint128::from(500u128))?)?
        .checked_div(Uint128::from(100000u128))?;

    Ok(trade_amount)
}

// pub fn calculate_trade_slippage(
//     direction: PositionDirection,
//     trade_amount: Uint128,
//     base: Number128,
//     quote: Uint128,
//     peg_multiplier: Uint128,
// ) -> StdResult<(Uint128, Uint128)> {
//     let oldPrice = quote
//         .checked_mul(Uint128::from(10_000_000_000u128))?
//         .checked_mul(peg_multiplier)?
//         .checked_div(Uint128::from(1000u128))?
//         .checked_div(Uint128::from(base.amount))?;
//     if !base.is_positive {
//         return Ok((oldPrice, Uint128::zero()));
//     }

// }
