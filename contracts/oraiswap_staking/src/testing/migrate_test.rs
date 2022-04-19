use crate::migration::{
    migrate_config, migrate_pool_infos, migrate_rewards_store, LegacyConfig, LegacyPoolInfo,
    LegacyRewardInfo, LEGACY_KEY_CONFIG, LEGACY_PREFIX_REWARD,
};
use crate::state::{read_config, read_pool_info, rewards_read, Config, RewardInfo};
use crate::state::{PoolInfo, PREFIX_POOL_INFO};
use cosmwasm_std::{testing::mock_dependencies, Api};
use cosmwasm_std::{CanonicalAddr, Decimal, Storage, Uint128};
use cosmwasm_storage::{singleton, Bucket};
use oraiswap::asset::AssetInfo;
use oraiswap::staking::AmountInfo;

pub fn pool_infos_old_store(storage: &mut dyn Storage) -> Bucket<LegacyPoolInfo> {
    Bucket::new(storage, PREFIX_POOL_INFO)
}

/// returns a bucket with all rewards owned by this owner (query it by owner)
pub fn rewards_store<'a>(
    storage: &'a mut dyn Storage,
    owner: &CanonicalAddr,
) -> Bucket<'a, LegacyRewardInfo> {
    Bucket::multilevel(storage, &[LEGACY_PREFIX_REWARD, owner.as_slice()])
}

#[test]
fn test_migration() {
    let mut deps = mock_dependencies(&[]);
    deps.api.canonical_length = 54;
    let mut legacy_store = pool_infos_old_store(&mut deps.storage);

    let asset_1 = deps.api.canonical_address(&"asset1".into()).unwrap();
    let pool_info_1 = LegacyPoolInfo {
        staking_token: deps.api.canonical_address(&"staking1".into()).unwrap(),
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
    let asset_2 = deps.api.canonical_address(&"asset2".into()).unwrap();
    let pool_info_2 = LegacyPoolInfo {
        staking_token: deps.api.canonical_address(&"staking2".into()).unwrap(),
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

    migrate_pool_infos(deps.as_mut().storage).unwrap();

    let new_pool_info_1: PoolInfo = read_pool_info(deps.as_mut().storage, &asset_1).unwrap();
    let new_pool_info_2: PoolInfo = read_pool_info(deps.as_mut().storage, &asset_2).unwrap();

    assert_eq!(
        new_pool_info_1,
        PoolInfo {
            staking_token: deps.api.canonical_address(&"staking1".into()).unwrap(),
            total_bond_amount: Uint128::from(1u128),
            reward_index: Decimal::percent(1),
            pending_reward: Uint128::from(1u128),
            migration_params: None,
        }
    );
    assert_eq!(
        new_pool_info_2,
        PoolInfo {
            staking_token: deps.api.canonical_address(&"staking2".into()).unwrap(),
            total_bond_amount: Uint128::from(2u128),
            reward_index: Decimal::percent(2),
            pending_reward: Uint128::from(2u128),
            migration_params: None,
        }
    );

    // migrate config
    let legacy_config = LegacyConfig {
        owner: deps
            .api
            .canonical_address(&"orai1g4h64yjt0fvzv5v2j8tyfnpe5kmnetejvfgs7g".into())
            .unwrap(),
        reward_addr: deps
            .api
            .canonical_address(&"orai1lus0f0rhx8s03gdllx2n6vhkmf0536dv57wfge".into())
            .unwrap(),
        minter: deps
            .api
            .canonical_address(&"orai1g4h64yjt0fvzv5v2j8tyfnpe5kmnetejvfgs7g".into())
            .unwrap(),
        oracle_addr: deps
            .api
            .canonical_address(&"orai18rgtdvlrev60plvucw2rz8nmj8pau9gst4q07m".into())
            .unwrap(),
        factory_addr: deps
            .api
            .canonical_address(&"orai1hemdkz4xx9kukgrunxu3yw0nvpyxf34v82d2c8".into())
            .unwrap(),
        base_denom: "orai".into(),
        premium_min_update_interval: 7600u64,
        short_reward_bound: (Decimal::one(), Decimal::one()),
    };
    singleton(&mut deps.storage, LEGACY_KEY_CONFIG)
        .save(&legacy_config)
        .unwrap();

    migrate_config(&mut deps.storage).unwrap();

    let new_config = read_config(&mut deps.storage).unwrap();

    assert_eq!(
        new_config,
        Config {
            owner: deps
                .api
                .canonical_address(&"orai1g4h64yjt0fvzv5v2j8tyfnpe5kmnetejvfgs7g".into())
                .unwrap(),
            rewarder: deps
                .api
                .canonical_address(&"orai1lus0f0rhx8s03gdllx2n6vhkmf0536dv57wfge".into())
                .unwrap(),
            minter: deps
                .api
                .canonical_address(&"orai1g4h64yjt0fvzv5v2j8tyfnpe5kmnetejvfgs7g".into())
                .unwrap(),
            oracle_addr: deps
                .api
                .canonical_address(&"orai18rgtdvlrev60plvucw2rz8nmj8pau9gst4q07m".into())
                .unwrap(),
            factory_addr: deps
                .api
                .canonical_address(&"orai1hemdkz4xx9kukgrunxu3yw0nvpyxf34v82d2c8".into())
                .unwrap(),
            base_denom: "orai".into(),
        }
    );

    // update reward store
    let staker_addr = deps
        .api
        .canonical_address(&"orai1g4h64yjt0fvzv5v2j8tyfnpe5kmnetejvfgs7g".into())
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
            },
        )
        .unwrap();

    // try migrate
    migrate_rewards_store(
        &mut deps.storage,
        &deps.api,
        vec![deps.api.human_address(&staker_addr).unwrap()],
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
        }
    );

    // // try migrate total reward amount
    // migrate_total_reward_amount(
    //     &mut deps.storage,
    //     &mut deps.api,
    //     vec![AmountInfo {
    //         asset_info: AssetInfo::NativeToken {
    //             denom: "orai".into(),
    //         },
    //         amount: Uint128::from(1u64),
    //     }],
    // )
    // .unwrap();

    // let total_reward_amount = read_total_reward_amount(
    //     &mut deps.storage,
    //     AssetInfo::NativeToken {
    //         denom: "orai".into(),
    //     }
    //     .to_raw(&deps.api)
    //     .unwrap()
    //     .as_bytes(),
    // )
    // .unwrap();

    // assert_eq!(total_reward_amount, Uint128::from(1u64));
}
