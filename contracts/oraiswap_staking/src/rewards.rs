use crate::state::{
    read_config, read_is_migrated, read_pool_info, rewards_read, rewards_store, store_pool_info,
    Config, PoolInfo, RewardInfo, CANONICAL_LENGTH,
};
use cosmwasm_std::{
    attr, coins, to_binary, Api, BankMsg, CanonicalAddr, CosmosMsg, Decimal, Deps, DepsMut, Env,
    HandleResponse, HumanAddr, MessageInfo, Order, StdResult, Storage, Uint128, WasmMsg,
};
use oraiswap::asset::{Asset, AssetInfo, AssetInfoRaw};
use oraiswap::staking::{RewardInfoResponse, RewardInfoResponseItem};

use cw20::Cw20HandleMsg;

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

    let config: Config = read_config(deps.storage)?;
    let total_amount = _withdraw_reward(deps.storage, &staker_addr, &asset_key)?;

    // now calculate weight
    let total_weight: u32 = config.reward_weights.iter().map(|(_, w)| w).sum();

    let mut messages: Vec<CosmosMsg> = vec![];

    for (asset_raw, weight) in config.reward_weights {
        // ignore empty weight
        if weight == 0 {
            continue;
        }
        let amount = Uint128::from(total_amount.u128() * (weight as u128) / (total_weight as u128));
        let message = match asset_raw {
            AssetInfoRaw::NativeToken { denom } => BankMsg::Send {
                from_address: env.contract.address.clone(),
                to_address: info.sender.clone(),
                amount: coins(amount.u128(), denom),
            }
            .into(),
            AssetInfoRaw::Token { contract_addr } => WasmMsg::Execute {
                contract_addr: deps.api.human_address(&contract_addr)?,
                msg: to_binary(&Cw20HandleMsg::Transfer {
                    recipient: info.sender.clone(),
                    amount,
                })?,
                send: vec![],
            }
            .into(),
        };
        messages.push(message);
    }

    Ok(HandleResponse {
        messages,
        attributes: vec![
            attr("action", "withdraw"),
            attr("amount", total_amount.to_string()),
        ],
        data: None,
    })
}

fn _withdraw_reward(
    storage: &mut dyn Storage,
    staker_addr: &CanonicalAddr,
    asset_key: &Option<Vec<u8>>,
) -> StdResult<Uint128> {
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

    let mut amount: Uint128 = Uint128::zero();
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

        amount += reward_info.pending_reward;
        reward_info.pending_reward = Uint128::zero();

        // Update rewards info
        if reward_info.bond_amount.is_zero() {
            rewards_store(storage, staker_addr).remove(&asset_key);
        } else {
            rewards_store(storage, staker_addr).save(&asset_key, &reward_info)?;
        }
    }

    Ok(amount)
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
