use crate::rewards::before_share_change;
use crate::state::{
    read_config, read_is_migrated, read_pool_info, rewards_read, rewards_store, store_is_migrated,
    store_pool_info, Config, PoolInfo, RewardInfo,
};
use cosmwasm_std::{
    attr, to_binary, CanonicalAddr, Coin, Decimal, DepsMut, Env, HandleResponse, HumanAddr,
    MessageInfo, StdError, StdResult, Storage, Uint128, WasmMsg,
};
use cw20::Cw20HandleMsg;
use oraiswap::asset::{Asset, AssetInfo, PairInfo};
use oraiswap::oracle::OracleContract;
use oraiswap::pair::HandleMsg as PairHandleMsg;
use oraiswap::querier::{query_pair_info, query_token_balance};
use oraiswap::staking::HandleMsg;

pub fn bond(
    deps: DepsMut,
    staker_addr: HumanAddr,
    asset_info: AssetInfo,
    amount: Uint128,
) -> StdResult<HandleResponse> {
    let staker_addr_raw: CanonicalAddr = deps.api.canonical_address(&staker_addr)?;
    let asset_key = asset_info.to_vec(deps.api)?;
    _increase_bond_amount(deps.storage, &staker_addr_raw, &asset_key, amount)?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "bond"),
            attr("staker_addr", staker_addr.as_str()),
            attr("asset_info", asset_info),
            attr("amount", amount.to_string()),
        ],
        data: None,
    })
}

pub fn unbond(
    deps: DepsMut,
    staker_addr: HumanAddr,
    asset_info: AssetInfo,
    amount: Uint128,
) -> StdResult<HandleResponse> {
    let staker_addr_raw: CanonicalAddr = deps.api.canonical_address(&staker_addr)?;
    let asset_key = asset_info.to_vec(deps.api)?;
    let staking_token: CanonicalAddr =
        _decrease_bond_amount(deps.storage, &staker_addr_raw, &asset_key, amount)?;
    let staking_token_addr = deps.api.human_address(&staking_token)?;

    Ok(HandleResponse {
        messages: vec![WasmMsg::Execute {
            contract_addr: staking_token_addr.clone(),
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: staker_addr.clone(),
                amount,
            })?,
            send: vec![],
        }
        .into()],
        attributes: vec![
            attr("action", "unbond"),
            attr("staker_addr", staker_addr.as_str()),
            attr("asset_info", asset_info),
            attr("amount", amount.to_string()),
            attr("staking_token", staking_token_addr.as_str()),
        ],
        data: None,
    })
}

pub fn auto_stake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    assets: [Asset; 2],
    slippage_tolerance: Option<Decimal>,
) -> StdResult<HandleResponse> {
    let config: Config = read_config(deps.storage)?;
    let factory_addr = deps.api.human_address(&config.factory_addr)?;

    let mut native_asset_op: Option<Asset> = None;
    let mut token_info_op: Option<(HumanAddr, Uint128)> = None;
    for asset in assets.iter() {
        match asset.info.clone() {
            AssetInfo::NativeToken { .. } => {
                asset.assert_sent_native_token_balance(&info)?;
                native_asset_op = Some(asset.clone())
            }
            AssetInfo::Token { contract_addr } => {
                token_info_op = Some((contract_addr, asset.amount))
            }
        }
    }

    // will fail if one of them is missing
    let native_asset: Asset = match native_asset_op {
        Some(v) => v,
        None => return Err(StdError::generic_err("Missing native asset")),
    };
    let (token_addr, token_amount) = match token_info_op {
        Some(v) => v,
        None => return Err(StdError::generic_err("Missing token asset")),
    };

    // query pair info to obtain pair contract address
    let asset_infos: [AssetInfo; 2] = [assets[0].info.clone(), assets[1].info.clone()];
    let oraiswap_pair: PairInfo = query_pair_info(&deps.querier, factory_addr, &asset_infos)?;

    // assert the token and lp token match with pool info
    let pool_info: PoolInfo =
        read_pool_info(deps.storage, &deps.api.canonical_address(&token_addr)?)?;

    if pool_info.staking_token != deps.api.canonical_address(&oraiswap_pair.liquidity_token)? {
        return Err(StdError::generic_err("Invalid staking token"));
    }

    // get current lp token amount to later compute the recived amount
    let prev_staking_token_amount = query_token_balance(
        &deps.querier,
        oraiswap_pair.liquidity_token.clone(),
        env.contract.address.clone(),
    )?;

    let oracle_contract = OracleContract(oraiswap_pair.oracle_addr);

    // compute tax
    let tax_amount: Uint128 = native_asset.compute_tax(&oracle_contract, &deps.querier)?;

    // 1. Transfer token asset to staking contract
    // 2. Increase allowance of token for pair contract
    // 3. Provide liquidity
    // 4. Execute staking hook, will stake in the name of the sender
    Ok(HandleResponse {
        messages: vec![
            WasmMsg::Execute {
                contract_addr: token_addr.clone(),
                msg: to_binary(&Cw20HandleMsg::TransferFrom {
                    owner: info.sender.clone(),
                    recipient: env.contract.address.clone(),
                    amount: token_amount,
                })?,
                send: vec![],
            }
            .into(),
            WasmMsg::Execute {
                contract_addr: token_addr.clone(),
                msg: to_binary(&Cw20HandleMsg::IncreaseAllowance {
                    spender: oraiswap_pair.contract_addr.clone(),
                    amount: token_amount,
                    expires: None,
                })?,
                send: vec![],
            }
            .into(),
            WasmMsg::Execute {
                contract_addr: oraiswap_pair.contract_addr.clone(),
                msg: to_binary(&PairHandleMsg::ProvideLiquidity {
                    assets: [
                        Asset {
                            amount: Asset::checked_sub(native_asset.amount, tax_amount)?,
                            info: native_asset.info.clone(),
                        },
                        Asset {
                            amount: token_amount,
                            info: AssetInfo::Token {
                                contract_addr: token_addr.clone(),
                            },
                        },
                    ],
                    slippage_tolerance,
                    receiver: None,
                })?,
                send: vec![Coin {
                    denom: native_asset.info.to_string(),
                    amount: Asset::checked_sub(native_asset.amount, tax_amount)?,
                }],
            }
            .into(),
            WasmMsg::Execute {
                contract_addr: env.contract.address,
                msg: to_binary(&HandleMsg::AutoStakeHook {
                    asset_info: AssetInfo::Token {
                        contract_addr: token_addr.clone(),
                    },
                    staking_token: oraiswap_pair.liquidity_token,
                    staker_addr: info.sender,
                    prev_staking_token_amount,
                })?,
                send: vec![],
            }
            .into(),
        ],
        attributes: vec![
            attr("action", "auto_stake"),
            attr("asset_token", token_addr.to_string()),
            attr("tax_amount", tax_amount.to_string()),
        ],
        data: None,
    })
}

pub fn auto_stake_hook(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset_info: AssetInfo,
    staking_token: HumanAddr,
    staker_addr: HumanAddr,
    prev_staking_token_amount: Uint128,
) -> StdResult<HandleResponse> {
    // only can be called by itself
    if info.sender != env.contract.address {
        return Err(StdError::generic_err("unauthorized"));
    }

    // stake all lp tokens received, compare with staking token amount before liquidity provision was executed
    let current_staking_token_amount =
        query_token_balance(&deps.querier, staking_token, env.contract.address)?;
    let amount_to_stake =
        Asset::checked_sub(current_staking_token_amount, prev_staking_token_amount)?;

    bond(deps, staker_addr, asset_info, amount_to_stake)
}

fn _increase_bond_amount(
    storage: &mut dyn Storage,
    staker_addr: &CanonicalAddr,
    asset_key: &[u8],
    amount: Uint128,
) -> StdResult<()> {
    let mut pool_info: PoolInfo = read_pool_info(storage, asset_key)?;
    let mut reward_info: RewardInfo = rewards_read(storage, staker_addr)
        .load(asset_key)
        .unwrap_or_else(|_| RewardInfo {
            index: Decimal::zero(),
            bond_amount: Uint128::zero(),
            pending_reward: Uint128::zero(),
        });

    // check if the position should be migrated
    let is_position_migrated = read_is_migrated(storage, asset_key, staker_addr);
    if pool_info.migration_params.is_some() {
        // the pool has been migrated, if position is not migrated and has tokens bonded, return error
        if !reward_info.bond_amount.is_zero() && !is_position_migrated {
            return Err(StdError::generic_err("The LP token for this asset has been deprecated, withdraw all your deprecated tokens to migrate your position"));
        } else if !is_position_migrated {
            // if the position is not migrated, but bond amount is zero, it means it's a new position, so store it as migrated
            store_is_migrated(storage, asset_key, staker_addr)?;
        }
    }

    // Withdraw reward to pending reward; before changing share
    before_share_change(pool_info.reward_index, &mut reward_info)?;

    // Increase total bond amount
    pool_info.total_bond_amount += amount;

    reward_info.bond_amount += amount;

    rewards_store(storage, staker_addr).save(asset_key, &reward_info)?;
    store_pool_info(storage, asset_key, &pool_info)?;

    Ok(())
}

fn _decrease_bond_amount(
    storage: &mut dyn Storage,
    staker_addr: &CanonicalAddr,
    asset_key: &[u8],
    amount: Uint128,
) -> StdResult<CanonicalAddr> {
    let mut pool_info: PoolInfo = read_pool_info(storage, asset_key)?;
    let mut reward_info: RewardInfo = rewards_read(storage, staker_addr).load(asset_key)?;

    if reward_info.bond_amount < amount {
        return Err(StdError::generic_err("Cannot unbond more than bond amount"));
    }

    // if the lp token was migrated, and the user did not close their position yet, cap the reward at the snapshot
    let should_migrate =
        !read_is_migrated(storage, asset_key, staker_addr) && pool_info.migration_params.is_some();
    let (pool_index, staking_token) = if should_migrate {
        let migraton_params = pool_info.migration_params.clone().unwrap();
        (
            migraton_params.index_snapshot,
            migraton_params.deprecated_staking_token,
        )
    } else {
        (pool_info.reward_index, pool_info.staking_token.clone())
    };

    // Distribute reward to pending reward; before changing share
    before_share_change(pool_index, &mut reward_info)?;

    // Decrease total bond amount
    if !should_migrate {
        // if it should migrate, we dont need to decrease from the current total bond amount
        pool_info.total_bond_amount = Asset::checked_sub(pool_info.total_bond_amount, amount)?;
    }

    // Update rewards info
    reward_info.bond_amount = Asset::checked_sub(reward_info.bond_amount, amount)?;

    if reward_info.bond_amount.is_zero() && should_migrate {
        store_is_migrated(storage, asset_key, staker_addr)?;
    }

    if reward_info.pending_reward.is_zero() && reward_info.bond_amount.is_zero() {
        rewards_store(storage, staker_addr).remove(asset_key);
    } else {
        rewards_store(storage, staker_addr).save(asset_key, &reward_info)?;
    }

    // Update pool info
    store_pool_info(storage, asset_key, &pool_info)?;

    Ok(staking_token)
}
