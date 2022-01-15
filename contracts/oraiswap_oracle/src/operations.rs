use std::ops::Mul;

use cosmwasm_std::{Coin, Decimal, Deps, StdError, StdResult};
use oraiswap::asset::{DECIMAL_FRACTION, ORAI_DENOM};

use crate::state::EXCHANGE_RATES;

// ComputeInternalSwap returns the amount of asked DecCoin should be returned for a given offerCoin at the effective
// exchange rate registered with the oracle.
// Different from ComputeSwap, ComputeInternalSwap does not charge a spread as its use is system internal.
// retAmount := offerCoin.Amount.Mul(exchange_rate)
pub fn compute_swap(deps: Deps, offer_coin: Coin, ask_denom: &str) -> StdResult<(Coin, Decimal)> {
    if offer_coin.denom.eq(ask_denom) {
        return Err(StdError::generic_err("Oraiswap Oracle: recursive swap"));
    }

    // Swap offer coin to base denom for simplicity of swap process
    let base_offer_coin = compute_internal_swap(deps, offer_coin, ORAI_DENOM)?;

    // Get swap amount based on the oracle price
    let ret_coin = compute_internal_swap(deps, base_offer_coin, ask_denom)?;

    // Apply only tobin tax without constant product spread
}

pub fn get_orai_exchange_rate(deps: Deps, denom: &str) -> StdResult<Decimal> {
    if denom == ORAI_DENOM {
        return Ok(Decimal::one());
    }

    EXCHANGE_RATES.load(deps.storage, denom.as_bytes())
}

// Different from ComputeSwap, ComputeInternalSwap does not charge a spread as its use is system internal.
fn compute_internal_swap(deps: Deps, offer_coin: Coin, ask_denom: &str) -> StdResult<Coin> {
    if offer_coin.denom.eq(ask_denom) {
        return Ok(offer_coin);
    }

    let offer_rate = get_orai_exchange_rate(deps, &offer_coin.denom)?;
    let ask_rate = get_orai_exchange_rate(deps, ask_denom)?;

    let ret_amount = offer_coin.amount.multiply_ratio(
        ask_rate.mul(DECIMAL_FRACTION),
        offer_rate.mul(DECIMAL_FRACTION),
    );

    Ok(Coin {
        amount: ret_amount,
        denom: ask_denom.to_string(),
    })
}
