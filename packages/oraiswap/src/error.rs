use cosmwasm_std::{StdError, Uint128};
use std::fmt;
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum OverflowOperation {
    Add,
    Sub,
    Mul,
    Pow,
    Shr,
    Shl,
}

impl fmt::Display for OverflowOperation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
#[error("Cannot {operation} with {operand1} and {operand2}")]
pub struct OverflowError {
    pub operation: OverflowOperation,
    pub operand1: String,
    pub operand2: String,
}

impl OverflowError {
    pub fn new(
        operation: OverflowOperation,
        operand1: impl ToString,
        operand2: impl ToString,
    ) -> Self {
        Self {
            operation,
            operand1: operand1.to_string(),
            operand2: operand2.to_string(),
        }
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid zero amount")]
    InvalidZeroAmount {},

    #[error("Max spread assertion")]
    MaxSpreadAssertion {},

    #[error("Max slippage assertion")]
    MaxSlippageAssertion {},

    #[error("Slippage_tolerance cannot bigger than 1")]
    InvalidExceedOneSlippage {},

    #[error("Withdraw amount is too small compared to the total share")]
    InvalidZeroRatio {},

    #[error("Asset mismatch")]
    AssetMismatch {},

    #[error("Too small offer amount")]
    TooSmallOfferAmount {},

    #[error("Pair already exists")]
    PairExisted {},

    #[error("Pair was already registered")]
    PairRegistered {},

    #[error(
        "Assertion failed; minimum receive amount: {minium_receive}, swap amount: {swap_amount}"
    )]
    SwapAssertionFailure {
        minium_receive: Uint128,
        swap_amount: Uint128,
    },

    #[error("must provide operations")]
    NoSwapOperation {},
}
