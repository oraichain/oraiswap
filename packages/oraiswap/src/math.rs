use std::convert::TryInto;

use cosmwasm_std::{Decimal, StdError, StdResult, Uint128, Uint256};

pub trait Converter128 {
    fn checked_div_decimal(&self, denominator: Decimal) -> StdResult<Self>
    where
        Self: Sized;
}

pub trait Converter256 {
    fn into_u128(&self) -> Uint128;
}

pub trait Truncate {
    fn to_string_round(&self, digits: usize) -> String;
}

impl Converter256 for Uint256 {
    fn into_u128(&self) -> Uint128 {
        u128::from_le_bytes(self.to_le_bytes()[0..16].try_into().unwrap()).into()
    }
}

impl Converter128 for Uint128 {
    fn checked_div_decimal(&self, denominator: Decimal) -> StdResult<Uint128> {
        Decimal::one()
            .checked_div(denominator)
            .map_err(|err| StdError::generic_err(err.to_string()))
            .map(|coeff| self.clone() * coeff)
    }
}
