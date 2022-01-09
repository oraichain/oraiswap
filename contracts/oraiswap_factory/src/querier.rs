use cosmwasm_std::{Binary, Deps, HumanAddr, QueryRequest, StdResult, WasmQuery};
use oraiswap::asset::PairInfoRaw;

pub fn query_liquidity_token(deps: Deps, contract_addr: HumanAddr) -> StdResult<HumanAddr> {
    // load pair_info form the pair contract
    let pair_info: PairInfoRaw = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Raw {
        contract_addr,
        key: Binary::from("\u{0}\u{9}pair_info".as_bytes()),
    }))?;

    deps.api.human_address(&pair_info.liquidity_token)
}
