use cosmwasm_std::{Api, Deps, DepsMut, Order, StdError, StdResult, Storage};
use oraiswap::{error::ContractError, querier::calc_range_start};

use crate::{
    legacy::v1::{
        old_read_all_is_migrated, old_read_is_migrated, old_read_pool_info,
        old_read_rewards_per_sec, old_rewards_read, old_stakers_read,
    },
    state::{
        read_finish_migrate_store_status, rewards_store, stakers_store, store_is_migrated,
        store_pool_info, store_rewards_per_sec,
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

    #[cfg(debug_assertions)]
    let asset_key_string = if let Ok(native_token) = String::from_utf8(asset_key.to_vec()) {
        api.debug(&format!(
            "native {}, lp {}",
            native_token.as_str(),
            staking_token.as_str()
        ));
        native_token
    } else {
        let key = api.addr_humanize(&asset_key.into())?.to_string();
        api.debug(&format!(
            "cw20 {}, lp {}",
            key.as_str(),
            staking_token.as_str()
        ));
        key
    };
    // store reward_per_sec to new new key
    if let Some(rewards_per_sec) = old_read_rewards_per_sec(storage, &asset_key).ok() {
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
        #[cfg(debug_assertions)]
        let is_migrated = old_read_is_migrated(storage, asset_key, staker);
        if asset_key_string
            .eq("ibc/9E4F68298EE0A201969E583100E5F9FAD145BAA900C04ED3B6B302D834D8E3C4")
        {
            api.debug(&format!(
                "staker: {:?}",
                api.addr_humanize(&cosmwasm_std::CanonicalAddr::from(staker.as_slice()))
            ));
        }
        if is_migrated {
            store_is_migrated(storage, &pool_info.staking_token, staker)?;
        };
        stakers_store(storage, &pool_info.staking_token).save(staker, &true)?;
        if let Some(reward) = old_rewards_read(storage, staker).load(asset_key).ok() {
            rewards_store(storage, staker).save(&pool_info.staking_token, &reward)?;
        }
    }
    // get the last staker key from the list
    let last_staker = stakers.last().map(|staker| staker.0.to_owned());
    // increment 1 based on the bytes to process next key
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
