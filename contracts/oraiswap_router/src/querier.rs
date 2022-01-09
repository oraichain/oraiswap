use cosmwasm_std::{Decimal, QuerierWrapper, StdResult, Uint128};
use orai_cosmwasm::OraiQuerier;

static DECIMAL_FRACTION: Uint128 = Uint128::new(1_000_000_000_000_000_000u128);

pub fn compute_tax(querier: &QuerierWrapper, amount: Uint128, denom: String) -> StdResult<Uint128> {
    if denom == "uluna" {
        return Ok(Uint128::zero());
    }

    let orai_querier = OraiQuerier::new(querier);
    let tax_rate: Decimal = (orai_querier.query_tax_rate()?).rate;
    let tax_cap: Uint128 = (orai_querier.query_tax_cap(denom)?).cap;
    Ok(std::cmp::min(
        amount.checked_sub(amount.multiply_ratio(
            DECIMAL_FRACTION,
            DECIMAL_FRACTION * tax_rate + DECIMAL_FRACTION,
        ))?,
        tax_cap,
    ))
}
