use cosmwasm_std::{CanonicalAddr, Order, StdResult, Storage};

use crate::state::{
    read_all_pool_info_keys, read_is_migrated, read_pool_info, read_rewards_per_sec,
    remove_pool_info, remove_rewards_per_sec, remove_store_is_migrated, rewards_read,
    rewards_store, stakers_read, stakers_remove, stakers_store, store_is_migrated, store_pool_info,
    store_rewards_per_sec,
};

pub fn migrate_asset_keys_to_lp_tokens(storage: &mut dyn Storage) -> StdResult<()> {
    let asset_keys = read_all_pool_info_keys(storage)?;
    for asset_key in asset_keys {
        let pool_info = read_pool_info(storage, &asset_key)?;
        // store new pool info with new staking token key
        store_pool_info(storage, &pool_info.staking_token, &pool_info)?;
        remove_pool_info(storage, &asset_key);
        let stakers = stakers_read(storage, &asset_key)
            .range(None, None, Order::Ascending)
            .map(|item| {
                let (k, _) = item?;
                Ok(CanonicalAddr::from(k))
            })
            .collect::<StdResult<Vec<CanonicalAddr>>>()?;

        // process each staker's map
        for staker in stakers {
            // first thing first, we update ≈our stakers list mapped with the old asset key
            stakers_store(storage, &pool_info.staking_token).save(&staker, &true)?;
            stakers_remove(storage, &asset_key, &staker);

            let rewards_bucket = rewards_read(storage, &staker).load(&asset_key)?;
            // update new key for our reward bucket
            rewards_store(storage, &staker).save(&pool_info.staking_token, &rewards_bucket)?;
            // remove old key
            rewards_store(storage, &staker).remove(&asset_key);

            let is_store_migrated = read_is_migrated(storage, &asset_key, &staker);
            if is_store_migrated {
                // new asset key is our lp token, we wont be using asset_info no more, so we need to update our store to a new key
                store_is_migrated(storage, &pool_info.staking_token, &staker)?;
                // remove old key
                remove_store_is_migrated(storage, &asset_key, &staker);
            }
        }

        // our final map, rewards per sec
        let rewards_per_sec = read_rewards_per_sec(storage, &asset_key)?;
        store_rewards_per_sec(storage, &pool_info.staking_token, rewards_per_sec)?;
        remove_rewards_per_sec(storage, &asset_key);
    }

    Ok(())
}
