use cosmwasm_std::{
    attr, coin, to_binary, BankMsg, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env, HandleResponse,
    HumanAddr, InitResponse, MessageInfo, QuerierWrapper, QueryRequest, StdError, StdResult,
    Uint128, WasmMsg, WasmQuery,
};

use crate::state::{
    read_config, read_last_distributed, read_pool_reward_per_sec, store_config,
    store_last_distributed, store_pool_reward_per_sec, Config,
};

use oraiswap::staking::{
    AssetInfoWeight, HandleMsg as StakingCw20HookMsg, QueryMsg as StakingQueryMsg,
};

use oraiswap::distributor::{
    ConfigResponse, DistributionInfoResponse, HandleMsg, InitMsg, QueryMsg,
};

// use cw20::{Cw20HandleMsg, MinterResponse};
use cw20::Cw20HandleMsg;
use oraiswap::asset::{Asset, AssetInfo};

const DISTRIBUTION_INTERVAL: u64 = 60u64;

pub fn init(deps: DepsMut, env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    store_config(
        deps.storage,
        &Config {
            owner: deps.api.canonical_address(&info.sender)?,
            staking_contract: deps.api.canonical_address(&msg.staking_contract)?,
            token_code_id: msg.token_code_id,
            base_denom: msg.base_denom,
            genesis_time: to_seconds(env.block.time),
            distribution_schedule: msg.distribution_schedule,
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
            token_code_id,
            distribution_schedule,
        } => update_config(deps, info, owner, token_code_id, distribution_schedule),

        HandleMsg::Distribute { asset_info } => distribute(deps, env, asset_info),
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
    token_code_id: Option<u64>,
    distribution_schedule: Option<Vec<(u64, u64, Uint128)>>,
) -> StdResult<HandleResponse> {
    let mut config: Config = read_config(deps.storage)?;
    if config.owner != deps.api.canonical_address(&info.sender)? {
        return Err(StdError::generic_err("unauthorized"));
    }

    if let Some(owner) = owner {
        config.owner = deps.api.canonical_address(&owner)?;
    }

    if let Some(distribution_schedule) = distribution_schedule {
        config.distribution_schedule = distribution_schedule;
    }

    if let Some(token_code_id) = token_code_id {
        config.token_code_id = token_code_id;
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
    let mut config: Config = read_config(deps.storage)?;
    if config.owner != deps.api.canonical_address(&info.sender)? {
        return Err(StdError::generic_err("unauthorized"));
    }

    for ra in reward_per_sec {
        let asset_key = &ra.0.to_vec(deps.api)?;
        store_pool_reward_per_sec(deps.storage, asset_key, &ra.1);
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
    // let time_elapsed = to_seconds(env.block.time) - config.genesis_time;
    let last_time_elapsed = now - last_distributed;
    let mut target_distribution_amount: Uint128 = Uint128::zero();

    let staking_contract = deps.api.human_address(&config.staking_contract)?;

    let mut messages: Vec<CosmosMsg> = Vec::new();

    let rewards = asset_infos
        .into_iter()
        .map(|asset_info| {
            let reward_weights =
                query_reward_weights(&deps.querier, staking_contract.clone(), asset_info.clone())?;

            let total_weight: u32 = reward_weights.iter().map(|r| r.weight).sum();
            let mut distribution_amount: Uint128 = Uint128::zero();

            let asset_key = &asset_info.to_vec(deps.api)?;
            let pool_reward_per_sec = read_pool_reward_per_sec(deps.storage, asset_key)?;
            let target_distribution_amount: Uint128 =
                (pool_reward_per_sec * (last_time_elapsed as u128)).into();
            for w in reward_weights.iter() {
                let amount: Uint128 = Uint128::from(
                    target_distribution_amount.u128() * (w.weight as u128) / (total_weight as u128),
                );

                if amount.is_zero() {
                    return Err(StdError::generic_err("cannot distribute zero amount"));
                }

                match w.info {
                    AssetInfo::Token { contract_addr } => {
                        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: contract_addr,
                            msg: to_binary(&Cw20HandleMsg::Transfer {
                                recipient: staking_contract.clone(),
                                amount,
                            })?,
                            send: vec![],
                        }))
                    }
                    AssetInfo::NativeToken { denom } => {
                        messages.push(CosmosMsg::Bank(BankMsg::Send {
                            from_address: env.contract.address,
                            to_address: staking_contract.clone(),
                            amount: vec![coin(amount.u128(), denom)],
                        }))
                    }
                }

                distribution_amount += amount;
            }

            Ok(Asset {
                info: asset_info.clone(),
                amount: distribution_amount,
            })
        })
        .filter(|m| m.is_ok())
        .collect::<StdResult<Vec<Asset>>>()?;

    // store last distributed
    store_last_distributed(deps.storage, now)?;

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: staking_contract.clone(),
        msg: to_binary(&StakingCw20HookMsg::DepositReward { rewards })?,
        send: vec![],
    }));

    Ok(HandleResponse {
        messages,
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
        token_code_id: state.token_code_id,
        base_denom: state.base_denom,
        genesis_time: state.genesis_time,
        distribution_schedule: state.distribution_schedule,
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
