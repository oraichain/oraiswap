use std::convert::TryInto;

use cosmwasm_std::{Uint128, Uint256};

pub trait Converter {
    fn into_u128(&self) -> Uint128
    where
        Self: Sized;
}

impl Converter for Uint256 {
    fn into_u128(&self) -> Uint128 {
        u128::from_le_bytes(self.to_le_bytes()[0..16].try_into().unwrap()).into()
    }
}
