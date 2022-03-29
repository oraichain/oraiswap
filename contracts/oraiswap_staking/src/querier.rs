use crate::math::{decimal_division, decimal_subtraction};
use cosmwasm_std::{
    to_binary, Decimal, Deps, HumanAddr, QueryRequest, StdResult, Uint128, WasmQuery,
};
use oraiswap::{
    asset::AssetInfo,
    asset::PairInfo,
    oracle::OracleContract,
    pair::PoolResponse,
    pair::QueryMsg as PairQueryMsg,
    querier::{query_pair_info, query_token_info},
};

pub fn compute_premium_rate(
    deps: Deps,
    oracle_addr: HumanAddr,
    factory_addr: HumanAddr,
    asset_token: HumanAddr,
    base_denom: String,
) -> StdResult<(Decimal, bool)> {
    let pair_info: PairInfo = query_pair_info(
        &deps.querier,
        factory_addr,
        &[
            AssetInfo::NativeToken {
                denom: base_denom.clone(),
            },
            AssetInfo::Token {
                contract_addr: asset_token.clone(),
            },
        ],
    )?;

    let pool: PoolResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pair_info.contract_addr,
        msg: to_binary(&PairQueryMsg::Pool {})?,
    }))?;

    let oraiswap_price: Decimal = if pool.assets[0].is_native_token() {
        if pool.assets[1].amount.is_zero() {
            Decimal::from_ratio(pool.assets[0].amount, Uint128::from(1u128))
        } else {
            Decimal::from_ratio(pool.assets[0].amount, pool.assets[1].amount)
        }
    } else if pool.assets[0].amount.is_zero() {
        Decimal::from_ratio(pool.assets[1].amount, Uint128::from(1u128))
    } else {
        Decimal::from_ratio(pool.assets[1].amount, pool.assets[0].amount)
    };

    // get denom from token contract
    let asset_token_info = query_token_info(&deps.querier, asset_token)?;
    let oracle_contract = OracleContract(oracle_addr);

    // oracle price in exchange rate format
    let oracle_price: Decimal = oracle_contract
        .query_exchange_rate(&deps.querier, asset_token_info.symbol, base_denom)?
        .item
        .exchange_rate;

    if oracle_price.is_zero() {
        Ok((Decimal::zero(), true))
    } else if oraiswap_price > oracle_price {
        // adjust (oraiswap_price - oracle_price) /  oracle_price => percentage gain of oraiswap_price compare to oracle_price
        Ok((
            decimal_division(
                decimal_subtraction(oraiswap_price, oracle_price)?,
                oracle_price,
            ),
            false,
        ))
    } else {
        Ok((Decimal::zero(), false))
    }
}
