use crate::asset::{Asset, AssetInfo, PairInfo};
use crate::factory::{ConfigResponse, QueryMsg as FactoryQueryMsg};
use crate::pair::{QueryMsg as PairQueryMsg, ReverseSimulationResponse, SimulationResponse};

use cosmwasm_std::{
    to_binary, HumanAddr, QuerierWrapper, QueryRequest, StdResult, Uint128, WasmQuery,
};
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg, TokenInfoResponse};

pub fn query_token_balance(
    querier: &QuerierWrapper,
    contract_addr: HumanAddr,
    account_addr: HumanAddr,
) -> StdResult<Uint128> {
    let res: Cw20BalanceResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr,
        msg: to_binary(&Cw20QueryMsg::Balance {
            address: account_addr,
        })?,
    }))?;

    // load balance form the token contract
    Ok(res.balance)
}

pub fn query_token_info(
    querier: &QuerierWrapper,
    contract_addr: HumanAddr,
) -> StdResult<TokenInfoResponse> {
    // load price form the oracle
    querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr,
        msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
    }))
}

pub fn query_supply(querier: &QuerierWrapper, contract_addr: HumanAddr) -> StdResult<Uint128> {
    // load price form the oracle
    query_token_info(querier, contract_addr).map(|token_info| token_info.total_supply)
}

pub fn query_pair_info(
    querier: &QuerierWrapper,
    factory_addr: HumanAddr,
    asset_infos: &[AssetInfo; 2],
) -> StdResult<PairInfo> {
    querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: factory_addr,
        msg: to_binary(&FactoryQueryMsg::Pair {
            asset_infos: asset_infos.clone(),
        })?,
    }))
}

pub fn query_pair_config(
    querier: &QuerierWrapper,
    factory_addr: HumanAddr,
) -> StdResult<ConfigResponse> {
    querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: factory_addr,
        msg: to_binary(&FactoryQueryMsg::Config {})?,
    }))
}

pub fn simulate(
    querier: &QuerierWrapper,
    pair_addr: HumanAddr,
    offer_asset: &Asset,
) -> StdResult<SimulationResponse> {
    querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pair_addr,
        msg: to_binary(&PairQueryMsg::Simulation {
            offer_asset: offer_asset.clone(),
        })?,
    }))
}

pub fn reverse_simulate(
    querier: &QuerierWrapper,
    pair_addr: HumanAddr,
    ask_asset: &Asset,
) -> StdResult<ReverseSimulationResponse> {
    querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: pair_addr,
        msg: to_binary(&PairQueryMsg::ReverseSimulation {
            ask_asset: ask_asset.clone(),
        })?,
    }))
}
