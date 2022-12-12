use cosmwasm_std::{Decimal, StdError, StdResult, Uint128};

pub trait Converter128 {
    fn checked_div_decimal(&self, denominator: Decimal) -> StdResult<Self>
    where
        Self: Sized;
}

impl Converter128 for Uint128 {
    fn checked_div_decimal(&self, denominator: Decimal) -> StdResult<Uint128> {
        Decimal::one()
            .checked_div(denominator)
            .map_err(|err| StdError::generic_err(err.to_string()))
            .map(|coeff| self.clone() * coeff)
    }
}
