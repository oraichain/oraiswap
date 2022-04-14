use crate::contract::{handle, init, query};
use crate::state::{read_pool_info, store_pool_info};
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{coin, from_binary, to_binary, Api, Decimal, StdError, Uint128, WasmMsg};
use cw20::{Cw20HandleMsg, Cw20ReceiveMsg};
use oraiswap::asset::{Asset, AssetInfo, ORAI_DENOM};
use oraiswap::mock_querier::ATOM_DENOM;
use oraiswap::staking::{
    AssetInfoWeight, Cw20HookMsg, HandleMsg, InitMsg, PoolInfoResponse, QueryMsg,
    RewardInfoResponse, RewardInfoResponseItem,
};

#[test]
fn test_deprecate() {
    let mut deps = mock_dependencies(&[
        coin(10000000000u128, ORAI_DENOM),
        coin(20000000000u128, ATOM_DENOM),
    ]);

    let msg = InitMsg {
        owner: Some("owner".into()),
        rewarder: "rewarder".into(),
        minter: Some("mint".into()),
        oracle_addr: "oracle".into(),
        factory_addr: "factory".into(),
        base_denom: None,
    };

    let info = mock_info("addr", &[]);
    let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = HandleMsg::RegisterAsset {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        staking_token: "staking".into(),
    };

    let info = mock_info("owner", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    let asset_key = deps.api.canonical_address(&"asset".into()).unwrap();
    let pool_info = read_pool_info(&deps.storage, &asset_key).unwrap();
    store_pool_info(&mut deps.storage, &asset_key, &pool_info).unwrap();

    // set reward weights for asset
    // will also add to the index the pending rewards from before the migration
    let msg = HandleMsg::UpdateRewardWeights {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        weights: vec![
            AssetInfoWeight {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                weight: 100,
            },
            AssetInfoWeight {
                info: AssetInfo::NativeToken {
                    denom: ATOM_DENOM.to_string(),
                },
                weight: 200,
            },
        ],
    };
    let info = mock_info("owner", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    // bond 100 tokens
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: "addr".into(),
        amount: Uint128(100u128),
        msg: to_binary(&Cw20HookMsg::Bond {
            asset_info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
        })
        .ok(),
    });
    let info = mock_info("staking", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    // owner of reward contract deposit 100 reward tokens
    // distribute weight => 80:20
    let msg = HandleMsg::DepositReward {
        rewards: vec![Asset {
            info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
            amount: Uint128(100u128),
        }],
    };
    let info = mock_info("rewarder", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    // query pool and reward info
    let res: PoolInfoResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::PoolInfo {
                asset_info: AssetInfo::Token {
                    contract_addr: "asset".into(),
                },
            },
        )
        .unwrap(),
    )
    .unwrap();
    let res_cmp = res.clone();
    assert_eq!(
        res_cmp,
        PoolInfoResponse {
            total_bond_amount: Uint128(100u128),
            reward_index: Decimal::from_ratio(80u128, 100u128),
            migration_index_snapshot: None,
            migration_deprecated_staking_token: None,
            ..res
        }
    );
    let data = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::RewardInfo {
            asset_info: None,
            staker_addr: "addr".into(),
        },
    )
    .unwrap();
    let res: RewardInfoResponse = from_binary(&data).unwrap();
    assert_eq!(
        res,
        RewardInfoResponse {
            staker_addr: "addr".into(),
            reward_infos: vec![RewardInfoResponseItem {
                asset_info: AssetInfo::Token {
                    contract_addr: "asset".into()
                },
                bond_amount: Uint128(100u128),
                pending_reward: Uint128(80u128),
                should_migrate: None,
            }],
        }
    );

    // handle deprecate
    let msg = HandleMsg::DeprecateStakingToken {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        new_staking_token: "new_staking".into(),
    };
    let info = mock_info("owner", &[]);
    handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    // deposit more rewards
    let msg = HandleMsg::DepositReward {
        rewards: vec![Asset {
            info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
            amount: Uint128(100u128),
        }],
    };
    let info = mock_info("rewarder", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    // query again
    let res: PoolInfoResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::PoolInfo {
                asset_info: AssetInfo::Token {
                    contract_addr: "asset".into(),
                },
            },
        )
        .unwrap(),
    )
    .unwrap();
    let res_cmp = res.clone();
    assert_eq!(
        res_cmp,
        PoolInfoResponse {
            staking_token: "new_staking".into(),
            total_bond_amount: Uint128::zero(), // reset
            reward_index: Decimal::from_ratio(80u128, 100u128), // stays the same
            migration_index_snapshot: Some(Decimal::from_ratio(80u128, 100u128)),
            migration_deprecated_staking_token: Some("staking".into()),
            pending_reward: Uint128(80u128), // new reward waiting here
            ..res
        }
    );
    let data = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::RewardInfo {
            asset_info: None,
            staker_addr: "addr".into(),
        },
    )
    .unwrap();
    let res: RewardInfoResponse = from_binary(&data).unwrap();
    assert_eq!(
        res,
        RewardInfoResponse {
            staker_addr: "addr".into(),
            reward_infos: vec![RewardInfoResponseItem {
                asset_info: AssetInfo::Token {
                    contract_addr: "asset".into()
                },
                bond_amount: Uint128(100u128),
                pending_reward: Uint128(80u128), // did not change
                should_migrate: Some(true),      // non-short pos should migrate
            }],
        }
    );

    // try to bond new or old staking token, should fail both
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: "addr".into(),
        amount: Uint128(100u128),
        msg: to_binary(&Cw20HookMsg::Bond {
            asset_info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
        })
        .ok(),
    });
    let info = mock_info("staking", &[]);
    let err = handle(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err("The staking token for this asset has been migrated to new_staking")
    );
    let info = mock_info("new_staking", &[]);
    let err = handle(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err("The LP token for this asset has been deprecated, withdraw all your deprecated tokens to migrate your position")
    );

    // unbond all the old tokens
    let msg = HandleMsg::Unbond {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        amount: Uint128(100u128),
    };
    let info = mock_info("addr", &[]);
    let res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
    // make sure that we are receiving deprecated lp tokens tokens
    assert_eq!(
        res.messages,
        vec![WasmMsg::Execute {
            contract_addr: "staking".into(),
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: "addr".into(),
                amount: Uint128(100u128),
            })
            .unwrap(),
            send: vec![],
        }
        .into()]
    );
    let data = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::RewardInfo {
            asset_info: None,
            staker_addr: "addr".into(),
        },
    )
    .unwrap();
    let res: RewardInfoResponse = from_binary(&data).unwrap();
    assert_eq!(
        res,
        RewardInfoResponse {
            staker_addr: "addr".into(),
            reward_infos: vec![RewardInfoResponseItem {
                asset_info: AssetInfo::Token {
                    contract_addr: "asset".into()
                },
                bond_amount: Uint128::zero(),
                pending_reward: Uint128(80u128), // still the same
                should_migrate: None,            // now its back to empty
            },],
        }
    );

    // now can bond the new staking token
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: "addr".into(),
        amount: Uint128(100u128),
        msg: to_binary(&Cw20HookMsg::Bond {
            asset_info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
        })
        .ok(),
    });
    let info = mock_info("new_staking", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    // deposit new rewards
    // will also add to the index the pending rewards from before the migration
    let msg = HandleMsg::DepositReward {
        rewards: vec![Asset {
            info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
            amount: Uint128(100u128),
        }],
    };
    let info = mock_info("rewarder", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    // expect to have 80 * 3 rewards
    // initial + deposit after deprecation + deposit after bonding again
    let data = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::RewardInfo {
            asset_info: None,
            staker_addr: "addr".into(),
        },
    )
    .unwrap();
    let res: RewardInfoResponse = from_binary(&data).unwrap();
    assert_eq!(
        res,
        RewardInfoResponse {
            staker_addr: "addr".into(),
            reward_infos: vec![RewardInfoResponseItem {
                asset_info: AssetInfo::Token {
                    contract_addr: "asset".into()
                },
                bond_amount: Uint128(100u128),
                pending_reward: Uint128(240u128), // 80 * 3
                should_migrate: None,
            },],
        }
    );

    // completely new users can bond
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: "newaddr".into(),
        amount: Uint128(100u128),
        msg: to_binary(&Cw20HookMsg::Bond {
            asset_info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
        })
        .ok(),
    });
    let info = mock_info("new_staking", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    let data = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::RewardInfo {
            asset_info: None,
            staker_addr: "newaddr".into(),
        },
    )
    .unwrap();
    let res: RewardInfoResponse = from_binary(&data).unwrap();
    assert_eq!(
        res,
        RewardInfoResponse {
            staker_addr: "newaddr".into(),
            reward_infos: vec![RewardInfoResponseItem {
                asset_info: AssetInfo::Token {
                    contract_addr: "asset".into()
                },
                bond_amount: Uint128(100u128),
                pending_reward: Uint128::zero(),
                should_migrate: None,
            },],
        }
    );
}
