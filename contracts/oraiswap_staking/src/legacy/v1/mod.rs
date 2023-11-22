use cosmwasm_std::{CanonicalAddr, Order, StdResult, Storage};
use cosmwasm_storage::{Bucket, ReadonlyBucket};
use oraiswap::asset::AssetRaw;

use crate::state::{PoolInfo, RewardInfo};

pub static PREFIX_POOL_INFO: &[u8] = b"pool_info_v2";
pub static PREFIX_REWARD: &[u8] = b"reward_v2";
pub static PREFIX_STAKER: &[u8] = b"staker";
pub static PREFIX_IS_MIGRATED: &[u8] = b"is_migrated";
pub static PREFIX_REWARDS_PER_SEC: &[u8] = b"rewards_per_sec";

pub fn old_read_pool_info(storage: &dyn Storage, asset_key: &[u8]) -> StdResult<PoolInfo> {
    ReadonlyBucket::new(storage, PREFIX_POOL_INFO).load(asset_key)
}

pub fn old_read_all_pool_info_keys(storage: &dyn Storage) -> StdResult<Vec<Vec<u8>>> {
    ReadonlyBucket::<PoolInfo>::new(storage, PREFIX_POOL_INFO)
        .range(None, None, cosmwasm_std::Order::Ascending)
        .map(|bucket| bucket.map(|b| b.0))
        .collect()
}
/// returns a bucket with all rewards owned by this staker (query it by staker)
/// (read-only version for queries)
pub fn old_stakers_read<'a>(
    storage: &'a dyn Storage,
    asset_key: &[u8],
) -> ReadonlyBucket<'a, bool> {
    ReadonlyBucket::multilevel(storage, &[PREFIX_STAKER, asset_key])
}

/// returns a bucket with all stakers belong by this staker (query it by staker)
pub fn old_stakers_store<'a>(storage: &'a mut dyn Storage, asset_key: &[u8]) -> Bucket<'a, bool> {
    Bucket::multilevel(storage, &[PREFIX_STAKER, asset_key])
}

// returns a bucket with all rewards owned by this staker (query it by staker)
pub fn old_rewards_store<'a>(
    storage: &'a mut dyn Storage,
    staker: &[u8],
) -> Bucket<'a, RewardInfo> {
    Bucket::multilevel(storage, &[PREFIX_REWARD, staker])
}

/// returns a bucket with all rewards owned by this staker (query it by staker)
/// (read-only version for queries)
pub fn old_rewards_read<'a>(
    storage: &'a dyn Storage,
    staker: &[u8],
) -> ReadonlyBucket<'a, RewardInfo> {
    ReadonlyBucket::multilevel(storage, &[PREFIX_REWARD, staker])
}

/// returns a bucket with all stakers belong by this staker (query it by staker)
pub fn old_stakers_remove<'a>(storage: &'a mut dyn Storage, asset_key: &[u8], staker: &[u8]) -> () {
    Bucket::<CanonicalAddr>::multilevel(storage, &[PREFIX_STAKER, asset_key]).remove(staker)
}

pub fn old_read_is_migrated(storage: &dyn Storage, asset_key: &[u8], staker: &[u8]) -> bool {
    ReadonlyBucket::multilevel(storage, &[PREFIX_IS_MIGRATED, staker])
        .load(asset_key)
        .unwrap_or(false)
}

pub fn old_read_all_is_migrated(
    storage: &dyn Storage,
    staker: &[u8],
) -> StdResult<Vec<(Vec<u8>, bool)>> {
    ReadonlyBucket::multilevel(storage, &[PREFIX_IS_MIGRATED, staker])
        .range(None, None, Order::Ascending)
        .collect::<StdResult<Vec<(Vec<u8>, bool)>>>()
}

pub fn old_remove_store_is_migrated(
    storage: &mut dyn Storage,
    asset_key: &[u8],
    staker: &[u8],
) -> () {
    Bucket::<bool>::multilevel(storage, &[PREFIX_IS_MIGRATED, staker]).remove(&asset_key)
}

pub fn store_rewards_per_sec(
    storage: &mut dyn Storage,
    asset_key: &[u8],
    assets: Vec<AssetRaw>,
) -> StdResult<()> {
    let mut weight_bucket: Bucket<Vec<AssetRaw>> = Bucket::new(storage, PREFIX_REWARDS_PER_SEC);
    weight_bucket.save(asset_key, &assets)
}

pub fn old_read_rewards_per_sec(
    storage: &dyn Storage,
    asset_key: &[u8],
) -> StdResult<Vec<AssetRaw>> {
    let weight_bucket: ReadonlyBucket<Vec<AssetRaw>> =
        ReadonlyBucket::new(storage, PREFIX_REWARDS_PER_SEC);
    weight_bucket.load(asset_key)
}

pub fn old_remove_rewards_per_sec(storage: &mut dyn Storage, asset_key: &[u8]) -> () {
    Bucket::<Vec<AssetRaw>>::new(storage, PREFIX_REWARDS_PER_SEC).remove(asset_key)
}
