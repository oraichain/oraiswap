use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("token_id already claimed")]
    Claimed {},

    #[error("Cannot set approval that is already expired")]
    Expired {},

    #[error("Invalid argument: {reason}")]
    InvalidArgument { reason: String },

    #[error("Token not found")]
    TokenNotFound {},
    // #[panic_msg = "Min royalty `{}` must be less or equal to max royalty `{}`"]
    // MaxRoyaltyLessThanMinRoyalty { min_royalty: Fraction, max_royalty: Fraction },
    // #[panic_msg = "Royalty `{}` of `{}` is less than min"]
    // RoyaltyMinThanAllowed { royalty: Fraction, gate_id: String },
    // #[panic_msg = "Royalty `{}` of `{}` is greater than max"]
    // RoyaltyMaxThanAllowed { royalty: Fraction, gate_id: String },
    // #[panic_msg = "Royalty `{}` is too large for the given NFT fee `{}`"]
    // RoyaltyTooLarge { royalty: Fraction, mintgate_fee: Fraction },
    // #[panic_msg = "Gate ID `{}` already exists"]
}
