use cosmwasm_std::{
    Binary, CanonicalAddr, HumanAddr, QuerierWrapper, QueryRequest, StdResult, WasmQuery,
};
use cosmwasm_storage::to_length_prefixed;
use oraiswap::asset::PairInfoRaw;

// need to_length_prefixed to make it compatible with singleton legacy
pub fn query_liquidity_token(
    querier: QuerierWrapper,
    contract_addr: HumanAddr,
) -> StdResult<CanonicalAddr> {
    // load pair_info form the pair contract
    let pair_info: PairInfoRaw = querier.query(&QueryRequest::Wasm(WasmQuery::Raw {
        contract_addr,
        key: Binary::from(to_length_prefixed(b"pair_info")),
    }))?;

    Ok(pair_info.liquidity_token)
}
