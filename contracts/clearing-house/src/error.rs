use cosmwasm_std::{StdError, OverflowError, DivideByZeroError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("Unauthorized")]
    Unauthorized {},
    #[error("Clearing house cannot call collateral contract")]
    InvalidCollateralAccountAuthority,
    #[error("Clearing house cannot call insurance contract")]
    InvalidInsuranceAccountAuthority,
    #[error("The User does not exist")]
    UserDoesNotExist,
    #[error("The state is not saved prior to this action")]
    ObjectDoesNotExist,
    #[error("Insufficient deposit")]
    InsufficientDeposit,
    #[error("Insufficient collateral")]
    InsufficientCollateral,
    #[error("Sufficient collateral")]
    SufficientCollateral,
    #[error("Max number of positions taken")]
    MaxNumberOfPositions,
    #[error("Admin Controls Prices Disabled")]
    AdminControlsPricesDisabled,
    #[error("Market Index Not Initialized")]
    MarketIndexNotInitialized,
    #[error("Market Index Already Initialized")]
    MarketIndexAlreadyInitialized,
    #[error("User Account And User Positions Account Mismatch")]
    UserAccountAndUserPositionsAccountMismatch,
    #[error("User Has No Position In Market")]
    UserHasNoPositionInMarket,
    #[error("Invalid Initial Peg")]
    InvalidInitialPeg,
    #[error("AMM repeg already configured with amt given")]
    InvalidRepegRedundant,
    #[error("AMM repeg incorrect repeg direction")]
    InvalidRepegDirection,
    #[error("AMM repeg out of bounds pnl")]
    InvalidRepegProfitability,
    #[error("Slippage Outside Limit Price")]
    SlippageOutsideLimit,
    #[error("Trade Size Too Small")]
    TradeSizeTooSmall,
    #[error("Price change too large when updating K")]
    InvalidUpdateK,
    #[error("Admin tried to withdraw amount larger than fees collected")]
    AdminWithdrawTooLarge,
    #[error("Math Error")]
    MathError,
    #[error("Conversion to u128/u64 failed with an overflow or underflow")]
    BnConversionError,
    #[error("Clock unavailable")]
    ClockUnavailable,
    #[error("Unable To Load Oracles")]
    UnableToLoadOracle,
    #[error("Oracle/Mark Spread Too Large")]
    OracleMarkSpreadLimit,
    #[error("Exchange is paused")]
    ExchangePaused,
    #[error("Invalid whitelist token")]
    InvalidWhitelistToken,
    #[error("Whitelist token not found")]
    WhitelistTokenNotFound,
    #[error("Invalid discount token")]
    InvalidDiscountToken,
    #[error("Discount token not found")]
    DiscountTokenNotFound,
    #[error("Invalid referrer")]
    InvalidReferrer,
    #[error("Referrer not found")]
    ReferrerNotFound,
    #[error("InvalidOracle")]
    InvalidOracle,
    #[error("OracleNotFound")]
    OracleNotFound,
    #[error("Liquidations Blocked By Oracle")]
    LiquidationsBlockedByOracle,
    #[error("Can not deposit more than max deposit")]
    UserMaxDeposit,
    #[error("Can not delete user that still has collateral")]
    CantDeleteUserWithCollateral,
    #[error("AMM funding out of bounds pnl")]
    InvalidFundingProfitability,
    #[error("Casting Failure")]
    CastingFailure,
    #[error("Oracle offset limit price below zero")]
    InvalidOracleOffset,
    #[error("Could not find oracle to calculate oracle offset limit price")]
    OracleNotFoundToOffset,
    #[error("Invalid Order")]
    InvalidOrder,
    #[error("User has no order")]
    UserHasNoOrder,
    #[error("Order Amount Too Small")]
    OrderAmountTooSmall,
    #[error("Max number of orders taken")]
    MaxNumberOfOrders,
    #[error("Order does not exist")]
    OrderDoesNotExist,
    #[error("Order not open")]
    OrderNotOpen,
    #[error("CouldNotFillOrder")]
    CouldNotFillOrder,
    #[error("Reduce only order increased risk")]
    ReduceOnlyOrderIncreasedRisk,
    #[error("Order state already initialized")]
    OrderStateAlreadyInitialized,
    #[error("Unable to load AccountLoader")]
    UnableToLoadAccountLoader,
    #[error("Trade Size Too Large")]
    TradeSizeTooLarge,
    #[error("Unable to write to remaining account")]
    UnableToWriteToRemainingAccount,
    #[error("User cant refer themselves")]
    UserCantReferThemselves,
    #[error("Did not receive expected referrer")]
    DidNotReceiveExpectedReferrer,
    #[error("Could not deserialize referrer")]
    CouldNotDeserializeReferrer,
    #[error("Market order must be in place and fill")]
    MarketOrderMustBeInPlaceAndFill,
    #[error("User Order Id Already In Use")]
    UserOrderIdAlreadyInUse,
    #[error("No positions liquidatable")]
    NoPositionsLiquidatable,
    #[error("Invalid Margin Ratio")]
    InvalidMarginRatio,
    #[error("Cant Cancel Post Only Order")]
    CantCancelPostOnlyOrder,
    #[error("CantExpireOrders")]
    CantExpireOrders,
    #[error("Helpers Error")]
    HelpersError,
    #[error("Math Error 1")]
    MathError1,
    #[error("Math Error 2")]
    MathError2,
    #[error("Math Error 3")]
    MathError3,
    #[error("Math Error 4")]
    MathError4,
    #[error("Math Error 5")]
    MathError5,
    #[error("Math Error 6")]
    MathError6,
    #[error("Math Error 7")]
    MathError7,
    #[error("Math Error 8")]
    MathError8,
    #[error("Math Error 9")]
    MathError9,
    #[error("Math Error 10")]
    MathError10,
    #[error("Math Error 11")]
    MathError11,
}


impl From<OverflowError> for ContractError {
    fn from(o: OverflowError) -> Self {
        StdError::from(o).into()
    }
}

impl From<DivideByZeroError> for ContractError {
    fn from(o: DivideByZeroError) -> Self {
        StdError::from(o).into()
    }
}