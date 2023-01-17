use std::num::TryFromIntError;
use std::string::FromUtf8Error;
use thiserror::Error;

use cosmwasm_std::StdError;
use cw_controllers::AdminError;
use cw_utils::PaymentError;

/// Never is a placeholder to ensure we don't return any errors
#[derive(Error, Debug)]
pub enum Never {}

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Payment(#[from] PaymentError),

    #[error("{0}")]
    Admin(#[from] AdminError),

    #[error("Channel doesn't exist: {id}")]
    NoSuchChannel { id: String },

    #[error("Didn't send any funds")]
    NoFunds {},

    #[error("Amount larger than 2**64, not supported by ics20 packets")]
    AmountOverflow {},

    #[error("Only supports channel with ibc version ics20-1, got {version}")]
    InvalidIbcVersion { version: String },

    #[error("Only supports unordered channel")]
    OnlyOrderedChannel {},

    #[error("Insufficient funds to redeem voucher on channel")]
    InsufficientFunds {},

    #[error("Only accepts tokens that originate on this chain or native tokens of remote chain, not other types")]
    NoForeignTokens {},

    #[error("Parsed port from denom ({port}) doesn't match packet")]
    FromOtherPort { port: String },

    #[error("Parsed channel from denom ({channel}) doesn't match packet")]
    FromOtherChannel { channel: String },

    #[error("Cannot migrate from different contract type: {previous_contract}")]
    CannotMigrate { previous_contract: String },

    #[error("Cannot migrate from unsupported version: {previous_version}")]
    CannotMigrateVersion { previous_version: String },

    #[error("Got a submessage reply with unknown id: {id}")]
    UnknownReplyId { id: u64 },

    #[error("You cannot lower the gas limit for a contract on the allow list")]
    CannotLowerGas,

    #[error("Only the governance contract can do this")]
    Unauthorized,

    #[error("You can only send tokens that have been explicitly allowed by governance")]
    NotOnAllowList,

    #[error("You can only send native tokens that has a map to the corresponding asset info")]
    NotOnMappingList,

    #[error("The contract address you are sending native tokens to is already revoked")]
    CustomContractRevoked,

    #[error("Could not find the mapping pair")]
    MappingPairNotFound,
}

impl From<FromUtf8Error> for ContractError {
    fn from(_: FromUtf8Error) -> Self {
        ContractError::Std(StdError::invalid_utf8("parsing denom key"))
    }
}

impl From<TryFromIntError> for ContractError {
    fn from(_: TryFromIntError) -> Self {
        ContractError::AmountOverflow {}
    }
}
