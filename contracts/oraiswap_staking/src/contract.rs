use crate::migration::migrate_pool_infos;
use crate::rewards::{adjust_premium, deposit_reward, query_reward_info, withdraw_reward};
use crate::staking::{
    auto_stake, auto_stake_hook, bond, decrease_short_token, increase_short_token, unbond,
};
use crate::state::{
    read_config, read_pool_info, store_config, store_pool_info, Config, MigrationParams, PoolInfo,
};

use cosmwasm_std::{
    attr, from_binary, to_binary, Binary, Decimal, Deps, DepsMut, Env, HandleResponse, HumanAddr,
    InitResponse, MessageInfo, MigrateResponse, StdError, StdResult, Uint128,
};
use oraiswap::asset::{AssetInfo, ORAI_DENOM};
use oraiswap::staking::{
    ConfigResponse, Cw20HookMsg, HandleMsg, InitMsg, MigrateMsg, PoolInfoResponse, QueryMsg,
};

use cw20::Cw20ReceiveMsg;

pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    store_config(
        deps.storage,
        &Config {
            owner: deps
                .api
                .canonical_address(&msg.owner.unwrap_or(info.sender.clone()))?,
            reward_addr: deps.api.canonical_address(&msg.reward_addr)?,
            // this is for minting short token
            minter: deps
                .api
                .canonical_address(&msg.minter.unwrap_or(info.sender))?,
            oracle_addr: deps.api.canonical_address(&msg.oracle_addr)?,
            factory_addr: deps.api.canonical_address(&msg.factory_addr)?,
            // default base_denom pass to factory is orai token
            base_denom: msg.base_denom.unwrap_or(ORAI_DENOM.to_string()),
            premium_min_update_interval: msg.premium_min_update_interval.unwrap_or(7600), // 2 hours
            // premium rate > 7% then reward_weight = 40%
            short_reward_bound: msg
                .short_reward_bound
                .unwrap_or((Decimal::percent(7), Decimal::percent(40))),
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
        HandleMsg::Receive(msg) => receive_cw20(deps, info, msg),
        HandleMsg::UpdateConfig {
            owner,
            premium_min_update_interval,
            short_reward_bound,
        } => update_config(
            deps,
            info,
            owner,
            premium_min_update_interval,
            short_reward_bound,
        ),
        HandleMsg::RegisterAsset {
            asset_info,
            staking_token,
        } => register_asset(deps, info, asset_info, staking_token),
        HandleMsg::DeprecateStakingToken {
            asset_info,
            new_staking_token,
        } => deprecate_staking_token(deps, info, asset_info, new_staking_token),
        HandleMsg::Unbond { asset_info, amount } => unbond(deps, info.sender, asset_info, amount),
        HandleMsg::Withdraw { asset_info } => withdraw_reward(deps, info, asset_info),
        HandleMsg::AdjustPremium { asset_tokens } => adjust_premium(deps, env, asset_tokens),
        HandleMsg::IncreaseShortToken {
            staker_addr,
            asset_token,
            amount,
        } => increase_short_token(deps, info, staker_addr, asset_token, amount),
        HandleMsg::DecreaseShortToken {
            staker_addr,
            asset_token,
            amount,
        } => decrease_short_token(deps, info, staker_addr, asset_token, amount),
        HandleMsg::AutoStake {
            assets,
            slippage_tolerance,
        } => auto_stake(deps, env, info, assets, slippage_tolerance),
        HandleMsg::AutoStakeHook {
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
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> StdResult<HandleResponse> {
    match from_binary(&cw20_msg.msg.unwrap_or(Binary::default())) {
        Ok(Cw20HookMsg::Bond { asset_info }) => {
            // check permission
            let asset_key = asset_info.to_vec(deps.api)?;
            let pool_info: PoolInfo = read_pool_info(deps.storage, &asset_key)?;

            // only staking token contract can execute this message
            let token_raw = deps.api.canonical_address(&info.sender)?;
            if pool_info.staking_token != token_raw {
                // if user is trying to bond old token, return friendly error message
                if let Some(params) = pool_info.migration_params {
                    if params.deprecated_staking_token == token_raw {
                        let staking_token_addr =
                            deps.api.human_address(&pool_info.staking_token)?;
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
        Ok(Cw20HookMsg::DepositReward { rewards }) => {
            let config: Config = read_config(deps.storage)?;

            // only reward token contract can execute this message
            if config.reward_addr != deps.api.canonical_address(&info.sender)? {
                return Err(StdError::generic_err("unauthorized"));
            }

            let mut rewards_amount = Uint128::zero();
            for (_, amount) in rewards.iter() {
                rewards_amount += *amount;
            }

            if rewards_amount != cw20_msg.amount {
                return Err(StdError::generic_err("rewards amount miss matched"));
            }

            deposit_reward(deps, rewards, rewards_amount)
        }
        Err(_) => Err(StdError::generic_err("invalid cw20 hook message")),
    }
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<HumanAddr>,
    premium_min_update_interval: Option<u64>,
    short_reward_bound: Option<(Decimal, Decimal)>,
) -> StdResult<HandleResponse> {
    let mut config: Config = read_config(deps.storage)?;

    if deps.api.canonical_address(&info.sender)? != config.owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    if let Some(owner) = owner {
        config.owner = deps.api.canonical_address(&owner)?;
    }

    if let Some(premium_min_update_interval) = premium_min_update_interval {
        config.premium_min_update_interval = premium_min_update_interval;
    }

    if let Some(short_reward_bound) = short_reward_bound {
        config.short_reward_bound = short_reward_bound;
    }

    store_config(deps.storage, &config)?;
    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_config")],
        data: None,
    })
}

fn register_asset(
    deps: DepsMut,
    info: MessageInfo,
    asset_info: AssetInfo,
    staking_token: HumanAddr,
) -> StdResult<HandleResponse> {
    let config: Config = read_config(deps.storage)?;

    if config.owner != deps.api.canonical_address(&info.sender)? {
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
            staking_token: deps.api.canonical_address(&staking_token)?,
            total_bond_amount: Uint128::zero(),
            total_short_amount: Uint128::zero(),
            reward_index: Decimal::zero(),
            short_reward_index: Decimal::zero(),
            pending_reward: Uint128::zero(),
            short_pending_reward: Uint128::zero(),
            premium_rate: Decimal::zero(),
            short_reward_weight: Decimal::zero(),
            premium_updated_time: 0,
            migration_params: None,
        },
    )?;

    Ok(HandleResponse {
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
    new_staking_token: HumanAddr,
) -> StdResult<HandleResponse> {
    let config: Config = read_config(deps.storage)?;

    if config.owner != deps.api.canonical_address(&info.sender)? {
        return Err(StdError::generic_err("unauthorized"));
    }

    let asset_key = asset_info.to_vec(deps.api)?;
    let mut pool_info: PoolInfo = read_pool_info(deps.storage, &asset_key)?;

    if pool_info.migration_params.is_some() {
        return Err(StdError::generic_err(
            "This asset LP token has already been migrated",
        ));
    }

    let deprecated_token_addr = deps.api.human_address(&pool_info.staking_token)?;

    pool_info.total_bond_amount = Uint128::zero();
    pool_info.migration_params = Some(MigrationParams {
        index_snapshot: pool_info.reward_index,
        deprecated_staking_token: pool_info.staking_token,
    });
    pool_info.staking_token = deps.api.canonical_address(&new_staking_token)?;

    store_pool_info(deps.storage, &asset_key, &pool_info)?;

    Ok(HandleResponse {
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
        QueryMsg::RewardInfo {
            staker_addr,
            asset_info,
        } => to_binary(&query_reward_info(deps, staker_addr, asset_info)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = read_config(deps.storage)?;
    let resp = ConfigResponse {
        owner: deps.api.human_address(&state.owner)?,
        reward_addr: deps.api.human_address(&state.reward_addr)?,
        minter: deps.api.human_address(&state.minter)?,
        oracle_addr: deps.api.human_address(&state.oracle_addr)?,
        factory_addr: deps.api.human_address(&state.factory_addr)?,
        base_denom: state.base_denom,
        premium_min_update_interval: state.premium_min_update_interval,
    };

    Ok(resp)
}

pub fn query_pool_info(deps: Deps, asset_info: AssetInfo) -> StdResult<PoolInfoResponse> {
    let asset_key = asset_info.to_vec(deps.api)?;
    let pool_info: PoolInfo = read_pool_info(deps.storage, &asset_key)?;
    Ok(PoolInfoResponse {
        asset_info,
        staking_token: deps.api.human_address(&pool_info.staking_token)?,
        total_bond_amount: pool_info.total_bond_amount,
        total_short_amount: pool_info.total_short_amount,
        reward_index: pool_info.reward_index,
        short_reward_index: pool_info.short_reward_index,
        pending_reward: pool_info.pending_reward,
        short_pending_reward: pool_info.short_pending_reward,
        premium_rate: pool_info.premium_rate,
        short_reward_weight: pool_info.short_reward_weight,
        premium_updated_time: pool_info.premium_updated_time,
        migration_deprecated_staking_token: pool_info.migration_params.clone().map(|params| {
            deps.api
                .human_address(&params.deprecated_staking_token)
                .unwrap()
        }),
        migration_index_snapshot: pool_info
            .migration_params
            .map(|params| params.index_snapshot),
    })
}

// migrate contract
pub fn migrate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: MigrateMsg,
) -> StdResult<MigrateResponse> {
    migrate_pool_infos(deps.storage)?;

    // when the migration is executed, deprecate directly the MIR pool
    let config = read_config(deps.storage)?;
    let self_info = MessageInfo {
        sender: deps.api.human_address(&config.owner)?,
        sent_funds: vec![],
    };

    // depricate old one
    deprecate_staking_token(
        deps,
        self_info,
        msg.asset_info_to_deprecate,
        msg.new_staking_token,
    )?;

    Ok(MigrateResponse::default())
}
