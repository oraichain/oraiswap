use crate::migration::{migrate_pool_infos, LegacyPoolInfo};
use crate::state::read_pool_info;
use crate::state::{PoolInfo, PREFIX_POOL_INFO};
use cosmwasm_std::{testing::mock_dependencies, Api};
use cosmwasm_std::{Decimal, Storage, Uint128};
use cosmwasm_storage::Bucket;

pub fn pool_infos_old_store(storage: &mut dyn Storage) -> Bucket<LegacyPoolInfo> {
    Bucket::new(storage, PREFIX_POOL_INFO)
}

#[test]
fn test_pool_infos_migration() {
    let mut deps = mock_dependencies(&[]);
    let mut legacy_store = pool_infos_old_store(&mut deps.storage);

    let asset_1 = deps.api.canonical_address(&"asset1".into()).unwrap();
    let pool_info_1 = LegacyPoolInfo {
        staking_token: deps.api.canonical_address(&"staking1".into()).unwrap(),
        total_bond_amount: Uint128::from(1u128),
        total_short_amount: Uint128::from(1u128),
        reward_index: Decimal::percent(1),
        pending_reward: Uint128::from(1u128),
    };
    let asset_2 = deps.api.canonical_address(&"asset2".into()).unwrap();
    let pool_info_2 = LegacyPoolInfo {
        staking_token: deps.api.canonical_address(&"staking2".into()).unwrap(),
        total_bond_amount: Uint128::from(2u128),
        total_short_amount: Uint128::from(2u128),
        reward_index: Decimal::percent(2),
        pending_reward: Uint128::from(2u128),
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
    )
}
