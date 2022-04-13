use crate::state::{
    read_is_migrated, read_pool_info, read_reward_weights, rewards_read, rewards_store,
    store_pool_info, PoolInfo, RewardInfo, CANONICAL_LENGTH,
};
use cosmwasm_std::{
    attr, Api, CanonicalAddr, CosmosMsg, Decimal, Deps, DepsMut, Env, HandleResponse, HumanAddr,
    MessageInfo, Order, StdResult, Storage, Uint128,
};
use oraiswap::asset::{Asset, AssetInfo, AssetRaw};
use oraiswap::staking::{RewardInfoResponse, RewardInfoResponseItem};

// deposit_reward must be from reward token contract
pub fn deposit_reward(
    deps: DepsMut,
    rewards: Vec<Asset>,
    rewards_amount: Uint128,
) -> StdResult<HandleResponse> {
    for asset in rewards.iter() {
        let asset_key = asset.info.to_vec(deps.api)?;
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

    // get reward assets and convert into CosmMsg
    let reward_assets = _get_reward_assets(deps.storage, &staker_addr, &asset_key)?;
    let messages = reward_assets
        .into_iter()
        .map(|reward_asset| {
            Ok(reward_asset.to_normal(deps.api)?.into_msg(
                None,
                &deps.querier,
                env.contract.address.clone(),
                info.sender.clone(),
            )?)
        })
        .collect::<StdResult<Vec<CosmosMsg>>>()?;

    Ok(HandleResponse {
        messages,
        attributes: vec![attr("action", "withdraw")],
        data: None,
    })
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
