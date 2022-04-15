use cosmwasm_std::{
    attr, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, HandleResponse, HumanAddr,
    InitResponse, MessageInfo, QuerierWrapper, QueryRequest, StdError, StdResult, Uint128, WasmMsg,
    WasmQuery,
};

use crate::state::{
    read_config, read_last_distributed, read_pool_reward_per_sec, store_config,
    store_last_distributed, store_pool_reward_per_sec, Config,
};

use oraiswap::staking::{
    AssetInfoWeight, HandleMsg as StakingCw20HookMsg, QueryMsg as StakingQueryMsg,
};

use oraiswap::rewarder::{ConfigResponse, DistributionInfoResponse, HandleMsg, InitMsg, QueryMsg};

use oraiswap::asset::{Asset, AssetInfo};

const DISTRIBUTION_INTERVAL: u64 = 60u64;

pub fn init(deps: DepsMut, env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    store_config(
        deps.storage,
        &Config {
            owner: deps.api.canonical_address(&info.sender)?,
            staking_contract: deps.api.canonical_address(&msg.staking_contract)?,
            genesis_time: to_seconds(env.block.time),
        },
    )?;

    store_last_distributed(deps.storage, to_seconds(env.block.time))?;

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
        } => update_config(deps, info, owner, staking_contract),

        HandleMsg::Distribute { asset_infos } => distribute(deps, env, asset_infos),
        HandleMsg::UpdateRewardPerSec {
            owner,
            reward_per_sec,
        } => update_reward_per_sec(deps, info, owner, reward_per_sec),
    }
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<HumanAddr>,
    staking_contract: Option<HumanAddr>,
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
    owner: Option<HumanAddr>,
    reward_per_sec: Vec<(AssetInfo, u128)>,
) -> StdResult<HandleResponse> {
    let config: Config = read_config(deps.storage)?;
    if config.owner != deps.api.canonical_address(&info.sender)? {
        return Err(StdError::generic_err("unauthorized"));
    }

    for ra in reward_per_sec {
        let asset_key = &ra.0.to_vec(deps.api)?;
        store_pool_reward_per_sec(deps.storage, asset_key, &ra.1)?;
    }

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_reward_per_sec")],
        data: None,
    })
}

/// Distribute
/// Anyone can handle distribute operation to distribute
/// mirror inflation rewards on the staking pool
pub fn distribute(
    deps: DepsMut,
    env: Env,
    asset_infos: Vec<AssetInfo>,
) -> StdResult<HandleResponse> {
    let last_distributed = read_last_distributed(deps.storage)?;
    let now = to_seconds(env.block.time);
    if last_distributed + DISTRIBUTION_INTERVAL > now {
        return Err(StdError::generic_err(
            "Cannot distribute reward tokens before interval",
        ));
    }

    let config: Config = read_config(deps.storage)?;
    let last_time_elapsed = now - last_distributed;

    let staking_contract = deps.api.human_address(&config.staking_contract)?;

    let rewards = asset_infos
        .into_iter()
        .map(|asset_info| {
            let asset_key = &asset_info.to_vec(deps.api)?;
            let pool_reward_per_sec = read_pool_reward_per_sec(deps.storage, asset_key)?;
            let target_distribution_amount: Uint128 =
                (pool_reward_per_sec * (last_time_elapsed as u128)).into();

            Ok(Asset {
                info: asset_info.clone(),
                amount: target_distribution_amount,
            })
        })
        .filter(|m| m.is_ok())
        .collect::<StdResult<Vec<Asset>>>()?;

    // store last distributed
    store_last_distributed(deps.storage, now)?;

    Ok(HandleResponse {
        messages: vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: staking_contract.clone(),
            msg: to_binary(&StakingCw20HookMsg::DepositReward { rewards })?,
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
        QueryMsg::RewardWeights {
            staking_contract_addr,
            asset_info,
        } => to_binary(&query_reward_weights(
            &deps.querier,
            staking_contract_addr,
            asset_info,
        )?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = read_config(deps.storage)?;
    let resp = ConfigResponse {
        owner: deps.api.human_address(&state.owner)?,
        genesis_time: state.genesis_time,
        staking_contract: deps.api.human_address(&state.staking_contract)?,
    };

    Ok(resp)
}

pub fn query_distribution_info(deps: Deps) -> StdResult<DistributionInfoResponse> {
    let last_distributed = read_last_distributed(deps.storage)?;
    let resp = DistributionInfoResponse { last_distributed };

    Ok(resp)
}

fn to_seconds(nanoseconds: u64) -> u64 {
    nanoseconds / 1_000_000_000
}

pub fn query_reward_weights(
    querier: &QuerierWrapper,
    staking_contract_addr: HumanAddr,
    asset_info: AssetInfo,
) -> StdResult<Vec<AssetInfoWeight>> {
    let res: Vec<AssetInfoWeight> = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: staking_contract_addr,
        msg: to_binary(&StakingQueryMsg::RewardWeights { asset_info })?,
    }))?;

    Ok(res)
}
