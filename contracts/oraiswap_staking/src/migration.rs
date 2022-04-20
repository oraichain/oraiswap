use cosmwasm_std::{Api, CanonicalAddr, Decimal, HumanAddr, Order, StdResult, Storage, Uint128};
use cosmwasm_storage::{singleton, singleton_read, Bucket, ReadonlyBucket};
use oraiswap::{asset::AssetInfo, staking::AmountInfo};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::{
    rewards_store, Config, MigrationParams, PoolInfo, RewardInfo, KEY_CONFIG, PREFIX_POOL_INFO,
};

pub static LEGACY_KEY_CONFIG: &[u8] = b"config";
pub static LEGACY_PREFIX_POOL_INFO: &[u8] = b"pool_info";
pub static LEGACY_PREFIX_REWARD: &[u8] = b"reward";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LegacyPoolInfo {
    pub staking_token: CanonicalAddr,
    pub pending_reward: Uint128, // not distributed amount due to zero bonding
    pub short_pending_reward: Uint128, // not distributed amount due to zero bonding
    pub total_bond_amount: Uint128,
    pub total_short_amount: Uint128,
    pub reward_index: Decimal,
    pub short_reward_index: Decimal,
    pub premium_rate: Decimal,
    pub short_reward_weight: Decimal,
    pub premium_updated_time: u64,
    pub migration_params: Option<MigrationParams>,
}

pub fn migrate_pool_infos(storage: &mut dyn Storage) -> StdResult<()> {
    let legacy_pool_infos_bucket: Bucket<LegacyPoolInfo> =
        Bucket::new(storage, LEGACY_PREFIX_POOL_INFO);

    let mut pools: Vec<(CanonicalAddr, LegacyPoolInfo)> = vec![];
    for item in legacy_pool_infos_bucket.range(None, None, Order::Ascending) {
        let (k, p) = item?;
        pools.push((CanonicalAddr::from(k), p));
    }

    // for (asset, _) in pools.clone().into_iter() {
    //     legacy_pool_infos_bucket.remove(asset.as_slice());
    // }

    let mut new_pool_infos_bucket: Bucket<PoolInfo> = Bucket::new(storage, PREFIX_POOL_INFO);

    for (asset, legacy_pool_info) in pools.into_iter() {
        let new_pool_info = &PoolInfo {
            staking_token: legacy_pool_info.staking_token,
            total_bond_amount: legacy_pool_info.total_bond_amount,
            reward_index: legacy_pool_info.reward_index,
            pending_reward: legacy_pool_info.pending_reward,
            migration_params: None,
        };
        new_pool_infos_bucket.save(asset.as_slice(), new_pool_info)?;
    }

    Ok(())
}

// migrate config
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LegacyConfig {
    pub owner: CanonicalAddr,
    pub reward_addr: CanonicalAddr,
    pub minter: CanonicalAddr,
    pub oracle_addr: CanonicalAddr,
    pub factory_addr: CanonicalAddr,
    pub base_denom: String,
    pub premium_min_update_interval: u64,
    // > premium_rate => < reward_weight
    pub short_reward_bound: (Decimal, Decimal),
}

pub fn read_old_config(storage: &dyn Storage) -> StdResult<LegacyConfig> {
    singleton_read(storage, LEGACY_KEY_CONFIG).load()
}

pub fn migrate_config(store: &mut dyn Storage) -> StdResult<()> {
    let config = read_old_config(store)?;

    // remove old config
    // singleton::<Config>(store, KEY_CONFIG).remove();
    let new_config = Config {
        owner: config.owner,
        rewarder: config.reward_addr,
        minter: config.minter,
        oracle_addr: config.oracle_addr,
        factory_addr: config.factory_addr,
        base_denom: config.base_denom,
    };

    singleton(store, KEY_CONFIG).save(&new_config)?;
    Ok(())
}

// migrate reward store
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LegacyRewardInfo {
    pub index: Decimal,
    pub bond_amount: Uint128,
    pub pending_reward: Uint128,
}

/// returns a bucket with all rewards owned by this owner (query it by owner)
/// (read-only version for queries)
pub fn legacy_rewards_read<'a>(
    storage: &'a dyn Storage,
    owner: &CanonicalAddr,
) -> ReadonlyBucket<'a, LegacyRewardInfo> {
    ReadonlyBucket::multilevel(storage, &[LEGACY_PREFIX_REWARD, owner.as_slice()])
}

pub fn migrate_rewards_store(
    store: &mut dyn Storage,
    api: &dyn Api,
    staker_addrs: Vec<HumanAddr>,
) -> StdResult<()> {
    let list_staker_addrs: Vec<CanonicalAddr> = staker_addrs
        .iter()
        .map(|addr| Ok(api.canonical_address(addr)?))
        .collect::<StdResult<Vec<CanonicalAddr>>>()?;
    for staker_addr in list_staker_addrs {
        let rewards_bucket = legacy_rewards_read(store, &staker_addr);
        let reward_pairs = rewards_bucket
            .range(None, None, Order::Ascending)
            .collect::<StdResult<Vec<(Vec<u8>, LegacyRewardInfo)>>>()?;

        for reward_pair in reward_pairs {
            let (asset_key, reward_info) = reward_pair;
            let native_token = if asset_key.len() == 20 { false } else { true };
            // try convert to contract token, otherwise it is native token
            let new_reward_info = RewardInfo {
                native_token,
                index: reward_info.index,
                bond_amount: reward_info.bond_amount,
                pending_reward: reward_info.pending_reward,
            };
            rewards_store(store, &staker_addr).save(&asset_key, &new_reward_info)?;
        }
    }

    Ok(())
}

// pub fn migrate_total_reward_amount(
//     store: &mut dyn Storage,
//     api: &dyn Api,
//     amount_infos: Vec<AmountInfo>,
// ) -> StdResult<()> {
//     for amount_info in amount_infos {
//         let asset_info_raw = amount_info.asset_info.to_raw(api)?;
//         store_total_reward_amount(store, asset_info_raw.as_bytes(), &amount_info.amount)?;
//     }
//     Ok(())
// }
