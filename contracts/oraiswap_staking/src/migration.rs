use cosmwasm_std::{CanonicalAddr, Decimal, Order, StdResult, Storage, Uint128};
use cosmwasm_storage::Bucket;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::{PoolInfo, PREFIX_POOL_INFO};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LegacyPoolInfo {
    pub staking_token: CanonicalAddr,
    pub pending_reward: Uint128,
    pub short_pending_reward: Uint128,
    pub total_bond_amount: Uint128,
    pub total_short_amount: Uint128,
    pub reward_index: Decimal,
    pub short_reward_index: Decimal,
    pub premium_rate: Decimal,
    pub short_reward_weight: Decimal,
    pub premium_updated_time: u64,
}

pub fn migrate_pool_infos(storage: &mut dyn Storage) -> StdResult<()> {
    let mut legacy_pool_infos_bucket: Bucket<LegacyPoolInfo> =
        Bucket::new(storage, PREFIX_POOL_INFO);

    let mut pools: Vec<(CanonicalAddr, LegacyPoolInfo)> = vec![];
    for item in legacy_pool_infos_bucket.range(None, None, Order::Ascending) {
        let (k, p) = item?;
        pools.push((CanonicalAddr::from(k), p));
    }

    for (asset, _) in pools.clone().into_iter() {
        legacy_pool_infos_bucket.remove(asset.as_slice());
    }

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
