use crate::math::compute_short_reward_weight;
use crate::querier::compute_premium_rate;
use crate::state::{
    read_config, read_is_migrated, read_pool_info, rewards_read, rewards_store, store_pool_info,
    Config, PoolInfo, RewardInfo,
};
use cosmwasm_std::{
    attr, to_binary, Api, CanonicalAddr, Decimal, Deps, DepsMut, Env, HandleResponse, HumanAddr,
    MessageInfo, Order, StdError, StdResult, Storage, Uint128, WasmMsg,
};
use oraiswap::asset::Asset;
use oraiswap::staking::{RewardInfoResponse, RewardInfoResponseItem};

use cw20::Cw20HandleMsg;

pub fn adjust_premium(
    deps: DepsMut,
    env: Env,
    asset_tokens: Vec<HumanAddr>,
) -> StdResult<HandleResponse> {
    let config: Config = read_config(deps.storage)?;
    let oracle_contract = deps.api.human_address(&config.oracle_contract)?;
    let oraiswap_factory = deps.api.human_address(&config.oraiswap_factory)?;

    for asset_token in asset_tokens.iter() {
        let asset_token_raw = deps.api.canonical_address(&asset_token)?;
        let pool_info: PoolInfo = read_pool_info(deps.storage, &asset_token_raw)?;
        if env.block.time < pool_info.premium_updated_time + config.premium_min_update_interval {
            return Err(StdError::generic_err(
                "cannot adjust premium before premium_min_update_interval passed",
            ));
        }

        let (premium_rate, no_price_feed) = compute_premium_rate(
            deps.as_ref(),
            oracle_contract.clone(),
            oraiswap_factory.clone(),
            asset_token.to_owned(),
            config.base_denom.to_string(),
        )?;

        // if asset does not have price feed, set short reward weight directly to zero
        let short_reward_weight = if no_price_feed {
            Decimal::zero()
        } else {
            // maximum short reward when premium rate > 7%
            if premium_rate > config.short_reward_bound.0 {
                config.short_reward_bound.1
            } else {
                compute_short_reward_weight(premium_rate)?
            }
        };

        store_pool_info(
            deps.storage,
            &asset_token_raw,
            &PoolInfo {
                premium_rate,
                short_reward_weight,
                premium_updated_time: env.block.time,
                ..pool_info
            },
        )?;
    }

    Ok(HandleResponse {
        attributes: vec![attr("action", "premium_adjustment")],
        messages: vec![],
        data: None,
    })
}

// deposit_reward must be from reward token contract
pub fn deposit_reward(
    deps: DepsMut,
    rewards: Vec<(HumanAddr, Uint128)>,
    rewards_amount: Uint128,
) -> StdResult<HandleResponse> {
    for (asset_token, amount) in rewards.iter() {
        let asset_token_raw: CanonicalAddr = deps.api.canonical_address(&asset_token)?;
        let mut pool_info: PoolInfo = read_pool_info(deps.storage, &asset_token_raw)?;

        // Decimal::from_ratio(1, 5).mul()
        // erf(pool_info.premium_rate.0)
        // 3.0f64
        let total_reward = *amount;
        // short_reward came from sLP Tokens are minted and immediately staked when a short position is created
        let mut short_reward = total_reward * pool_info.short_reward_weight;
        let mut normal_reward = Asset::checked_sub(total_reward, short_reward).unwrap();

        if pool_info.total_bond_amount.is_zero() {
            pool_info.pending_reward += normal_reward;
        } else {
            normal_reward += pool_info.pending_reward;
            let normal_reward_per_bond =
                Decimal::from_ratio(normal_reward, pool_info.total_bond_amount);
            pool_info.reward_index = pool_info.reward_index + normal_reward_per_bond;
            pool_info.pending_reward = Uint128::zero();
        }

        if pool_info.total_short_amount.is_zero() {
            pool_info.short_pending_reward += short_reward;
        } else {
            short_reward += pool_info.short_pending_reward;
            let short_reward_per_bond =
                Decimal::from_ratio(short_reward, pool_info.total_short_amount);
            pool_info.short_reward_index = pool_info.short_reward_index + short_reward_per_bond;
            pool_info.short_pending_reward = Uint128::zero();
        }

        store_pool_info(deps.storage, &asset_token_raw, &pool_info)?;
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
    info: MessageInfo,
    asset_token: Option<HumanAddr>,
) -> StdResult<HandleResponse> {
    let staker_addr = deps.api.canonical_address(&info.sender)?;
    // unwrap option<result> to result<option>
    let asset_token = asset_token.map_or(Ok(None), |a| deps.api.canonical_address(&a).map(Some))?;
    // .map_or(Ok(None), |r| r.map(Some))?;
    let normal_reward = _withdraw_reward(deps.storage, &staker_addr, &asset_token, false)?;
    let short_reward = _withdraw_reward(deps.storage, &staker_addr, &asset_token, true)?;

    let amount = normal_reward + short_reward;
    let config: Config = read_config(deps.storage)?;
    Ok(HandleResponse {
        messages: vec![WasmMsg::Execute {
            contract_addr: deps.api.human_address(&config.oraix_token)?,
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: info.sender,
                amount,
            })?,
            send: vec![],
        }
        .into()],
        attributes: vec![
            attr("action", "withdraw"),
            attr("amount", amount.to_string()),
        ],
        data: None,
    })
}

fn _withdraw_reward(
    storage: &mut dyn Storage,
    staker_addr: &CanonicalAddr,
    asset_token: &Option<CanonicalAddr>,
    is_short: bool,
) -> StdResult<Uint128> {
    let rewards_bucket = rewards_read(storage, staker_addr, is_short);

    // single reward withdraw
    let reward_pairs: Vec<(CanonicalAddr, RewardInfo)> = if let Some(asset_token) = asset_token {
        let reward_info = rewards_bucket.may_load(asset_token.as_slice())?;
        if let Some(reward_info) = reward_info {
            vec![(asset_token.clone(), reward_info)]
        } else {
            vec![]
        }
    } else {
        rewards_bucket
            .range(None, None, Order::Ascending)
            .map(|item| {
                let (k, v) = item?;
                Ok((CanonicalAddr::from(k), v))
            })
            .collect::<StdResult<Vec<(CanonicalAddr, RewardInfo)>>>()?
    };

    let mut amount: Uint128 = Uint128::zero();
    for reward_pair in reward_pairs {
        let (asset_token_raw, mut reward_info) = reward_pair;
        let pool_info: PoolInfo = read_pool_info(storage, &asset_token_raw)?;

        // Withdraw reward to pending reward
        // if the lp token was migrated, and the user did not close their position yet, cap the reward at the snapshot
        let pool_index = if is_short {
            pool_info.short_reward_index
        } else if pool_info.migration_params.is_some()
            && !read_is_migrated(storage, &asset_token_raw, staker_addr)
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
            rewards_store(storage, staker_addr, is_short).remove(asset_token_raw.as_slice());
        } else {
            rewards_store(storage, staker_addr, is_short)
                .save(asset_token_raw.as_slice(), &reward_info)?;
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
    asset_token: Option<HumanAddr>,
) -> StdResult<RewardInfoResponse> {
    let staker_addr_raw = deps.api.canonical_address(&staker_addr)?;

    let reward_infos: Vec<RewardInfoResponseItem> = vec![
        _read_reward_infos(
            deps.api,
            deps.storage,
            &staker_addr_raw,
            &asset_token,
            false,
        )?,
        _read_reward_infos(deps.api, deps.storage, &staker_addr_raw, &asset_token, true)?,
    ]
    .concat();

    Ok(RewardInfoResponse {
        staker_addr,
        reward_infos,
    })
}

fn _read_reward_infos(
    api: &dyn Api,
    storage: &dyn Storage,
    staker_addr: &CanonicalAddr,
    asset_token: &Option<HumanAddr>,
    is_short: bool,
) -> StdResult<Vec<RewardInfoResponseItem>> {
    let rewards_bucket = rewards_read(storage, staker_addr, is_short);
    let reward_infos: Vec<RewardInfoResponseItem> = if let Some(asset_token) = asset_token {
        let asset_token_raw = api.canonical_address(&asset_token)?;

        if let Some(mut reward_info) = rewards_bucket.may_load(asset_token_raw.as_slice())? {
            let pool_info = read_pool_info(storage, &asset_token_raw)?;

            let (pool_index, should_migrate) = if is_short {
                (pool_info.short_reward_index, None)
            } else if pool_info.migration_params.is_some()
                && !read_is_migrated(storage, &asset_token_raw, staker_addr)
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
                asset_token: asset_token.clone(),
                bond_amount: reward_info.bond_amount,
                pending_reward: reward_info.pending_reward,
                is_short,
                should_migrate,
            }]
        } else {
            vec![]
        }
    } else {
        rewards_bucket
            .range(None, None, Order::Ascending)
            .map(|item| {
                let (k, v) = item?;
                let asset_token_raw = CanonicalAddr::from(k);
                let mut reward_info = v;

                let pool_info = read_pool_info(storage, &asset_token_raw)?;
                let (pool_index, should_migrate) = if is_short {
                    (pool_info.short_reward_index, None)
                } else if pool_info.migration_params.is_some()
                    && !read_is_migrated(storage, &asset_token_raw, staker_addr)
                {
                    (
                        pool_info.migration_params.unwrap().index_snapshot,
                        Some(true),
                    )
                } else {
                    (pool_info.reward_index, None)
                };

                before_share_change(pool_index, &mut reward_info)?;

                Ok(RewardInfoResponseItem {
                    asset_token: api.human_address(&asset_token_raw)?,
                    bond_amount: reward_info.bond_amount,
                    pending_reward: reward_info.pending_reward,
                    is_short,
                    should_migrate,
                })
            })
            .collect::<StdResult<Vec<RewardInfoResponseItem>>>()?
    };

    Ok(reward_infos)
}
