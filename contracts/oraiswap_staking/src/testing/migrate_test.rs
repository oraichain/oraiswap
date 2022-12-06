use crate::migration::{migrate_rewards_store, LegacyPoolInfo, LegacyRewardInfo};
use crate::state::PREFIX_POOL_INFO;
use crate::state::{rewards_read, RewardInfo, PREFIX_REWARD};
use cosmwasm_std::{testing::mock_dependencies, Api};
use cosmwasm_std::{CanonicalAddr, Decimal, Storage, Uint128};
use cosmwasm_storage::Bucket;

pub fn pool_infos_old_store(storage: &mut dyn Storage) -> Bucket<LegacyPoolInfo> {
    Bucket::new(storage, PREFIX_POOL_INFO)
}

/// returns a bucket with all rewards owned by this owner (query it by owner)
pub fn rewards_store<'a>(
    storage: &'a mut dyn Storage,
    owner: &CanonicalAddr,
) -> Bucket<'a, LegacyRewardInfo> {
    Bucket::multilevel(storage, &[PREFIX_REWARD, owner.as_slice()])
}

#[test]
fn test_migration() {
    let mut deps = mock_dependencies();
    let mut legacy_store = pool_infos_old_store(&mut deps.storage);

    let asset_1 = deps.api.addr_canonicalize("asset1").unwrap();
    let pool_info_1 = LegacyPoolInfo {
        staking_token: deps.api.addr_canonicalize("staking1").unwrap(),
        total_bond_amount: Uint128::from(1u128),
        total_short_amount: Uint128::from(1u128),
        reward_index: Decimal::percent(1),
        short_reward_index: Decimal::percent(1),
        pending_reward: Uint128::from(1u128),
        short_pending_reward: Uint128::from(1u128),
        premium_rate: Decimal::percent(1),
        short_reward_weight: Decimal::percent(1),
        premium_updated_time: 1,
        migration_params: None,
    };
    let asset_2 = deps.api.addr_canonicalize("asset2").unwrap();
    let pool_info_2 = LegacyPoolInfo {
        staking_token: deps.api.addr_canonicalize("staking2").unwrap(),
        total_bond_amount: Uint128::from(2u128),
        total_short_amount: Uint128::from(2u128),
        reward_index: Decimal::percent(2),
        short_reward_index: Decimal::percent(2),
        pending_reward: Uint128::from(2u128),
        short_pending_reward: Uint128::from(2u128),
        premium_rate: Decimal::percent(2),
        short_reward_weight: Decimal::percent(2),
        premium_updated_time: 2,
        migration_params: None,
    };

    legacy_store.save(asset_1.as_slice(), &pool_info_1).unwrap();
    legacy_store.save(asset_2.as_slice(), &pool_info_2).unwrap();

    // update reward store
    let staker_addr = deps
        .api
        .addr_canonicalize("orai1g4h64yjt0fvzv5v2j8tyfnpe5kmnetejvfgs7g")
        .unwrap();
    let asset_key = "foobar".as_bytes();
    // store legacy reward info
    rewards_store(&mut deps.storage, &staker_addr)
        .save(
            asset_key,
            &LegacyRewardInfo {
                index: Decimal::one(),
                bond_amount: Uint128::from(1000u64),
                pending_reward: Uint128::from(500u64),
                native_token: false,
            },
        )
        .unwrap();

    // try migrate
    migrate_rewards_store(
        &mut deps.storage,
        &deps.api,
        vec![deps.api.addr_humanize(&staker_addr).unwrap()],
    )
    .unwrap();

    let new_reward = rewards_read(&mut deps.storage, &staker_addr);
    let reward_info = new_reward.load(asset_key).unwrap();

    assert_eq!(
        reward_info,
        RewardInfo {
            index: Decimal::one(),
            bond_amount: Uint128::from(1000u64),
            pending_reward: Uint128::from(500u64),
            native_token: false,
            pending_withdraw: vec![],
        }
    );
}
