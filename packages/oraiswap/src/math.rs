use std::{cmp::min, convert::TryInto};

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

impl Truncate for Decimal {
    fn to_string_round(&self, digits: usize) -> String {
        let parts = self.to_string();
        let mut parts_iter = parts.split('.');

        let mut whole_part = parts_iter.next().unwrap().to_string(); // split always returns at least one element

        if let Some(fractional_part) = parts_iter.next() {
            whole_part.push('.');
            whole_part.push_str(&fractional_part[..min(digits, fractional_part.len())]);
        }

        whole_part
    }
}
