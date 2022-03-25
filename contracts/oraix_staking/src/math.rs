use cosmwasm_std::{Decimal, Uint128};
use oraiswap::asset::Asset;

// use 10^9 for decimal fractional
const DECIMAL_FRACTIONAL: Uint128 = Uint128(1_000_000_000u128);

/// return a / b
pub fn decimal_division(a: Decimal, b: Decimal) -> Decimal {
    Decimal::from_ratio(DECIMAL_FRACTIONAL * a, b * DECIMAL_FRACTIONAL)
}

pub fn decimal_subtraction(a: Decimal, b: Decimal) -> Decimal {
    Decimal::from_ratio(
        Asset::checked_sub(DECIMAL_FRACTIONAL * a, DECIMAL_FRACTIONAL * b).unwrap(),
        DECIMAL_FRACTIONAL,
    )
}
