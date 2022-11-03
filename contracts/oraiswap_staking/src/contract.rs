use crate::migration::migrate_rewards_store;
use crate::rewards::{
    deposit_reward, process_reward_assets, query_all_reward_infos, query_reward_info,
    withdraw_reward, withdraw_reward_others,
};
use crate::staking::{auto_stake, auto_stake_hook, bond, unbond, update_list_stakers};
use crate::state::{
    read_config, read_pool_info, read_rewards_per_sec, stakers_read, store_config, store_pool_info,
    store_rewards_per_sec, Config, MigrationParams, PoolInfo,
};

use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, Binary, CanonicalAddr, Decimal, Deps, DepsMut, Env,
    MessageInfo, Order, Response, Response, Response, StdError, StdResult, Uint128,
};
use oraiswap::asset::{Asset, AssetInfo, AssetRaw, ORAI_DENOM};
use oraiswap::staking::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, PoolInfoResponse,
    QueryMsg, RewardsPerSecResponse,
};

use cw20::Cw20ReceiveMsg;

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    store_config(
        deps.storage,
        &Config {
            owner: deps
                .api
                .addr_canonicalize(&msg.owner.unwrap_or(info.sender.clone()))?,
            rewarder: deps.api.addr_canonicalize(&msg.rewarder)?,
            oracle_addr: deps.api.addr_canonicalize(&msg.oracle_addr)?,
            factory_addr: deps.api.addr_canonicalize(&msg.factory_addr)?,
            // default base_denom pass to factory is orai token
            base_denom: msg.base_denom.unwrap_or(ORAI_DENOM.to_string()),
        },
    )?;

    Ok(Response::default())
}

pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, info, msg),
        ExecuteMsg::UpdateConfig { rewarder, owner } => update_config(deps, info, owner, rewarder),
        ExecuteMsg::UpdateRewardsPerSec { asset_info, assets } => {
            update_rewards_per_sec(deps, info, asset_info, assets)
        }
        ExecuteMsg::DepositReward { rewards } => deposit_reward(deps, info, rewards),
        ExecuteMsg::RegisterAsset {
            asset_info,
            staking_token,
        } => register_asset(deps, info, asset_info, staking_token),
        ExecuteMsg::DeprecateStakingToken {
            asset_info,
            new_staking_token,
        } => deprecate_staking_token(deps, info, asset_info, new_staking_token),
        ExecuteMsg::Unbond { asset_info, amount } => {
            unbond(deps, env, info.sender, asset_info, amount)
        }
        ExecuteMsg::Withdraw { asset_info } => withdraw_reward(deps, env, info, asset_info),
        ExecuteMsg::WithdrawOthers {
            asset_info,
            staker_addrs,
        } => withdraw_reward_others(deps, env, info, staker_addrs, asset_info),
        ExecuteMsg::AutoStake {
            assets,
            slippage_tolerance,
        } => auto_stake(deps, env, info, assets, slippage_tolerance),
        ExecuteMsg::AutoStakeHook {
            asset_info,
            staking_token,
            staker_addr,
            prev_staking_token_amount,
        } => auto_stake_hook(
            deps,
            env,
            info,
            asset_info,
            staking_token,
            staker_addr,
            prev_staking_token_amount,
        ),
        ExecuteMsg::UpdateListStakers {
            asset_info,
            stakers,
        } => update_list_stakers(deps, env, info, asset_info, stakers),
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> StdResult<Response> {
    match from_binary(&cw20_msg.msg.unwrap_or(Binary::default())) {
        Ok(Cw20HookMsg::Bond { asset_info }) => {
            // check permission
            let asset_key = asset_info.to_vec(deps.api)?;
            let pool_info: PoolInfo = read_pool_info(deps.storage, &asset_key)?;

            // only staking token contract can execute this message
            let token_raw = deps.api.addr_canonicalize(info.sender.as_str())?;
            if pool_info.staking_token != token_raw {
                // if user is trying to bond old token, return friendly error message
                if let Some(params) = pool_info.migration_params {
                    if params.deprecated_staking_token == token_raw {
                        let staking_token_addr =
                            deps.api.addr_humanize(&pool_info.staking_token)?;
                        return Err(StdError::generic_err(format!(
                            "The staking token for this asset has been migrated to {}",
                            staking_token_addr
                        )));
                    }
                }

                return Err(StdError::generic_err("unauthorized"));
            }

            bond(deps, cw20_msg.sender, asset_info, cw20_msg.amount)
        }
        Err(_) => Err(StdError::generic_err("invalid cw20 hook message")),
    }
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<Addr>,
    rewarder: Option<Addr>,
) -> StdResult<Response> {
    let mut config: Config = read_config(deps.storage)?;

    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    if let Some(owner) = owner {
        config.owner = deps.api.addr_canonicalize(&owner)?;
    }

    if let Some(rewarder) = rewarder {
        config.rewarder = deps.api.addr_canonicalize(&rewarder)?;
    }

    store_config(deps.storage, &config)?;
    Ok(Response {
        messages: vec![],
        attributes: vec![attr("action", "update_config")],
        data: None,
    })
}

// need to withdraw all rewards of the stakers belong to the pool
// may need to call withdraw from backend side by querying all stakers with pagination in case out of gas
fn update_rewards_per_sec(
    deps: DepsMut,
    info: MessageInfo,
    asset_info: AssetInfo,
    assets: Vec<Asset>,
) -> StdResult<Response> {
    let config: Config = read_config(deps.storage)?;

    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    let asset_key = asset_info.to_vec(deps.api)?;

    // withdraw all rewards for all stakers from this pool
    let staker_addrs = stakers_read(deps.storage, &asset_key)
        .range(None, None, Order::Ascending)
        .map(|item| {
            let (k, _) = item?;
            Ok(CanonicalAddr::from(k))
        })
        .collect::<StdResult<Vec<CanonicalAddr>>>()?;

    // let mut messages: Vec<CosmosMsg> = vec![];

    // withdraw reward for each staker
    for staker_addr_raw in staker_addrs {
        process_reward_assets(
            deps.storage,
            &staker_addr_raw,
            &Some(asset_key.clone()),
            false,
        )?;
    }

    // convert assets to raw_assets
    let raw_assets = assets
        .into_iter()
        .map(|w| Ok(w.to_raw(deps.api)?))
        .collect::<StdResult<Vec<AssetRaw>>>()?;

    store_rewards_per_sec(deps.storage, &asset_key, raw_assets)?;

    Ok(Response {
        messages: vec![],
        attributes: vec![attr("action", "update_rewards_per_sec")],
        data: None,
    })
}

fn register_asset(
    deps: DepsMut,
    info: MessageInfo,
    asset_info: AssetInfo,
    staking_token: Addr,
) -> StdResult<Response> {
    let config: Config = read_config(deps.storage)?;

    if config.owner != deps.api.addr_canonicalize(info.sender.as_str())? {
        return Err(StdError::generic_err("unauthorized"));
    }

    // query asset_key from AssetInfo
    let asset_key = asset_info.to_vec(deps.api)?;
    if read_pool_info(deps.storage, &asset_key).is_ok() {
        return Err(StdError::generic_err("Asset was already registered"));
    }

    store_pool_info(
        deps.storage,
        &asset_key,
        &PoolInfo {
            staking_token: deps.api.addr_canonicalize(&staking_token)?,
            total_bond_amount: Uint128::zero(),
            reward_index: Decimal::zero(),
            pending_reward: Uint128::zero(),
            migration_params: None,
        },
    )?;

    Ok(Response {
        messages: vec![],
        attributes: vec![
            attr("action", "register_asset"),
            attr("asset_info", asset_info),
        ],
        data: None,
    })
}

fn deprecate_staking_token(
    deps: DepsMut,
    info: MessageInfo,
    asset_info: AssetInfo,
    new_staking_token: Addr,
) -> StdResult<Response> {
    let config: Config = read_config(deps.storage)?;

    if config.owner != deps.api.addr_canonicalize(info.sender.as_str())? {
        return Err(StdError::generic_err("unauthorized"));
    }

    let asset_key = asset_info.to_vec(deps.api)?;
    let mut pool_info: PoolInfo = read_pool_info(deps.storage, &asset_key)?;

    if pool_info.migration_params.is_some() {
        return Err(StdError::generic_err(
            "This asset LP token has already been migrated",
        ));
    }

    let deprecated_token_addr = deps.api.addr_humanize(&pool_info.staking_token)?;

    pool_info.total_bond_amount = Uint128::zero();
    pool_info.migration_params = Some(MigrationParams {
        index_snapshot: pool_info.reward_index,
        deprecated_staking_token: pool_info.staking_token,
    });
    pool_info.staking_token = deps.api.addr_canonicalize(&new_staking_token)?;

    store_pool_info(deps.storage, &asset_key, &pool_info)?;

    Ok(Response {
        messages: vec![],
        attributes: vec![
            attr("action", "depcrecate_staking_token"),
            attr("asset_info", asset_info),
            attr(
                "deprecated_staking_token",
                deprecated_token_addr.to_string(),
            ),
            attr("new_staking_token", new_staking_token.to_string()),
        ],
        data: None,
    })
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::PoolInfo { asset_info } => to_binary(&query_pool_info(deps, asset_info)?),
        QueryMsg::RewardsPerSec { asset_info } => {
            to_binary(&query_rewards_per_sec(deps, asset_info)?)
        }
        QueryMsg::RewardInfo {
            staker_addr,
            asset_info,
        } => to_binary(&query_reward_info(deps, staker_addr, asset_info)?),
        QueryMsg::RewardInfos {
            asset_info,
            start_after,
            limit,
            order,
        } => to_binary(&query_all_reward_infos(
            deps,
            asset_info,
            start_after,
            limit,
            order,
        )?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = read_config(deps.storage)?;
    let resp = ConfigResponse {
        owner: deps.api.addr_humanize(&state.owner)?,
        rewarder: deps.api.addr_humanize(&state.rewarder)?,
        oracle_addr: deps.api.addr_humanize(&state.oracle_addr)?,
        factory_addr: deps.api.addr_humanize(&state.factory_addr)?,
        base_denom: state.base_denom,
    };

    Ok(resp)
}

pub fn query_pool_info(deps: Deps, asset_info: AssetInfo) -> StdResult<PoolInfoResponse> {
    let asset_key = asset_info.to_vec(deps.api)?;
    let pool_info: PoolInfo = read_pool_info(deps.storage, &asset_key)?;
    Ok(PoolInfoResponse {
        asset_info,
        staking_token: deps.api.addr_humanize(&pool_info.staking_token)?,
        total_bond_amount: pool_info.total_bond_amount,
        reward_index: pool_info.reward_index,
        pending_reward: pool_info.pending_reward,
        migration_deprecated_staking_token: pool_info.migration_params.clone().map(|params| {
            deps.api
                .addr_humanize(&params.deprecated_staking_token)
                .unwrap()
        }),
        migration_index_snapshot: pool_info
            .migration_params
            .map(|params| params.index_snapshot),
    })
}

pub fn query_rewards_per_sec(
    deps: Deps,
    asset_info: AssetInfo,
) -> StdResult<RewardsPerSecResponse> {
    let asset_key = asset_info.to_vec(deps.api)?;

    let raw_assets = read_rewards_per_sec(deps.storage, &asset_key)?;

    let assets = raw_assets
        .into_iter()
        .map(|w| Ok(w.to_normal(deps.api)?))
        .collect::<StdResult<Vec<Asset>>>()?;

    Ok(RewardsPerSecResponse { assets })
}

// migrate contract
pub fn migrate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: MigrateMsg,
) -> StdResult<Response> {
    // migrate_pool_infos(deps.storage)?;
    // migrate_config(deps.storage)?;
    migrate_rewards_store(deps.storage, deps.api, msg.staker_addrs)?;
    // migrate_total_reward_amount(deps.storage, deps.api, msg.amount_infos)?;

    // when the migration is executed, deprecate directly the MIR pool
    // let config = read_config(deps.storage)?;
    // let self_info = MessageInfo {
    //     sender: deps.api.addr_humanize(&config.owner)?,
    //     sent_funds: vec![],
    // };

    // depricate old one
    // deprecate_staking_token(
    //     deps,
    //     self_info,
    //     msg.asset_info_to_deprecate,
    //     msg.new_staking_token,
    // )?;

    Ok(Response::default())
}
