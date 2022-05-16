use cosmwasm_std::{StdError, OverflowError, DivideByZeroError, ConversionOverflowError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Clearing House history already initialized")]
    HistoryAlreadyInitialized,

    #[error("Only clearing house can record")]
    UnauthorizedClearingHouse,

    #[error("Not An Admin")]
    ConversionOverflowError(#[from] ConversionOverflowError),

    #[error("Math Error")]
    MathError,
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