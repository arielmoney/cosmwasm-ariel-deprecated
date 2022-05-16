use cosmwasm_std::{OverflowError, StdError};
use cw_controllers::AdminError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Admin(#[from] AdminError),

    #[error("Admin only")]
    UnauthorizedAdmin {},

    #[error("Clearing house only")]
    UnauthorizedClearingHouse {},

    #[error("Math error")]
    MathError {},

    #[error("Invalid asset")]
    InvalidIncomingAsset {},

    #[error("Insufficient funds")]
    InsufficientFunds {},
}

impl From<OverflowError> for ContractError {
    fn from(o: OverflowError) -> Self {
        StdError::from(o).into()
    }
}
