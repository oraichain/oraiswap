use cosmwasm_std::{Api, CanonicalAddr, Order, StdResult, Storage};
use cosmwasm_storage::{Bucket, ReadonlyBucket};

use crate::state::{
    read_all_pool_info_keys, read_all_pool_infos, read_is_migrated, read_pool_info,
    read_rewards_per_sec, remove_pool_info, remove_rewards_per_sec, remove_store_is_migrated,
    rewards_read, rewards_store, stakers_read, stakers_remove, stakers_store, store_is_migrated,
    store_pool_info, store_rewards_per_sec, RewardInfo,
};

pub static PREFIX_REWARD: &[u8] = b"reward_v2";
static PREFIX_STAKER: &[u8] = b"staker";
static PREFIX_IS_MIGRATED: &[u8] = b"is_migrated";

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

pub fn old_remove_store_is_migrated(
    storage: &mut dyn Storage,
    asset_key: &[u8],
    staker: &[u8],
) -> () {
    Bucket::<bool>::multilevel(storage, &[PREFIX_IS_MIGRATED, staker]).remove(&asset_key)
}

pub fn migrate_asset_keys_to_lp_tokens(storage: &mut dyn Storage, api: &dyn Api) -> StdResult<()> {
    let pools = read_all_pool_infos(storage)?;
    for (asset_key, pool_info) in pools {
        let staking_token = api.addr_humanize(&pool_info.staking_token)?;
        if let Ok(native_token) = String::from_utf8(asset_key.clone()) {
            api.debug(&format!(
                "native {}, lp {}",
                native_token.as_str(),
                staking_token.as_str()
            ));
        } else {
            api.debug(&format!(
                "cw20 {}, lp {}",
                api.addr_humanize(&asset_key.clone().into())?.as_str(),
                staking_token.as_str()
            ));
        }

        store_pool_info(storage, pool_info.staking_token.as_slice(), &pool_info)?;
        remove_pool_info(storage, &asset_key);

        let stakers = stakers_read(storage, &asset_key)
            .range(None, None, Order::Ascending)
            .collect::<StdResult<Vec<(Vec<u8>, bool)>>>()?;

        // process each staker's map
        let mut ind = 0;
        for (staker, _) in stakers {
            if let Ok(staker_addr) = api.addr_humanize(&staker.clone().into()) {
                api.debug(&format!("staker {} {}", ind, staker_addr.as_str()));
                ind += 1;
            }
            // first thing first, we update ≈our stakers list mapped with the old asset key
            stakers_store(storage, &pool_info.staking_token).save(&staker, &true)?;
            stakers_remove(storage, &asset_key, &staker);
            if let Ok(rewards_bucket) = rewards_read(storage, &staker).load(&asset_key) {
                // update new key for our reward bucket
                rewards_store(storage, &staker).save(&pool_info.staking_token, &rewards_bucket)?;
                // remove old key
                rewards_store(storage, &staker).remove(&asset_key);
            }

            if read_is_migrated(storage, &asset_key, &staker) {
                // new asset key is our lp token, we wont be using asset_info no more, so we need to update our store to a new key
                store_is_migrated(storage, &pool_info.staking_token, &staker)?;
                // remove old key
                remove_store_is_migrated(storage, &asset_key, &staker);
            }
        }

        // our final map, rewards per sec
        if let Some(rewards_per_sec) = read_rewards_per_sec(storage, &asset_key).ok() {
            store_rewards_per_sec(storage, &pool_info.staking_token, rewards_per_sec)?;
            remove_rewards_per_sec(storage, &asset_key);
        }
    }
    Ok(())
}
