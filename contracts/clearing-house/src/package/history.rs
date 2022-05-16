use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::states::history::{CurveRecord, DepositRecord, FundingPaymentRecord, FundingRateRecord, LiquidationRecord, TradeRecord};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HistoryExecuteMsg {
    UpdateAdmin {
        new_admin: String,
    },
    UpdateClearingHouse {
        new_house: String,
    },
    RecordCurve {
        c: CurveRecord
    },
    RecordDeposit {
        d: DepositRecord
    },
    RecordFundingPayment {
        f: FundingPaymentRecord
    },
    RecordFundingRate {
        f: FundingRateRecord
    },
    RecordLiquidation {
        l: LiquidationRecord
    },
    RecordTrade {
        t: TradeRecord
    },
    RecordFundingPaymentsMultiple {
        vecf: Vec<FundingPaymentRecord>
    },
}
