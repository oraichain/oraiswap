use oraiswap::staking::AssetInfoRawWeight;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, Decimal, StdResult, Storage, Uint128};
use cosmwasm_storage::{singleton, singleton_read, Bucket, ReadonlyBucket};

pub static KEY_CONFIG: &[u8] = b"config";
pub static PREFIX_POOL_INFO: &[u8] = b"pool_info";
static PREFIX_REWARD: &[u8] = b"reward";
static PREFIX_STAKER: &[u8] = b"staker";
static PREFIX_TOTAL_REWARD_AMOUNT: &[u8] = b"total_reward_amount"; // total_amount for each reward asset, use this to check balance when deposit
static PREFIX_IS_MIGRATED: &[u8] = b"is_migrated";
static PREFIX_REWARD_WEIGHT: &[u8] = b"reward_weight";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: CanonicalAddr,
    pub rewarder: CanonicalAddr,
    pub minter: CanonicalAddr,
    pub oracle_addr: CanonicalAddr,
    pub factory_addr: CanonicalAddr,
    pub base_denom: String,
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolInfo {
    pub staking_token: CanonicalAddr,
    pub pending_reward: Uint128, // not distributed amount due to zero bonding
    pub total_bond_amount: Uint128,
    pub reward_index: Decimal,
    pub migration_params: Option<MigrationParams>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrationParams {
    pub index_snapshot: Decimal,
    pub deprecated_staking_token: CanonicalAddr,
}

pub fn store_pool_info(
    storage: &mut dyn Storage,
    asset_key: &[u8],
    pool_info: &PoolInfo,
) -> StdResult<()> {
    Bucket::new(storage, PREFIX_POOL_INFO).save(asset_key, pool_info)
}

pub fn read_pool_info(storage: &dyn Storage, asset_key: &[u8]) -> StdResult<PoolInfo> {
    ReadonlyBucket::new(storage, PREFIX_POOL_INFO).load(asset_key)
}

pub fn store_total_reward_amount(
    storage: &mut dyn Storage,
    asset_key: &[u8],
    total_amount: &Uint128,
) -> StdResult<()> {
    Bucket::new(storage, PREFIX_TOTAL_REWARD_AMOUNT).save(asset_key, total_amount)
}

pub fn read_total_reward_amount(storage: &dyn Storage, asset_key: &[u8]) -> StdResult<Uint128> {
    ReadonlyBucket::new(storage, PREFIX_TOTAL_REWARD_AMOUNT).load(asset_key)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RewardInfo {
    pub native_token: bool,
    pub index: Decimal,
    pub bond_amount: Uint128,
    pub pending_reward: Uint128,
}

/// returns a bucket with all rewards owned by this owner (query it by owner)
pub fn rewards_store<'a>(
    storage: &'a mut dyn Storage,
    owner: &CanonicalAddr,
) -> Bucket<'a, RewardInfo> {
    Bucket::multilevel(storage, &[PREFIX_REWARD, owner.as_slice()])
}

/// returns a bucket with all rewards owned by this owner (query it by owner)
/// (read-only version for queries)
pub fn rewards_read<'a>(
    storage: &'a dyn Storage,
    owner: &CanonicalAddr,
) -> ReadonlyBucket<'a, RewardInfo> {
    ReadonlyBucket::multilevel(storage, &[PREFIX_REWARD, owner.as_slice()])
}

/// returns a bucket with all stakers belong by this owner (query it by owner)
pub fn stakers_store<'a>(storage: &'a mut dyn Storage, asset_key: &[u8]) -> Bucket<'a, bool> {
    Bucket::multilevel(storage, &[PREFIX_STAKER, asset_key])
}

/// returns a bucket with all rewards owned by this owner (query it by owner)
/// (read-only version for queries)
pub fn stakers_read<'a>(storage: &'a dyn Storage, asset_key: &[u8]) -> ReadonlyBucket<'a, bool> {
    ReadonlyBucket::multilevel(storage, &[PREFIX_STAKER, asset_key])
}

pub fn store_is_migrated(
    storage: &mut dyn Storage,
    asset_key: &[u8],
    staker: &CanonicalAddr,
) -> StdResult<()> {
    Bucket::multilevel(storage, &[PREFIX_IS_MIGRATED, staker.as_slice()]).save(asset_key, &true)
}

pub fn read_is_migrated(storage: &dyn Storage, asset_key: &[u8], staker: &CanonicalAddr) -> bool {
    ReadonlyBucket::multilevel(storage, &[PREFIX_IS_MIGRATED, staker.as_slice()])
        .load(asset_key)
        .unwrap_or(false)
}

pub fn store_reward_weights(
    storage: &mut dyn Storage,
    asset_key: &[u8],
    weights: Vec<AssetInfoRawWeight>,
) -> StdResult<()> {
    let mut weight_bucket: Bucket<Vec<AssetInfoRawWeight>> =
        Bucket::new(storage, PREFIX_REWARD_WEIGHT);
    weight_bucket.save(asset_key, &weights)
}

pub fn read_reward_weights(
    storage: &dyn Storage,
    asset_key: &[u8],
) -> StdResult<Vec<AssetInfoRawWeight>> {
    let weight_bucket: ReadonlyBucket<Vec<AssetInfoRawWeight>> =
        ReadonlyBucket::new(storage, PREFIX_REWARD_WEIGHT);
    weight_bucket.load(asset_key)
}

// upper bound key by 1, for Order::Ascending
pub fn calc_range_start(start_after: Option<Vec<u8>>) -> Option<Vec<u8>> {
    start_after.map(|input| {
        let mut copy = input.clone();
        // zero out all trailing 255, increment first that is not such
        for i in (0..input.len()).rev() {
            if copy[i] == 255 {
                copy[i] = 0;
            } else {
                copy[i] += 1;
                break;
            }
        }
        copy
    })
}
