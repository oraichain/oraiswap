use std::ops::{Add, Div, Mul, Sub};

use cosmwasm_std::{Coin, Decimal, Deps, StdError, StdResult};
use oraiswap::{
    asset::{DECIMAL_FRACTION, ORAI_DENOM},
    oracle::ContractInfo,
    Decimal256, Uint256,
};

use crate::state::{CONTRACT_INFO, EXCHANGE_RATES, NATIVE_ORAI_POOL_DELTA, TOBIN_TAXES};

// ComputeInternalSwap returns the amount of asked DecCoin should be returned for a given offerCoin at the effective
// exchange rate registered with the oracle.
// Different from ComputeSwap, ComputeInternalSwap does not charge a spread as its use is system internal.
// retAmount := offerCoin.Amount.Mul(exchange_rate)
// pub fn compute_swap(deps: Deps, offer_coin: Coin, ask_denom: &str) -> StdResult<(Coin, Decimal)> {
//     if offer_coin.denom.eq(ask_denom) {
//         return Err(StdError::generic_err("Oraiswap Oracle: recursive swap"));
//     }

//     // Swap offer coin to base denom for simplicity of swap process
//     let base_offer_coin = compute_internal_swap(deps, offer_coin, ORAI_DENOM)?;

//     // Get swap amount based on the oracle price
//     let ret_coin = compute_internal_swap(deps, base_offer_coin, ask_denom)?;

//     // Apply only tobin tax without constant product spread
//     if offer_coin.denom.ne(ORAI_DENOM) && ask_denom != ORAI_DENOM {
//         let offer_tobin_tax = TOBIN_TAXES.load(deps.storage, offer_coin.denom.as_bytes())?;
//         let ask_tobin_tax = TOBIN_TAXES.load(deps.storage, ask_denom.as_bytes())?;

//         // Apply highest tobin tax for the denoms in the swap operation
//         let spread = offer_tobin_tax.max(ask_tobin_tax);

//         return Ok((ret_coin, spread));
//     }

//     // TODO: using Decimal256 as much as possible
//     let ContractInfo {
//         base_pool,
//         min_stability_spread,
//         ..
//     } = CONTRACT_INFO.load(deps.storage)?;
//     // constant-product, which by construction is square of base(equilibrium) pool
//     // NativeOraiPool := BasePool + delta
//     // OW20OraiPool := (BasePool * BasePool) / NativeOraiPool
//     let base_pool: Uint256 = base_pool.into();
//     let cp = base_pool.mul(base_pool);
//     let native_orai_pool_delta = NATIVE_ORAI_POOL_DELTA.load(deps.storage)?;
//     let native_orai_pool = base_pool.add(native_orai_pool_delta);
//     let ow20_orai_pool = cp.div(native_orai_pool);

//     let offer_pool; // quote denom(orai) unit
//     let ask_pool; // base denom(orai) unit
//     if offer_coin.denom.ne(ORAI_DENOM) {
//         // Native Orai -> OW20 Orai swap
//         offer_pool = native_orai_pool;
//         ask_pool = ow20_orai_pool;
//     } else {
//         // OW20 Orai -> Native Orai swap
//         offer_pool = ow20_orai_pool;
//         ask_pool = native_orai_pool;
//     }

//     // Get cp(constant-product) based swap amount
//     // base_ask_amount = ask_pool - cp / (offerPool + base_offer_amount)
//     // base_ask_amount is base denom(orai) unit
//     let base_ask_amount = ask_pool.sub(cp.div(offer_pool.add(base_offer_coin.amount)));

//     // Both baseOffer and baseAsk are usdr units, so spread can be calculated by
//     // spread = (baseOfferAmt - baseAskAmt) / baseOfferAmt
//     let base_offer_amount = base_offer_coin.amount;
//     let spread = Decimal::from_ratio(base_offer_amount - base_ask_amount, base_offer_amount)
//         .min(min_stability_spread);
// }

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
