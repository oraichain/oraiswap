use cosmwasm_std::{Api, CanonicalAddr, Decimal, HumanAddr, Order, StdResult, Storage, Uint128};
use cosmwasm_storage::ReadonlyBucket;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::{rewards_store, MigrationParams, RewardInfo, PREFIX_REWARD};

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
    ReadonlyBucket::multilevel(storage, &[PREFIX_REWARD, owner.as_slice()])
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
                pending_withdraw: vec![],
            };
            rewards_store(store, &staker_addr).save(&asset_key, &new_reward_info)?;
        }
    }

    Ok(())
}
