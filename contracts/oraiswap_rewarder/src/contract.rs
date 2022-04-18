use cosmwasm_std::{
    attr, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, HandleResponse, HumanAddr,
    InitResponse, MessageInfo, QuerierWrapper, StdError, StdResult, Uint128, WasmMsg,
};
use cosmwasm_std::{QueryRequest, WasmQuery};

use crate::state::{
    read_config, read_last_distributed, store_config, store_last_distributed, Config,
};

use oraiswap::staking::QueryMsg as StakingQueryMsg;
use oraiswap::staking::{HandleMsg as StakingHandleMsg, RewardsPerSecResponse};

use oraiswap::rewarder::{
    ConfigResponse, DistributionInfoResponse, HandleMsg, InitMsg, QueryMsg,
    RewardAmountPerSecondResponse,
};

use oraiswap::asset::{Asset, AssetInfo};

// 600 seconds default
const DEFAULT_DISTRIBUTION_INTERVAL: u64 = 600;

pub fn init(deps: DepsMut, env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    store_config(
        deps.storage,
        &Config {
            init_time: env.block.time,
            owner: deps.api.canonical_address(&info.sender)?,
            staking_contract: deps.api.canonical_address(&msg.staking_contract)?,
            distribution_interval: msg
                .distribution_interval
                .unwrap_or(DEFAULT_DISTRIBUTION_INTERVAL),
        },
    )?;

    Ok(InitResponse::default())
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::UpdateConfig {
            owner,
            staking_contract,
            distribution_interval,
        } => update_config(deps, info, owner, staking_contract, distribution_interval),

        HandleMsg::Distribute { asset_infos } => distribute(deps, env, asset_infos),
    }
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<HumanAddr>,
    staking_contract: Option<HumanAddr>,
    distribution_interval: Option<u64>,
) -> StdResult<HandleResponse> {
    let mut config: Config = read_config(deps.storage)?;
    if config.owner != deps.api.canonical_address(&info.sender)? {
        return Err(StdError::generic_err("unauthorized"));
    }

    if let Some(owner) = owner {
        config.owner = deps.api.canonical_address(&owner)?;
    }

    if let Some(staking_contract) = staking_contract {
        config.staking_contract = deps.api.canonical_address(&staking_contract)?;
    }

    if let Some(distribution_interval) = distribution_interval {
        config.distribution_interval = distribution_interval;
    }

    store_config(deps.storage, &config)?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_config")],
        data: None,
    })
}

/// Distribute
/// Anyone can handle distribute operation to distribute
pub fn distribute(
    deps: DepsMut,
    env: Env,
    asset_infos: Vec<AssetInfo>,
) -> StdResult<HandleResponse> {
    let config: Config = read_config(deps.storage)?;
    let staking_contract = deps.api.human_address(&config.staking_contract)?;
    let now = env.block.time;
    let mut rewards: Vec<Asset> = vec![];
    for asset_info in asset_infos {
        let asset_key = asset_info.to_vec(deps.api)?;
        // default is init time
        let last_distributed =
            read_last_distributed(deps.storage, &asset_key).unwrap_or(config.init_time);

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
        let distribution_amount = Uint128(reward_amount.u128() * (last_time_elapsed as u128));

        // update rewards
        rewards.push(Asset {
            info: asset_info,
            amount: distribution_amount,
        });
    }

    Ok(HandleResponse {
        messages: vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: staking_contract,
            msg: to_binary(&StakingHandleMsg::DepositReward { rewards })?,
            send: vec![],
        })],
        data: None,
        attributes: vec![attr("action", "distribute")],
    })
}

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
        owner: deps.api.human_address(&state.owner)?,
        staking_contract: deps.api.human_address(&state.staking_contract)?,
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
        deps.api.human_address(&state.staking_contract)?,
        asset_info,
    )?;

    Ok(RewardAmountPerSecondResponse { reward_amount })
}

fn _read_pool_reward_per_sec(
    querier: &QuerierWrapper,
    staking_contract: HumanAddr,
    asset_info: AssetInfo,
) -> StdResult<Uint128> {
    let res: RewardsPerSecResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: staking_contract,
        msg: to_binary(&StakingQueryMsg::RewardsPerSec { asset_info })?,
    }))?;

    Ok(res.assets.iter().map(|a| a.amount).sum())
}
