use crate::asset::{Asset, AssetInfo, PairInfo};
use crate::factory::{ConfigResponse, QueryMsg as FactoryQueryMsg};
use crate::pair::{
    PairResponse, QueryMsg as PairQueryMsg, ReverseSimulationResponse, SimulationResponse,
};

use cosmwasm_std::{Addr, QuerierWrapper, StdResult, Uint128};
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg, TokenInfoResponse};

pub fn query_token_balance(
    querier: &QuerierWrapper,
    contract_addr: Addr,
    account_addr: Addr,
) -> StdResult<Uint128> {
    let res: Cw20BalanceResponse = querier.query_wasm_smart(
        contract_addr,
        &Cw20QueryMsg::Balance {
            address: account_addr.to_string(),
        },
    )?;

    // load balance form the token contract
    Ok(res.balance)
}

pub fn query_token_info(
    querier: &QuerierWrapper,
    contract_addr: Addr,
) -> StdResult<TokenInfoResponse> {
    // load price form the oracle
    querier.query_wasm_smart(contract_addr, &Cw20QueryMsg::TokenInfo {})
}

pub fn query_supply(querier: &QuerierWrapper, contract_addr: Addr) -> StdResult<Uint128> {
    // load price form the oracle
    query_token_info(querier, contract_addr).map(|token_info| token_info.total_supply)
}

pub fn query_pair_info(
    querier: &QuerierWrapper,
    factory_addr: Addr,
    asset_infos: &[AssetInfo; 2],
) -> StdResult<PairInfo> {
    querier.query_wasm_smart(
        factory_addr,
        &FactoryQueryMsg::Pair {
            asset_infos: asset_infos.clone(),
        },
    )
}

pub fn query_pair_config(
    querier: &QuerierWrapper,
    factory_addr: Addr,
) -> StdResult<ConfigResponse> {
    querier.query_wasm_smart(factory_addr, &FactoryQueryMsg::Config {})
}

pub fn simulate(
    querier: &QuerierWrapper,
    pair_addr: Addr,
    offer_asset: &Asset,
) -> StdResult<SimulationResponse> {
    querier.query_wasm_smart(
        pair_addr,
        &PairQueryMsg::Simulation {
            offer_asset: offer_asset.clone(),
        },
    )
}

pub fn reverse_simulate(
    querier: &QuerierWrapper,
    pair_addr: Addr,
    ask_asset: &Asset,
) -> StdResult<ReverseSimulationResponse> {
    querier.query_wasm_smart(
        pair_addr,
        &PairQueryMsg::ReverseSimulation {
            ask_asset: ask_asset.clone(),
        },
    )
}

pub fn query_pair_info_from_pair(
    querier: &QuerierWrapper,
    pair_contract: Addr,
) -> StdResult<PairInfo> {
    let res: PairResponse = querier.query_wasm_smart(pair_contract, &PairQueryMsg::Pair {})?;
    Ok(res.info)
}

// upper bound key by 1, for Order::Ascending
pub fn calc_range_start(start_after: Option<Vec<u8>>) -> Option<Vec<u8>> {
    start_after.map(|mut input| {
        // zero out all trailing 255, increment first that is not such
        for i in (0..input.len()).rev() {
            if input[i] == 255 {
                input[i] = 0;
            } else {
                input[i] += 1;
                break;
            }
        }
        input
    })
}
