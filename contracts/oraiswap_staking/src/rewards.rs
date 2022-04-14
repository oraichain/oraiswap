use crate::state::{
    calc_range_start, read_config, read_is_migrated, read_pool_info, read_reward_weights,
    read_total_reward_amount, rewards_read, rewards_store, stakers_read, store_pool_info,
    store_total_reward_amount, PoolInfo, RewardInfo, CANONICAL_LENGTH,
};
use cosmwasm_std::{
    attr, Api, CanonicalAddr, CosmosMsg, Decimal, Deps, DepsMut, Env, HandleResponse, HumanAddr,
    MessageInfo, Order, StdError, StdResult, Storage, Uint128,
};
use oraiswap::asset::{Asset, AssetInfo, AssetRaw};
use oraiswap::staking::{RewardInfoResponse, RewardInfoResponseItem};

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

// deposit_reward must be from reward token contract
pub fn deposit_reward(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    rewards: Vec<Asset>,
) -> StdResult<HandleResponse> {
    let config = read_config(deps.storage)?;

    // only rewarder can execute this message, rewarder may be a contract
    if config.rewarder != deps.api.canonical_address(&info.sender)? {
        return Err(StdError::generic_err("unauthorized"));
    }

    let mut rewards_amount = Uint128::zero();

    // for each asset, make sure we have enough balance according to weight, so we need to store total amount of each token and verify it
    for asset in rewards.iter() {
        let asset_key = asset.info.to_vec(deps.api)?;

        // get reward_weights from this pool:
        let reward_weights = read_reward_weights(deps.storage, &asset_key).map_err(|_err| {
            StdError::generic_err(format!("No reward weights for '{}' stored", asset.info))
        })?;

        let total_weight: u32 = reward_weights.iter().map(|rw| rw.weight).sum();

        // check total_amount of each asset
        for rw in reward_weights {
            // ignore empty weight
            if rw.weight == 0 {
                continue;
            }
            let plus_amount =
                Uint128::from(asset.amount.u128() * (rw.weight as u128) / (total_weight as u128));

            // update new total_reward_amount
            let reward_asset_key = rw.info.as_bytes();
            let total_reward_amount = plus_amount
                + read_total_reward_amount(deps.storage, &reward_asset_key)
                    .unwrap_or(Uint128::zero());

            // check if current reward asset balance >= reward_amount then update new total_amount, otherwise throw Error
            let rw_info = rw.info.to_normal(deps.api)?;
            let reward_balance = rw_info
                .query_pool(&deps.querier, env.contract.address.clone())
                .unwrap_or(Uint128::zero());

            // each time call deposit reward, must check the balance is enough
            if reward_balance.lt(&total_reward_amount) {
                return Err(StdError::generic_err(format!(
                    "token {} has not enough balance",
                    rw_info
                )));
            }

            // update new total_reward_amount
            store_total_reward_amount(deps.storage, &reward_asset_key, &total_reward_amount)?;
        }

        let mut pool_info: PoolInfo = read_pool_info(deps.storage, &asset_key)?;

        let mut normal_reward = asset.amount;

        // normal rewards are array of Assets
        if pool_info.total_bond_amount.is_zero() {
            pool_info.pending_reward += normal_reward;
        } else {
            normal_reward += pool_info.pending_reward;
            let normal_reward_per_bond =
                Decimal::from_ratio(normal_reward, pool_info.total_bond_amount);
            pool_info.reward_index = pool_info.reward_index + normal_reward_per_bond;
            pool_info.pending_reward = Uint128::zero();
        }

        store_pool_info(deps.storage, &asset_key, &pool_info)?;

        rewards_amount += asset.amount;
    }

    Ok(HandleResponse {
        messages: vec![],
        data: None,
        attributes: vec![
            attr("action", "deposit_reward"),
            attr("rewards_amount", rewards_amount.to_string()),
        ],
    })
}

// withdraw all rewards or single reward depending on asset_token
pub fn withdraw_reward(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset_info: Option<AssetInfo>,
) -> StdResult<HandleResponse> {
    let staker_addr = deps.api.canonical_address(&info.sender)?;
    let asset_key = asset_info.map_or(None, |a| a.to_vec(deps.api).ok());

    let reward_assets = process_withdraw_reward(deps.storage, deps.api, staker_addr, asset_key)?;

    let messages = reward_assets
        .into_iter()
        .map(|ra| {
            Ok(ra.into_msg(
                None,
                &deps.querier,
                env.contract.address.clone(),
                info.sender.clone(),
            )?)
        })
        .collect::<StdResult<Vec<CosmosMsg>>>()?;

    Ok(HandleResponse {
        messages,
        attributes: vec![attr("action", "withdraw_reward")],
        data: None,
    })
}

pub fn withdraw_reward_others(
    deps: DepsMut,
    env: Env,
    staker_addrs: Vec<HumanAddr>,
    asset_info: Option<AssetInfo>,
) -> StdResult<HandleResponse> {
    let asset_key = asset_info.map_or(None, |a| a.to_vec(deps.api).ok());
    let mut messages: Vec<CosmosMsg> = vec![];

    // withdraw reward for each staker
    for staker_addr in staker_addrs {
        let staker_addr_raw = deps.api.canonical_address(&staker_addr)?;
        let reward_assets =
            process_withdraw_reward(deps.storage, deps.api, staker_addr_raw, asset_key.clone())?;

        messages.extend(
            reward_assets
                .into_iter()
                .map(|ra| {
                    Ok(ra.into_msg(
                        None,
                        &deps.querier,
                        env.contract.address.clone(),
                        staker_addr.clone(),
                    )?)
                })
                .collect::<StdResult<Vec<CosmosMsg>>>()?,
        );
    }

    Ok(HandleResponse {
        messages,
        attributes: vec![attr("action", "withdraw_reward_others")],
        data: None,
    })
}

// update total_reward_amount and reward info then return reward assets
pub fn process_withdraw_reward(
    storage: &mut dyn Storage,
    api: &dyn Api,
    staker_addr: CanonicalAddr,
    asset_key: Option<Vec<u8>>,
) -> StdResult<Vec<Asset>> {
    // get reward assets and convert into CosmMsg
    let reward_raw_assets = _get_reward_assets(storage, &staker_addr, &asset_key)?;
    let mut reward_assets: Vec<Asset> = vec![];
    for reward_raw_asset in reward_raw_assets {
        let reward_asset = reward_raw_asset.to_normal(api)?;
        // each reward amount we need to update total_reward_amount
        let reward_asset_key = reward_raw_asset.info.as_bytes();
        let total_reward_amount =
            read_total_reward_amount(storage, &reward_asset_key).unwrap_or(Uint128::zero());

        // each time call withdraw reward, must check the balance is enough, so if total_reward_amount < reward_asset.amount
        // an error will be thrown
        store_total_reward_amount(
            storage,
            &reward_asset_key,
            &(total_reward_amount - reward_asset.amount)?,
        )?;

        // finally push messsage
        reward_assets.push(reward_asset);
    }

    Ok(reward_assets)
}

fn _get_reward_assets(
    storage: &mut dyn Storage,
    staker_addr: &CanonicalAddr,
    asset_key: &Option<Vec<u8>>,
) -> StdResult<Vec<AssetRaw>> {
    let rewards_bucket = rewards_read(storage, staker_addr);

    // single reward withdraw, using Vec to store reference variable in local function
    let reward_pairs = if let Some(asset_key) = asset_key {
        let reward_info = rewards_bucket.may_load(&asset_key)?;
        if let Some(reward_info) = reward_info {
            vec![(asset_key.to_vec(), reward_info)]
        } else {
            vec![]
        }
    } else {
        rewards_bucket
            .range(None, None, Order::Ascending)
            .collect::<StdResult<Vec<(Vec<u8>, RewardInfo)>>>()?
    };

    let mut reward_assets: Vec<AssetRaw> = vec![];

    for reward_pair in reward_pairs {
        let (asset_key, mut reward_info) = reward_pair;
        let pool_info: PoolInfo = read_pool_info(storage, &asset_key)?;

        // Withdraw reward to pending reward
        // if the lp token was migrated, and the user did not close their position yet, cap the reward at the snapshot
        let pool_index = if pool_info.migration_params.is_some()
            && !read_is_migrated(storage, &asset_key, staker_addr)
        {
            pool_info.migration_params.unwrap().index_snapshot
        } else {
            pool_info.reward_index
        };

        before_share_change(pool_index, &mut reward_info)?;

        let total_amount = reward_info.pending_reward;
        // calculate and accumulate the reward amount
        let reward_weights = read_reward_weights(storage, &asset_key)?;
        // now calculate weight
        let total_weight: u32 = reward_weights.iter().map(|rw| rw.weight).sum();

        for rw in reward_weights {
            // ignore empty weight
            if rw.weight == 0 {
                continue;
            }
            let amount =
                Uint128::from(total_amount.u128() * (rw.weight as u128) / (total_weight as u128));

            // update, first time push it, later update the amount
            match reward_assets.iter_mut().find(|ra| ra.info.eq(&rw.info)) {
                None => {
                    reward_assets.push(AssetRaw {
                        info: rw.info,
                        amount,
                    });
                }
                Some(reward_asset) => {
                    reward_asset.amount += amount;
                }
            }
        }

        // reset pending_reward
        reward_info.pending_reward = Uint128::zero();

        // Update rewards info
        if reward_info.bond_amount.is_zero() {
            rewards_store(storage, staker_addr).remove(&asset_key);
        } else {
            rewards_store(storage, staker_addr).save(&asset_key, &reward_info)?;
        }
    }

    Ok(reward_assets)
}

// withdraw reward to pending reward
pub fn before_share_change(pool_index: Decimal, reward_info: &mut RewardInfo) -> StdResult<()> {
    let pending_reward = Asset::checked_sub(
        reward_info.bond_amount * pool_index,
        reward_info.bond_amount * reward_info.index,
    )?;

    reward_info.index = pool_index;
    reward_info.pending_reward += pending_reward;
    Ok(())
}

pub fn query_reward_info(
    deps: Deps,
    staker_addr: HumanAddr,
    asset_info: Option<AssetInfo>,
) -> StdResult<RewardInfoResponse> {
    let staker_addr_raw = deps.api.canonical_address(&staker_addr)?;

    let reward_infos: Vec<RewardInfoResponseItem> =
        _read_reward_infos(deps.api, deps.storage, &staker_addr_raw, &asset_info)?;

    Ok(RewardInfoResponse {
        staker_addr,
        reward_infos,
    })
}

pub fn query_all_reward_infos(
    deps: Deps,
    asset_info: AssetInfo,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
    order: Option<u8>,
) -> StdResult<Vec<RewardInfoResponse>> {
    let asset_key = asset_info.to_vec(deps.api)?;
    let start_after = start_after
        .map_or(None, |a| deps.api.canonical_address(&a).ok())
        .map(|c| c.to_vec());

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let (start, end, order_by) = match order {
        Some(1) => (calc_range_start(start_after), None, Order::Ascending),
        _ => (None, start_after, Order::Descending),
    };

    let info_responses = stakers_read(deps.storage, &asset_key)
        .range(start.as_deref(), end.as_deref(), order_by)
        .take(limit)
        .map(|item| {
            let (k, _) = item?;
            let staker_addr_raw = CanonicalAddr::from(k);
            let reward_infos: Vec<RewardInfoResponseItem> = _read_reward_infos(
                deps.api,
                deps.storage,
                &staker_addr_raw,
                &Some(asset_info.clone()),
            )?;
            let staker_addr = deps.api.human_address(&staker_addr_raw)?;
            Ok(RewardInfoResponse {
                staker_addr,
                reward_infos,
            })
        })
        .collect::<StdResult<Vec<RewardInfoResponse>>>()?;

    Ok(info_responses)
}

fn _read_reward_infos(
    api: &dyn Api,
    storage: &dyn Storage,
    staker_addr: &CanonicalAddr,
    asset_info: &Option<AssetInfo>,
) -> StdResult<Vec<RewardInfoResponseItem>> {
    let rewards_bucket = rewards_read(storage, staker_addr);
    let reward_infos: Vec<RewardInfoResponseItem> = if let Some(asset_info) = asset_info {
        let asset_key = asset_info.to_vec(api)?;

        if let Some(mut reward_info) = rewards_bucket.may_load(&asset_key)? {
            let pool_info = read_pool_info(storage, &asset_key)?;

            let (pool_index, should_migrate) = if pool_info.migration_params.is_some()
                && !read_is_migrated(storage, &asset_key, staker_addr)
            {
                (
                    pool_info.migration_params.unwrap().index_snapshot,
                    Some(true),
                )
            } else {
                (pool_info.reward_index, None)
            };

            before_share_change(pool_index, &mut reward_info)?;

            vec![RewardInfoResponseItem {
                asset_info: asset_info.to_owned(),
                bond_amount: reward_info.bond_amount,
                pending_reward: reward_info.pending_reward,

                should_migrate,
            }]
        } else {
            vec![]
        }
    } else {
        rewards_bucket
            .range(None, None, Order::Ascending)
            .map(|item| {
                let (asset_key, mut reward_info) = item?;

                let pool_info = read_pool_info(storage, &asset_key)?;
                let (pool_index, should_migrate) = if pool_info.migration_params.is_some()
                    && !read_is_migrated(storage, &asset_key, staker_addr)
                {
                    (
                        pool_info.migration_params.unwrap().index_snapshot,
                        Some(true),
                    )
                } else {
                    (pool_info.reward_index, None)
                };

                before_share_change(pool_index, &mut reward_info)?;

                // try convert to contract token, otherwise it is native token

                let asset_info = if asset_key.len() == CANONICAL_LENGTH {
                    AssetInfo::Token {
                        contract_addr: api.human_address(&asset_key.into())?,
                    }
                } else {
                    AssetInfo::NativeToken {
                        denom: String::from_utf8(asset_key)?,
                    }
                };

                Ok(RewardInfoResponseItem {
                    asset_info,
                    bond_amount: reward_info.bond_amount,
                    pending_reward: reward_info.pending_reward,
                    should_migrate,
                })
            })
            .collect::<StdResult<Vec<RewardInfoResponseItem>>>()?
    };

    Ok(reward_infos)
}
