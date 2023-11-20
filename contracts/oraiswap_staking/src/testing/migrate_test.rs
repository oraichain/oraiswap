use crate::contract::{migrate_store, query_pool_info};
use crate::migration::{
    migrate_asset_keys_to_lp_tokens, migrate_single_asset_key_to_lp_token, old_rewards_read,
    old_rewards_store, old_stakers_read, old_stakers_store,
};
use crate::state::{
    read_all_pool_info_keys, read_all_pool_infos, read_pool_info, read_rewards_per_sec,
    rewards_read, stakers_read, store_pool_info, PoolInfo, RewardInfo,
};

use cosmwasm_testing_util::mock::{load_state, mock_dependencies};

use cosmwasm_std::{Addr, Decimal, Order, Uint128};
use cosmwasm_std::{Api, DepsMut};
use oraiswap::asset::AssetInfo;

const MAINET_STATE_BYTES: &[u8] = include_bytes!("./mainnet.state");
const MILKY_CONTRACT: &str = "orai1gzvndtzceqwfymu2kqhta2jn6gmzxvzqwdgvjw";

#[test]
fn test_forked_mainnet() {
    let mut deps = mock_dependencies();
    let DepsMut { api, storage, .. } = deps.as_mut();
    load_state(storage, MAINET_STATE_BYTES);

    let staking_token = read_pool_info(storage, &api.addr_canonicalize(MILKY_CONTRACT).unwrap())
        .unwrap()
        .staking_token;

    migrate_asset_keys_to_lp_tokens(storage, api).unwrap();

    let pool_info = query_pool_info(
        deps.as_ref(),
        deps.api.addr_humanize(&staking_token).unwrap(),
    )
    .unwrap();

    println!("milky pool {:?}", pool_info);
}

// #[test]
// fn test_migration() {
//     // fixture
//     let mut deps = mock_dependencies();
//     let first_asset_info = AssetInfo::NativeToken {
//         denom: "orai".to_string(),
//     };
//     let first_old_asset_key = deps
//         .api
//         .addr_canonicalize(&first_asset_info.to_string())
//         .unwrap();
//     let second_asset_info = AssetInfo::Token {
//         contract_addr: Addr::unchecked("airi"),
//     };
//     let second_old_asset_key = deps
//         .api
//         .addr_canonicalize(&second_asset_info.to_string())
//         .unwrap();
//
//     let first_staking_token = Addr::unchecked("staking1");
//     let second_staking_token = Addr::unchecked("staking2");
//     let first_staking_canon = deps
//         .api
//         .addr_canonicalize(first_staking_token.as_str())
//         .unwrap();
//     let second_staking_canon = deps
//         .api
//         .addr_canonicalize(second_staking_token.as_str())
//         .unwrap();
//
//     let deps_mut = deps.as_mut();
//     let storage = deps_mut.storage;
//
//     // populate fake data, can change to 100 if want
//     for n in 0..2u64 {
//         let amount = Uint128::from(n);
//         // let staker = deps_mut
//         //     .api
//         //     .addr_canonicalize(format!("staker{:?}", n.to_string().as_str()).as_str())
//         //     .unwrap();
//         let (asset_key, staking_token, _is_store_migrated) = if n == 0 {
//             (
//                 first_old_asset_key.clone(),
//                 first_staking_canon.clone(),
//                 true,
//             )
//         } else {
//             (
//                 second_old_asset_key.clone(),
//                 second_staking_canon.clone(),
//                 false,
//             )
//         };
//         let pool_info = PoolInfo {
//             staking_token: staking_token.clone(),
//             pending_reward: amount.clone(),
//             total_bond_amount: amount.clone(),
//             reward_index: Decimal::zero(),
//             migration_params: None,
//         };
//         store_pool_info(storage, &asset_key, &pool_info).unwrap();
//         // stakers_store(storage, &asset_key)
//         //     .save(&staker, &true)
//         //     .unwrap();
//         // if n / 2 == 0 {
//         //     rewards_store(storage, &staker)
//         //         .save(
//         //             &asset_key,
//         //             &RewardInfo {
//         //                 native_token: true,
//         //                 index: Decimal::zero(),
//         //                 bond_amount: amount.clone(),
//         //                 pending_reward: amount.clone(),
//         //                 pending_withdraw: vec![],
//         //             },
//         //         )
//         //         .unwrap();
//         // }
//         // if is_store_migrated {
//         //     store_is_migrated(storage, &asset_key, &staker).unwrap();
//         // }
//         // if n / 2 != 0 {
//         //     store_rewards_per_sec(
//         //         storage,
//         //         &asset_key,
//         //         vec![AssetRaw {
//         //             info: AssetInfoRaw::NativeToken {
//         //                 denom: "atom".to_string(),
//         //             },
//         //             amount: amount.clone(),
//         //         }],
//         //     )
//         //     .unwrap();
//         // }
//     }
//
//     // check asset keys. They should match with our old asset keys set above
//     let pool_info_keys = read_all_pool_info_keys(storage).unwrap();
//     assert_eq!(pool_info_keys.len(), 2);
//     assert_eq!(pool_info_keys.contains(&first_old_asset_key.to_vec()), true);
//     assert_eq!(
//         pool_info_keys.contains(&second_old_asset_key.to_vec()),
//         true
//     );
//
//     // action
//     migrate_asset_keys_to_lp_tokens(storage, deps_mut.api).unwrap();
//
//     // assert
//     // query to see if the stores have been migrated successfully
//     // the keys should be staking1 and staking2
//     let pool_info_keys = read_all_pool_info_keys(storage).unwrap();
//     // should have 4 keys because we dont delete old keys in the migrate msg, and two new keys are added
//     assert_eq!(pool_info_keys.len(), 4);
//     assert_eq!(
//         pool_info_keys.contains(&first_staking_canon.clone().to_vec()),
//         true
//     );
//     assert_eq!(
//         pool_info_keys.contains(&second_staking_canon.clone().to_vec()),
//         true
//     );
//     // keys already deleted
//     assert_eq!(pool_info_keys.contains(&first_old_asset_key.to_vec()), true);
//     assert_eq!(
//         pool_info_keys.contains(&second_old_asset_key.to_vec()),
//         true
//     );
//
//     // for n in 0..2u64 {
//     //     let amount = Uint128::from(n);
//     //     let staker = deps_mut
//     //         .api
//     //         .addr_canonicalize(format!("staker{:?}", n.to_string().as_str()).as_str())
//     //         .unwrap();
//
//     //     let (staking_token, is_store_migrated) = if n == 0 {
//     //         (first_staking_canon.clone(), true)
//     //     } else {
//     //         (second_staking_canon.clone(), false)
//     //     };
//     //     assert_eq!(
//     //         stakers_read(storage, &staking_token).load(&staker).unwrap(),
//     //         true
//     //     );
//
//     //     assert_eq!(
//     //         read_is_migrated(storage, &staking_token, &staker),
//     //         is_store_migrated
//     //     );
//
//     //     if n / 2 == 0 {
//     //         assert_eq!(
//     //             rewards_read(storage, &staker)
//     //                 .load(&staking_token)
//     //                 .unwrap()
//     //                 .bond_amount,
//     //             amount.clone()
//     //         );
//     //     }
//
//     //     if n / 2 != 0 {
//     //         assert_eq!(
//     //             read_rewards_per_sec(storage, &staking_token).unwrap().len(),
//     //             1
//     //         );
//     //     }
//     // }
// }
//
// #[test]
// fn test_migrate_store() {
//     // fixture
//     let mut deps = mock_dependencies();
//     let deps_mut = deps.as_mut();
//     let storage = deps_mut.storage;
//     let amount = Uint128::from(10u64);
//     let first_asset_info = AssetInfo::NativeToken {
//         denom: "orai".to_string(),
//     };
//     let old_asset_key = first_asset_info.to_vec(deps_mut.api).unwrap();
//
//     let staking_token = Addr::unchecked("staking1");
//     let staking_token_canon = deps_mut
//         .api
//         .addr_canonicalize(staking_token.as_str())
//         .unwrap();
//
//     let stakers_addr = Addr::unchecked("staker");
//
//     let pool_info = PoolInfo {
//         staking_token: staking_token_canon.clone(),
//         pending_reward: amount.clone(),
//         total_bond_amount: amount.clone(),
//         reward_index: Decimal::zero(),
//         migration_params: None,
//     };
//     store_pool_info(storage, &old_asset_key, &pool_info).unwrap();
//     // store both keys old and new so we can test migrating the store
//     store_pool_info(storage, &staking_token_canon, &pool_info).unwrap();
//
//     old_stakers_store(storage, &old_asset_key)
//         .save(stakers_addr.as_bytes(), &true)
//         .unwrap();
//
//     let rewards_info = RewardInfo {
//         native_token: true,
//         index: Decimal::zero(),
//         bond_amount: amount.clone(),
//         pending_reward: amount.clone(),
//         pending_withdraw: vec![],
//     };
//     old_rewards_store(storage, stakers_addr.as_bytes())
//         .save(&old_asset_key, &rewards_info)
//         .unwrap();
//
//     // check asset keys. They should match with our old asset keys set above
//     let pool_info_keys = read_all_pool_info_keys(storage).unwrap();
//     assert_eq!(pool_info_keys.len(), 2);
//     assert_eq!(pool_info_keys.contains(&old_asset_key.to_vec()), true);
//     assert_eq!(pool_info_keys.contains(&staking_token_canon.to_vec()), true);
//
//     // action
//     migrate_store(storage, deps_mut.api, first_asset_info, None).unwrap();
//
//     // assert
//     // query to see if the stores have been migrated successfully
//     // the keys should be staking1
//     let pool_info_keys = read_all_pool_info_keys(storage).unwrap();
//     // we have fully removed all data related to the old key, so the pool info length should be 1
//     assert_eq!(pool_info_keys.len(), 1);
//     assert_eq!(
//         pool_info_keys.last().unwrap().to_owned(),
//         staking_token_canon.to_vec()
//     );
//
//     assert_eq!(
//         old_stakers_read(storage, &old_asset_key)
//             .load(stakers_addr.as_bytes())
//             .is_ok(),
//         false
//     );
//     assert_eq!(
//         stakers_read(storage, &staking_token_canon)
//             .load(stakers_addr.as_bytes())
//             .is_ok(),
//         true
//     );
//
//     assert_eq!(
//         old_rewards_read(storage, stakers_addr.as_bytes())
//             .load(&old_asset_key)
//             .is_ok(),
//         false
//     );
//     assert_eq!(
//         rewards_read(storage, stakers_addr.as_bytes())
//             .load(&staking_token_canon)
//             .is_ok(),
//         true
//     );
// }

#[test]
fn test_migrate_single_asset_key_to_lp_token() {
    let mut deps = mock_dependencies();
    let deps_mut = deps.as_mut();
    load_state(deps_mut.storage, MAINET_STATE_BYTES);
    let limit = Some(1000);

    let asset_keys = read_all_pool_infos(deps_mut.storage).unwrap();

    for (asset_key, pool_info) in asset_keys.iter() {
        migrate_single_asset_key_to_lp_token(
            deps_mut.storage,
            deps_mut.api,
            asset_key.as_slice(),
            None,
            limit,
        )
        .unwrap();
        for item in old_stakers_read(deps_mut.as_ref().storage, asset_key)
            .range(None, None, Order::Ascending)
            .take(1000)
        {
            let (staker, ..) = item.unwrap();

            // assert staker in new asset_key
            assert!(
                stakers_read(deps_mut.as_ref().storage, &pool_info.staking_token)
                    .load(staker.as_slice())
                    .unwrap()
            );

            // assert reward in new asset_key
            if let Some(old_reward) = old_rewards_read(deps_mut.as_ref().storage, staker.as_slice())
                .load(asset_key.as_slice())
                .ok()
            {
                let new_reward = rewards_read(deps_mut.as_ref().storage, staker.as_slice())
                    .load(&pool_info.staking_token)
                    .unwrap();

                assert_eq!(old_reward, new_reward);
            }
        }
        // assert pool_info in with new key
        assert_eq!(
            *pool_info,
            read_pool_info(deps_mut.as_ref().storage, &pool_info.staking_token).unwrap()
        );

        // assert reward_per_sec
        if let Some(rewards_per_sec) =
            read_rewards_per_sec(deps_mut.as_ref().storage, &asset_key).ok()
        {
            assert_eq!(
                rewards_per_sec,
                read_rewards_per_sec(deps_mut.as_ref().storage, &pool_info.staking_token).unwrap()
            )
        }
    }
}
