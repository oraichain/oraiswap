use cosmwasm_std::{Binary, Deps, HumanAddr, QueryRequest, StdResult, WasmQuery};
use cosmwasm_storage::to_length_prefixed;
use oraiswap::asset::PairInfoRaw;

// need to_length_prefixed to make it compatible with singleton legacy
pub fn query_liquidity_token(deps: Deps, contract_addr: HumanAddr) -> StdResult<HumanAddr> {
    // load pair_info form the pair contract
    let pair_info: PairInfoRaw = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Raw {
        contract_addr,
        key: Binary::from(to_length_prefixed(b"pair_info")),
    }))?;

    deps.api.human_address(&pair_info.liquidity_token)
}
