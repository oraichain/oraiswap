use cosmwasm_std::{Api, Deps, DepsMut, Order, StdError, StdResult, Storage};
use oraiswap::{error::ContractError, querier::calc_range_start};

use crate::{
    legacy::v1::{
        old_read_all_is_migrated, old_read_is_migrated, old_read_pool_info,
        old_read_rewards_per_sec, old_rewards_read, old_stakers_read,
    },
    state::{
        read_finish_migrate_store_status, read_is_migrated, read_pool_info, rewards_store,
        stakers_store, store_is_migrated, store_pool_info, store_rewards_per_sec,
    },
};

pub fn migrate_single_asset_key_to_lp_token(
    storage: &mut dyn Storage,
    api: &dyn Api,
    asset_key: &[u8],
) -> StdResult<u64> {
    let pool_info = old_read_pool_info(storage, asset_key)?;
    // store pool_info to new key
    store_pool_info(storage, &pool_info.staking_token, &pool_info)?;
    let staking_token = api.addr_humanize(&pool_info.staking_token)?;

    let asset_key_string = if let Ok(native_token) = String::from_utf8(asset_key.to_vec()) {
        #[cfg(debug_assertions)]
        api.debug(&format!(
            "native {}, lp {}",
            native_token.as_str(),
            staking_token.as_str()
        ));
        native_token
    } else {
        let key = api.addr_humanize(&asset_key.into())?.to_string();

        #[cfg(debug_assertions)]
        api.debug(&format!(
            "cw20 {}, lp {}",
            key.as_str(),
            staking_token.as_str()
        ));
        key
    };
    // store reward_per_sec to new new key
    if let Ok(rewards_per_sec) = old_read_rewards_per_sec(storage, &asset_key) {
        #[cfg(debug_assertions)]
        api.debug(&format!("rewards_per_sec {:?}", rewards_per_sec));
        store_rewards_per_sec(storage, &pool_info.staking_token, rewards_per_sec)?;
    }

    let stakers = old_stakers_read(storage, asset_key)
        .range(None, None, Order::Ascending)
        // Get next_key
        .collect::<StdResult<Vec<(Vec<u8>, bool)>>>()?;

    #[cfg(debug_assertions)]
    api.debug(&format!("stakers.len {:?} ", stakers.len()));

    // Store stakers to new staking key token
    for (staker, _) in stakers.iter() {
        let all_is_migrated = old_read_all_is_migrated(storage, staker)?;
        for (old_asset_key, old_is_migrated) in all_is_migrated {
            let old_pool_info = old_read_pool_info(storage, &old_asset_key)?;
            let new_is_migrated = read_is_migrated(storage, &old_pool_info.staking_token, staker);
            if old_is_migrated && !new_is_migrated {
                store_is_migrated(storage, &old_pool_info.staking_token, staker)?;
            }
        }
        stakers_store(storage, &pool_info.staking_token).save(staker, &true)?;
        if let Some(reward) = old_rewards_read(storage, staker).load(asset_key).ok() {
            rewards_store(storage, staker).save(&pool_info.staking_token, &reward)?;
        }
    }
    Ok(stakers.len() as u64)
}

pub fn validate_migrate_store_status(storage: &mut dyn Storage) -> StdResult<()> {
    let migrate_store_status = read_finish_migrate_store_status(storage)?;
    if migrate_store_status {
        return Ok(());
    }
    Err(StdError::generic_err(
        ContractError::ContractUpgrade {}.to_string(),
    ))
}
