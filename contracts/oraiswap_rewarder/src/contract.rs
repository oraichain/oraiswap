use cosmwasm_std::{
    attr, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, HandleResponse, HumanAddr,
    InitResponse, MessageInfo, Order, StdError, StdResult, Uint128, WasmMsg,
};
use cosmwasm_storage::ReadonlyBucket;

use crate::state::{
    read_config, read_last_distributed, read_pool_reward_per_sec, store_config,
    store_last_distributed, store_pool_reward_per_sec, Config, PREFIX_REWARD_PER_SEC,
};

use oraiswap::staking::HandleMsg as StakingHandleMsg;

use oraiswap::rewarder::{
    ConfigResponse, DistributionInfoResponse, HandleMsg, InitMsg, QueryMsg, RewardPerSecondResponse,
};

use oraiswap::asset::{Asset, AssetInfo};

// 600 seconds default
const DEFAULT_DISTRIBUTION_INTERVAL: u64 = 600;

pub fn init(deps: DepsMut, env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    store_config(
        deps.storage,
        &Config {
            owner: deps.api.canonical_address(&info.sender)?,
            staking_contract: deps.api.canonical_address(&msg.staking_contract)?,
            distribution_interval: msg
                .distribution_interval
                .unwrap_or(DEFAULT_DISTRIBUTION_INTERVAL),
        },
    )?;

    store_last_distributed(deps.storage, env.block.time)?;

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

        HandleMsg::Distribute {} => distribute(deps, env),
        HandleMsg::UpdateRewardPerSec { reward } => update_reward_per_sec(deps, info, reward),
        HandleMsg::UpdateRewardsPerSec { rewards } => update_rewards_per_sec(deps, info, rewards),
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

pub fn update_reward_per_sec(
    deps: DepsMut,
    info: MessageInfo,
    reward: Asset,
) -> StdResult<HandleResponse> {
    let config: Config = read_config(deps.storage)?;
    if config.owner != deps.api.canonical_address(&info.sender)? {
        return Err(StdError::generic_err("unauthorized"));
    }

    let asset_key = reward.info.to_vec(deps.api)?;
    store_pool_reward_per_sec(deps.storage, &asset_key, &reward)?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_reward_per_sec")],
        data: None,
    })
}

pub fn update_rewards_per_sec(
    deps: DepsMut,
    info: MessageInfo,
    rewards: Vec<Asset>,
) -> StdResult<HandleResponse> {
    let config: Config = read_config(deps.storage)?;
    if config.owner != deps.api.canonical_address(&info.sender)? {
        return Err(StdError::generic_err("unauthorized"));
    }

    for reward in rewards {
        let asset_key = reward.info.to_vec(deps.api)?;
        store_pool_reward_per_sec(deps.storage, &asset_key, &reward)?;
    }

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_rewards_per_sec")],
        data: None,
    })
}

/// Distribute
/// Anyone can handle distribute operation to distribute
pub fn distribute(deps: DepsMut, env: Env) -> StdResult<HandleResponse> {
    let config: Config = read_config(deps.storage)?;
    let last_distributed = read_last_distributed(deps.storage)?;
    let now = env.block.time;
    let last_time_elapsed = now - last_distributed;
    if last_time_elapsed < config.distribution_interval {
        return Err(StdError::generic_err(
            "Cannot distribute reward tokens before interval",
        ));
    }

    // convert reward
    let staking_contract = deps.api.human_address(&config.staking_contract)?;
    let reward_bucket: ReadonlyBucket<Asset> =
        ReadonlyBucket::new(deps.storage, PREFIX_REWARD_PER_SEC);
    let rewards = reward_bucket
        .range(None, None, Order::Ascending)
        .map(|item| {
            let (_, reward) = item?;

            let target_distribution_amount =
                Uint128(reward.amount.u128() * (last_time_elapsed as u128));

            Ok(Asset {
                info: reward.info,
                amount: target_distribution_amount,
            })
        })
        .collect::<StdResult<Vec<Asset>>>()?;

    // store last distributed
    store_last_distributed(deps.storage, now)?;

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
        QueryMsg::DistributionInfo {} => to_binary(&query_distribution_info(deps)?),
        QueryMsg::RewardPerSec { asset_info } => {
            to_binary(&query_reward_per_sec(deps, asset_info)?)
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

pub fn query_distribution_info(deps: Deps) -> StdResult<DistributionInfoResponse> {
    let last_distributed = read_last_distributed(deps.storage)?;
    let resp = DistributionInfoResponse { last_distributed };

    Ok(resp)
}

pub fn query_reward_per_sec(
    deps: Deps,
    asset_info: AssetInfo,
) -> StdResult<RewardPerSecondResponse> {
    let asset_key = asset_info.to_vec(deps.api)?;
    let reward = read_pool_reward_per_sec(deps.storage, &asset_key)?;

    Ok(RewardPerSecondResponse { reward })
}
