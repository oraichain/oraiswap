use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Decimal, StdError, StdResult, Uint128};
use cw20::Cw20Coin;
use std::convert::TryInto;

#[cw_serde]
pub enum Amount {
    Native(Coin),
    // FIXME? USe Cw20CoinVerified, and validate cw20 addresses
    Cw20(Cw20Coin),
}

impl Amount {
    // TODO: write test for this
    pub fn from_parts(denom: String, amount: Uint128) -> Self {
        if denom.starts_with("cw20:") {
            let address = denom.get(5..).unwrap().into();
            Amount::Cw20(Cw20Coin { address, amount })
        } else {
            Amount::Native(Coin { denom, amount })
        }
    }

    pub fn cw20(amount: u128, addr: &str) -> Self {
        Amount::Cw20(Cw20Coin {
            address: addr.into(),
            amount: Uint128::new(amount),
        })
    }

    pub fn native(amount: u128, denom: &str) -> Self {
        Amount::Native(Coin {
            denom: denom.to_string(),
            amount: Uint128::new(amount),
        })
    }
}

impl Amount {
    pub fn denom(&self) -> String {
        match self {
            Amount::Native(c) => c.denom.clone(),
            Amount::Cw20(c) => format!("cw20:{}", c.address.as_str()),
        }
    }

    // this returns original cw20 address if it's cw20
    pub fn raw_denom(&self) -> String {
        match self {
            Amount::Native(c) => c.denom.clone(),
            Amount::Cw20(c) => c.address.to_string(),
        }
    }

    pub fn amount(&self) -> Uint128 {
        match self {
            Amount::Native(c) => c.amount,
            Amount::Cw20(c) => c.amount,
        }
    }

    /// convert the amount into u64
    pub fn u64_amount(&self) -> Result<u64, StdError> {
        Ok(self
            .amount()
            .u128()
            .try_into()
            .map_err(|_| StdError::generic_err("error casting to u64 from u128".to_string()))?)
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Amount::Native(c) => c.amount.is_zero(),
            Amount::Cw20(c) => c.amount.is_zero(),
        }
    }
}

impl Amount {
    fn mul_ratio_decimal(&self, ratio: Decimal) -> StdResult<Uint128> {
        Decimal::one()
            .checked_mul(ratio)
            .map_err(|err| StdError::generic_err(err.to_string()))
            .map(|coeff| self.amount() * coeff)
    }

    pub fn convert_remote_to_cw20(
        &self,
        remote_decimals: u8,
        cw20_decimals: u8,
    ) -> StdResult<Uint128> {
        self.mul_ratio_decimal(Decimal::from_ratio(
            10u128.pow(cw20_decimals as u32),
            10u128.pow(remote_decimals as u32),
        ))
    }

    pub fn convert_cw20_to_remote(
        &self,
        remote_decimals: u8,
        cw20_decimals: u8,
    ) -> StdResult<Uint128> {
        self.mul_ratio_decimal(Decimal::from_ratio(
            10u128.pow(remote_decimals as u32),
            10u128.pow(cw20_decimals as u32),
        ))
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::coin;

    use super::*;

    #[test]
    pub fn test_div_ratio_decimal() {
        let amount = Amount::Native(coin(10u128, "orai"));
        let new_amount = amount.convert_cw20_to_remote(18, 6).unwrap();
        assert_eq!(new_amount, Uint128::from(10000000000000u128));
        let new_amount = amount.convert_remote_to_cw20(18, 6).unwrap();
        assert_eq!(new_amount, Uint128::from(0u128))
    }
}
