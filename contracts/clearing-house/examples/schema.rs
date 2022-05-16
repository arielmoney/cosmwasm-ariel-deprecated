use std::env::current_dir;
use std::fs::create_dir_all;
use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

// use clearing_house::states::{market, state, user};
use clearing_house::package::execute::{InstantiateMsg, ExecuteMsg};
use clearing_house::package::queries::QueryMsg;
// use clearing_house::states::history::{CurveRecord, DepositRecord, FundingPaymentRecord, FundingRateRecord, LiquidationRecord, TradeRecord};
// use clearing_house::package::response::{UserResponse, UserPositionResponse, AdminResponse, IsExchangePausedResponse, IsFundingPausedResponse, AdminControlsPricesResponse, VaultsResponse, MarginRatioResponse, PartialLiquidationClosePercentageResponse, PartialLiquidationPenaltyPercentageResponse, FullLiquidationPenaltyPercentageResponse, PartialLiquidatorSharePercentageResponse, FullLiquidatorSharePercentageResponse, MaxDepositLimitResponse, FeeStructureResponse, CurveHistoryResponse, DepositHistoryResponse, FundingPaymentHistoryResponse, FundingRateHistoryResponse, LiquidationHistoryResponse, TradeHistoryResponse, MarketInfoResponse, LengthResponse};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    // messages schema export
    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);

    // state schema export
    // export_schema(&schema_for!(CurveRecord), &out_dir);
    // export_schema(&schema_for!(DepositRecord), &out_dir);
    // export_schema(&schema_for!(FundingPaymentRecord), &out_dir);
    // export_schema(&schema_for!(FundingRateRecord), &out_dir);
    // export_schema(&schema_for!(LiquidationRecord), &out_dir);
    // export_schema(&schema_for!(market::Market), &out_dir);
    // export_schema(&schema_for!(market::Amm), &out_dir);
    // export_schema(&schema_for!(state::State), &out_dir);
    // export_schema(&schema_for!(TradeRecord), &out_dir);
    // export_schema(&schema_for!(user::User), &out_dir);
    // export_schema(&schema_for!(user::Position), &out_dir);
    
    // response schema exports
    // export_schema(&schema_for!(UserResponse), &out_dir);
    // export_schema(&schema_for!(UserPositionResponse), &out_dir);
    // export_schema(&schema_for!(AdminResponse), &out_dir);
    // export_schema(&schema_for!(IsExchangePausedResponse), &out_dir);
    // export_schema(&schema_for!(IsFundingPausedResponse), &out_dir);
    // export_schema(&schema_for!(AdminControlsPricesResponse), &out_dir);
    // export_schema(&schema_for!(VaultsResponse), &out_dir);
    // export_schema(&schema_for!(MarginRatioResponse), &out_dir);
    // export_schema(
    //     &schema_for!(PartialLiquidationClosePercentageResponse),
    //     &out_dir,
    // );
    // export_schema(
    //     &schema_for!(PartialLiquidationPenaltyPercentageResponse),
    //     &out_dir,
    // );
    // export_schema(
    //     &schema_for!(FullLiquidationPenaltyPercentageResponse),
    //     &out_dir,
    // );
    // export_schema(
    //     &schema_for!(PartialLiquidatorSharePercentageResponse),
    //     &out_dir,
    // );
    // export_schema(
    //     &schema_for!(FullLiquidatorSharePercentageResponse),
    //     &out_dir,
    // );
    // export_schema(&schema_for!(MaxDepositLimitResponse), &out_dir);
    // export_schema(&schema_for!(FeeStructureResponse), &out_dir);
    // export_schema(&schema_for!(CurveHistoryResponse), &out_dir);
    // export_schema(&schema_for!(DepositHistoryResponse), &out_dir);
    // export_schema(&schema_for!(FundingPaymentHistoryResponse), &out_dir);
    // export_schema(&schema_for!(FundingRateHistoryResponse), &out_dir);
    // export_schema(&schema_for!(LiquidationHistoryResponse), &out_dir);
    // export_schema(&schema_for!(LengthResponse), &out_dir);
    // export_schema(&schema_for!(TradeHistoryResponse), &out_dir);
    // export_schema(&schema_for!(MarketInfoResponse), &out_dir);
}