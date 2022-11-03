use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, QuerierWrapper,
    Response, StdError, StdResult, Uint128, WasmMsg,
};
use cosmwasm_std::{QueryRequest, WasmQuery};

use crate::state::{
    read_config, read_last_distributed, store_config, store_last_distributed, Config,
};

use oraiswap::staking::QueryMsg as StakingQueryMsg;
use oraiswap::staking::{ExecuteMsg as StakingExecuteMsg, RewardsPerSecResponse};

use oraiswap::rewarder::{
    ConfigResponse, DistributionInfoResponse, ExecuteMsg, InstantiateMsg, QueryMsg,
    RewardAmountPerSecondResponse,
};

use oraiswap::asset::{Asset, AssetInfo};

// 600 seconds default
const DEFAULT_DISTRIBUTION_INTERVAL: u64 = 600;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    store_config(
        deps.storage,
        &Config {
            init_time: env.block.time.seconds(),
            owner: deps.api.addr_canonicalize(info.sender.as_str())?,
            staking_contract: deps.api.addr_canonicalize(msg.staking_contract.as_str())?,
            distribution_interval: msg
                .distribution_interval
                .unwrap_or(DEFAULT_DISTRIBUTION_INTERVAL),
        },
    )?;

    Ok(Response::default())
}

#[entry_point]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateConfig {
            owner,
            staking_contract,
            distribution_interval,
        } => update_config(deps, info, owner, staking_contract, distribution_interval),

        ExecuteMsg::Distribute { asset_infos } => distribute(deps, env, asset_infos),
    }
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<Addr>,
    staking_contract: Option<Addr>,
    distribution_interval: Option<u64>,
) -> StdResult<Response> {
    let mut config: Config = read_config(deps.storage)?;
    if config.owner != deps.api.addr_canonicalize(info.sender.as_str())? {
        return Err(StdError::generic_err("unauthorized"));
    }

    if let Some(owner) = owner {
        config.owner = deps.api.addr_canonicalize(owner.as_str())?;
    }

    if let Some(staking_contract) = staking_contract {
        config.staking_contract = deps.api.addr_canonicalize(staking_contract.as_str())?;
    }

    if let Some(distribution_interval) = distribution_interval {
        config.distribution_interval = distribution_interval;
    }

    store_config(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

/// Distribute
/// Anyone can handle distribute operation to distribute
pub fn distribute(deps: DepsMut, env: Env, asset_infos: Vec<AssetInfo>) -> StdResult<Response> {
    let config: Config = read_config(deps.storage)?;
    let staking_contract = deps.api.addr_humanize(&config.staking_contract)?;
    let now = env.block.time.seconds();
    let mut rewards: Vec<Asset> = vec![];
    for asset_info in asset_infos {
        let asset_key = asset_info.to_vec(deps.api)?;
        // default is init time
        let last_distributed = read_last_distributed(deps.storage, &asset_key)
            .unwrap_or(now - config.distribution_interval - 1);

        let last_time_elapsed = now - last_distributed;
        if last_time_elapsed < config.distribution_interval {
            // Cannot distribute reward tokens before interval, process next one
            continue;
        }

        // store last distributed
        store_last_distributed(deps.storage, &&asset_key, now)?;

        // reward amount per second for a pool
        let reward_amount =
            _read_pool_reward_per_sec(&deps.querier, staking_contract.clone(), asset_info.clone())?;

        // get total reward amount for a pool
        let distribution_amount = Uint128::from(reward_amount.u128() * (last_time_elapsed as u128));

        // update rewards
        rewards.push(Asset {
            info: asset_info,
            amount: distribution_amount,
        });
    }

    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: staking_contract.to_string(),
            msg: to_binary(&StakingExecuteMsg::DepositReward { rewards })?,
            funds: vec![],
        }))
        .add_attributes(vec![attr("action", "distribute")]))
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::DistributionInfo { asset_info } => {
            to_binary(&query_distribution_info(deps, asset_info)?)
        }
        QueryMsg::RewardAmountPerSec { asset_info } => {
            to_binary(&query_reward_amount_per_sec(deps, asset_info)?)
        }
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = read_config(deps.storage)?;
    let resp = ConfigResponse {
        owner: deps.api.addr_humanize(&state.owner)?,
        staking_contract: deps.api.addr_humanize(&state.staking_contract)?,
        distribution_interval: state.distribution_interval,
    };

    Ok(resp)
}

pub fn query_distribution_info(
    deps: Deps,
    asset_info: AssetInfo,
) -> StdResult<DistributionInfoResponse> {
    let asset_key = asset_info.to_vec(deps.api)?;
    let last_distributed = read_last_distributed(deps.storage, &asset_key)?;
    let resp = DistributionInfoResponse { last_distributed };

    Ok(resp)
}

pub fn query_reward_amount_per_sec(
    deps: Deps,
    asset_info: AssetInfo,
) -> StdResult<RewardAmountPerSecondResponse> {
    let state = read_config(deps.storage)?;
    let reward_amount = _read_pool_reward_per_sec(
        &deps.querier,
        deps.api.addr_humanize(&state.staking_contract)?,
        asset_info,
    )?;

    Ok(RewardAmountPerSecondResponse { reward_amount })
}

fn _read_pool_reward_per_sec(
    querier: &QuerierWrapper,
    staking_contract: Addr,
    asset_info: AssetInfo,
) -> StdResult<Uint128> {
    let res: RewardsPerSecResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: staking_contract.to_string(),
        msg: to_binary(&StakingQueryMsg::RewardsPerSec { asset_info })?,
    }))?;

    Ok(res.assets.iter().map(|a| a.amount).sum())
}
