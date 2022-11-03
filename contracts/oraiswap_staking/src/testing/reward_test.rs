use crate::contract::{handle, init, query};
use crate::state::{read_pool_info, rewards_read, store_pool_info, PoolInfo, RewardInfo};
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{coin, from_binary, to_binary, Api, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;
use oraiswap::asset::{Asset, AssetInfo, ORAI_DENOM};
use oraiswap::mock_app::{MockApp, ATOM_DENOM};
use oraiswap::staking::{
    Cw20HookMsg, ExecuteMsg, InstantiateMsg, PoolInfoResponse, QueryMsg, RewardInfoResponse,
    RewardInfoResponseItem,
};

#[test]
fn test_deposit_reward() {
    let mut deps = mock_dependencies(&[
        coin(10000000000u128, ORAI_DENOM),
        coin(20000000000u128, ATOM_DENOM),
    ]);

    let msg = InstantiateMsg {
        owner: Some("owner".into()),
        rewarder: "rewarder".into(),
        minter: Some("mint".into()),
        oracle_addr: "oracle".into(),
        factory_addr: "factory".into(),
        base_denom: None,
    };

    let info = mock_info("addr", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // will also add to the index the pending rewards from before the migration
    let msg = ExecuteMsg::UpdateRewardsPerSec {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        assets: vec![
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: 100u128.into(),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ATOM_DENOM.to_string(),
                },
                amount: 200u128.into(),
            },
        ],
    };
    let info = mock_info("owner", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::RegisterAsset {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        staking_token: "staking".into(),
    };

    let info = mock_info("owner", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let token_raw = deps.api.addr_canonicalize(&"asset".into()).unwrap();
    let pool_info = read_pool_info(&deps.storage, &token_raw).unwrap();
    store_pool_info(&mut deps.storage, &token_raw, &pool_info).unwrap();

    // bond 100 tokens
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
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
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // factory deposit 100 reward tokens
    let msg = ExecuteMsg::DepositReward {
        rewards: vec![Asset {
            info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
            amount: Uint128(100u128),
        }],
    };
    let info = mock_info("rewarder", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    // Check pool state
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
            reward_index: Decimal::from_ratio(100u128, 100u128),
            ..res
        }
    );

    let asset_key = deps.api.addr_canonicalize(&"asset".into()).unwrap();
    let pool_info: PoolInfo = read_pool_info(&deps.storage, &asset_key).unwrap();
    store_pool_info(
        &mut deps.storage,
        &asset_key,
        &PoolInfo {
            reward_index: Decimal::zero(),
            ..pool_info
        },
    )
    .unwrap();

    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

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
            reward_index: Decimal::from_ratio(100u128, 100u128),
            ..res
        }
    );
}

#[test]
fn test_deposit_reward_when_no_bonding() {
    let mut deps = mock_dependencies(&[
        coin(10000000000u128, ORAI_DENOM),
        coin(20000000000u128, ATOM_DENOM),
    ]);

    let msg = InstantiateMsg {
        owner: Some("owner".into()),
        rewarder: "rewarder".into(),
        minter: Some("mint".into()),
        oracle_addr: "oracle".into(),
        factory_addr: "factory".into(),
        base_denom: None,
    };

    let info = mock_info("addr", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // will also add to the index the pending rewards from before the migration
    let msg = ExecuteMsg::UpdateRewardsPerSec {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        assets: vec![
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: 100u128.into(),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ATOM_DENOM.to_string(),
                },
                amount: 200u128.into(),
            },
        ],
    };
    let info = mock_info("owner", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::RegisterAsset {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        staking_token: "staking".into(),
    };

    let info = mock_info("owner", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let token_raw = deps.api.addr_canonicalize(&"asset".into()).unwrap();
    let pool_info = read_pool_info(&deps.storage, &token_raw).unwrap();
    store_pool_info(&mut deps.storage, &token_raw, &pool_info).unwrap();

    // factory deposit 100 reward tokens
    let msg = ExecuteMsg::DepositReward {
        rewards: vec![Asset {
            info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
            amount: Uint128(100u128),
        }],
    };
    let info = mock_info("rewarder", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    // Check pool state
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
            reward_index: Decimal::zero(),
            pending_reward: Uint128(100u128),
            ..res
        }
    );

    let asset_key = deps.api.addr_canonicalize(&"asset".into()).unwrap();
    let pool_info: PoolInfo = read_pool_info(&deps.storage, &asset_key).unwrap();
    store_pool_info(
        &mut deps.storage,
        &asset_key,
        &PoolInfo {
            pending_reward: Uint128::zero(),
            ..pool_info
        },
    )
    .unwrap();

    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

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
            reward_index: Decimal::zero(),
            pending_reward: Uint128(100u128),
            ..res
        }
    );
}

#[test]
fn test_before_share_changes() {
    let mut deps = mock_dependencies(&[
        coin(10000000000u128, ORAI_DENOM),
        coin(20000000000u128, ATOM_DENOM),
    ]);

    let msg = InstantiateMsg {
        owner: Some("owner".into()),
        rewarder: "rewarder".into(),
        minter: Some("mint".into()),
        oracle_addr: "oracle".into(),
        factory_addr: "factory".into(),
        base_denom: None,
    };

    let info = mock_info("addr", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // will also add to the index the pending rewards from before the migration
    let msg = ExecuteMsg::UpdateRewardsPerSec {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        assets: vec![
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: 100u128.into(),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ATOM_DENOM.to_string(),
                },
                amount: 200u128.into(),
            },
        ],
    };
    let info = mock_info("owner", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::RegisterAsset {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        staking_token: "staking".into(),
    };

    let info = mock_info("owner", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let token_raw = deps.api.addr_canonicalize(&"asset".into()).unwrap();
    let pool_info = read_pool_info(&deps.storage, &token_raw).unwrap();
    store_pool_info(&mut deps.storage, &token_raw, &pool_info).unwrap();

    // bond 100 tokens
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
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
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::DepositReward {
        rewards: vec![Asset {
            info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
            amount: Uint128(100u128),
        }],
    };

    let info = mock_info("rewarder", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let asset_key = deps.api.addr_canonicalize(&"asset".into()).unwrap();
    let addr_raw = deps.api.addr_canonicalize(&"addr".into()).unwrap();
    let reward_bucket = rewards_read(&deps.storage, &addr_raw);
    let reward_info: RewardInfo = reward_bucket.load(asset_key.as_slice()).unwrap();
    assert_eq!(
        RewardInfo {
            pending_reward: Uint128::zero(),
            bond_amount: Uint128(100u128),
            index: Decimal::zero(),
            native_token: false,
            pending_withdraw: vec![],
        },
        reward_info
    );

    // bond 100 more tokens
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
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
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let reward_bucket = rewards_read(&deps.storage, &addr_raw);
    let reward_info: RewardInfo = reward_bucket.load(asset_key.as_slice()).unwrap();
    assert_eq!(
        RewardInfo {
            pending_reward: Uint128(100u128),
            bond_amount: Uint128(200u128),
            index: Decimal::from_ratio(100u128, 100u128),
            native_token: false,
            pending_withdraw: vec![],
        },
        reward_info
    );

    // factory deposit 100 reward tokens; = 0.8 + 0.4 = 1.2 is reward_index
    let msg = ExecuteMsg::DepositReward {
        rewards: vec![Asset {
            info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
            amount: Uint128(100u128),
        }],
    };
    let info = mock_info("rewarder", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // unbond
    let msg = ExecuteMsg::Unbond {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        amount: Uint128(100u128),
    };
    let info = mock_info("addr", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let reward_bucket = rewards_read(&deps.storage, &addr_raw);
    let reward_info: RewardInfo = reward_bucket.load(asset_key.as_slice()).unwrap();
    assert_eq!(
        RewardInfo {
            pending_reward: Uint128(200u128),
            bond_amount: Uint128(100u128),
            index: Decimal::from_ratio(150u128, 100u128),
            native_token: false,
            pending_withdraw: vec![],
        },
        reward_info
    );
}

#[test]
fn test_withdraw() {
    let mut app = MockApp::new();

    app.set_oracle_contract(oraiswap_oracle::testutils::contract());

    app.set_token_contract(oraiswap_token::testutils::contract());

    app.set_factory_and_pair_contract(
        oraiswap_factory::testutils::contract(),
        oraiswap_pair::testutils::contract(),
    );

    app.set_balance(
        "addr".into(),
        &[
            coin(10000000000u128, ORAI_DENOM),
            coin(20000000000u128, ATOM_DENOM),
        ],
    );

    let asset_addr = app.create_token("asset");
    let reward_addr = app.create_token("reward");

    // update other contract token balance
    app.set_token_balances(&[
        (
            &"reward".to_string(),
            &[(&"addr".to_string(), &Uint128::from(10000000000u128))],
        ),
        (
            &"asset".to_string(),
            &[(&"addr".to_string(), &Uint128::from(10000000000u128))],
        ),
    ]);

    let msg = InstantiateMsg {
        owner: Some("owner".into()),
        rewarder: reward_addr.clone(),
        minter: Some("mint".into()),
        oracle_addr: app.oracle_addr.clone(),
        factory_addr: app.factory_addr.clone(),
        base_denom: None,
    };

    let code_id = app.upload(crate::testutils::contract());

    let staking_addr = app
        .instantiate(code_id, "addr".into(), &msg, &[], "staking")
        .unwrap();

    // funding some balances to the staking contract from rewarder
    app.set_balance(
        staking_addr.clone(),
        &[
            coin(10000000000u128, ORAI_DENOM),
            coin(20000000000u128, ATOM_DENOM),
        ],
    );
    app.set_token_balances(&[
        (
            &"reward".to_string(),
            &[(&staking_addr.to_string(), &Uint128::from(10000000000u128))],
        ),
        (
            &"asset".to_string(),
            &[(&staking_addr.to_string(), &Uint128::from(10000000000u128))],
        ),
    ]);

    // will also add to the index the pending rewards from before the migration
    let msg = ExecuteMsg::UpdateRewardsPerSec {
        asset_info: AssetInfo::Token {
            contract_addr: asset_addr.clone(),
        },
        assets: vec![
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: 100u128.into(),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ATOM_DENOM.to_string(),
                },
                amount: 200u128.into(),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: reward_addr.clone(),
                },
                amount: 200u128.into(),
            },
        ],
    };

    let _res = app
        .execute("owner".into(), staking_addr.clone(), &msg, &[])
        .unwrap();

    let lp_addr = app.create_token("lptoken");

    let msg = ExecuteMsg::RegisterAsset {
        asset_info: AssetInfo::Token {
            contract_addr: asset_addr.clone(),
        },
        staking_token: lp_addr.clone(),
    };

    let _res = app
        .execute("owner".into(), staking_addr.clone(), &msg, &[])
        .unwrap();

    // bond 100 tokens
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr".into(),
        amount: Uint128(100u128),
        msg: to_binary(&Cw20HookMsg::Bond {
            asset_info: AssetInfo::Token {
                contract_addr: asset_addr.clone(),
            },
        })
        .ok(),
    });

    let _res = app
        .execute(lp_addr.clone(), staking_addr.clone(), &msg, &[])
        .unwrap();

    let msg = ExecuteMsg::DepositReward {
        rewards: vec![Asset {
            info: AssetInfo::Token {
                contract_addr: asset_addr.clone(),
            },
            amount: Uint128(100u128),
        }],
    };

    let _res = app
        .execute(reward_addr.clone(), staking_addr.clone(), &msg, &[])
        .unwrap();

    // set allowance
    app.execute(
        "addr".into(),
        asset_addr.clone(),
        &oraiswap_token::msg::ExecuteMsg::IncreaseAllowance {
            spender: staking_addr.clone(),
            amount: Uint128::from(100u128),
            expires: None,
        },
        &[],
    )
    .unwrap();

    let msg = ExecuteMsg::Withdraw {
        asset_info: Some(AssetInfo::Token {
            contract_addr: asset_addr.clone(),
        }),
    };

    let res = app
        .execute("addr".into(), staking_addr.clone(), &msg, &[])
        .unwrap();

    println!("{:?}", res);
}

#[test]
fn test_update_rewards_per_sec() {
    let mut deps = mock_dependencies(&[
        coin(10000000000u128, ORAI_DENOM),
        coin(20000000000u128, ATOM_DENOM),
    ]);

    let msg = InstantiateMsg {
        owner: Some("owner".into()),
        rewarder: "rewarder".into(),
        minter: Some("mint".into()),
        oracle_addr: "oracle".into(),
        factory_addr: "factory".into(),
        base_denom: None,
    };

    let info = mock_info("addr", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // will also add to the index the pending rewards from before the migration
    let msg = ExecuteMsg::UpdateRewardsPerSec {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        assets: vec![
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: 100u128.into(),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ATOM_DENOM.to_string(),
                },
                amount: 200u128.into(),
            },
        ],
    };
    let info = mock_info("owner", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::RegisterAsset {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        staking_token: "staking".into(),
    };

    let info = mock_info("owner", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let token_raw = deps.api.addr_canonicalize(&"asset".into()).unwrap();
    let pool_info = read_pool_info(&deps.storage, &token_raw).unwrap();
    store_pool_info(&mut deps.storage, &token_raw, &pool_info).unwrap();

    // bond 100 tokens
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr".into(),
        amount: Uint128(300u128),
        msg: to_binary(&Cw20HookMsg::Bond {
            asset_info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
        })
        .ok(),
    });
    let info = mock_info("staking", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // factory deposit 300 reward tokens
    let msg = ExecuteMsg::DepositReward {
        rewards: vec![Asset {
            info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
            amount: Uint128(300u128),
        }],
    };
    let info = mock_info("rewarder", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    // change rewards per second for the pool
    let _res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::UpdateRewardsPerSec {
            asset_info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
            assets: vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    amount: 33u128.into(),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: ATOM_DENOM.to_string(),
                    },
                    amount: 67u128.into(),
                },
            ],
        },
    )
    .unwrap();

    // factory deposit 100 reward tokens
    let msg = ExecuteMsg::DepositReward {
        rewards: vec![Asset {
            info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
            amount: Uint128(100u128),
        }],
    };
    let info = mock_info("rewarder", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    // Check reward info, pending reward should be zero because of withdrawal
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
                bond_amount: Uint128(300u128),
                pending_reward: Uint128(99u128),
                pending_withdraw: vec![
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: ORAI_DENOM.to_string()
                        },
                        amount: Uint128(99)
                    },
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: ATOM_DENOM.to_string()
                        },
                        amount: Uint128(199)
                    }
                ],
                should_migrate: None,
            },],
        }
    );
}

#[test]
fn test_update_rewards_per_sec_with_multiple_bond() {
    let mut deps = mock_dependencies(&[
        coin(10000000000u128, ORAI_DENOM),
        coin(20000000000u128, ATOM_DENOM),
    ]);

    let msg = InstantiateMsg {
        owner: Some("owner".into()),
        rewarder: "rewarder".into(),
        minter: Some("mint".into()),
        oracle_addr: "oracle".into(),
        factory_addr: "factory".into(),
        base_denom: None,
    };

    let info = mock_info("addr", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // will also add to the index the pending rewards from before the migration
    let msg = ExecuteMsg::UpdateRewardsPerSec {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        assets: vec![
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: 100u128.into(),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ATOM_DENOM.to_string(),
                },
                amount: 200u128.into(),
            },
        ],
    };
    let info = mock_info("owner", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::RegisterAsset {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        staking_token: "staking".into(),
    };

    let info = mock_info("owner", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let token_raw = deps.api.addr_canonicalize(&"asset".into()).unwrap();
    let pool_info = read_pool_info(&deps.storage, &token_raw).unwrap();
    store_pool_info(&mut deps.storage, &token_raw, &pool_info).unwrap();

    // bond 100 tokens
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr".into(),
        amount: Uint128(300u128),
        msg: to_binary(&Cw20HookMsg::Bond {
            asset_info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
        })
        .ok(),
    });
    let info = mock_info("staking", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // factory deposit 300 reward tokens
    let msg = ExecuteMsg::DepositReward {
        rewards: vec![Asset {
            info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
            amount: Uint128(300u128),
        }],
    };
    let info = mock_info("rewarder", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    // change rewards per second for the pool
    let _res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::UpdateRewardsPerSec {
            asset_info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
            assets: vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    amount: 33u128.into(),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: ATOM_DENOM.to_string(),
                    },
                    amount: 67u128.into(),
                },
            ],
        },
    )
    .unwrap();

    // bond 100 tokens
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr1".into(),
        amount: Uint128(300u128),
        msg: to_binary(&Cw20HookMsg::Bond {
            asset_info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
        })
        .ok(),
    });
    let info = mock_info("staking", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let data = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::RewardInfo {
            asset_info: None,
            staker_addr: "addr1".into(),
        },
    )
    .unwrap();
    let res: RewardInfoResponse = from_binary(&data).unwrap();
    assert_eq!(
        res,
        RewardInfoResponse {
            staker_addr: "addr1".into(),
            reward_infos: vec![RewardInfoResponseItem {
                asset_info: AssetInfo::Token {
                    contract_addr: "asset".into()
                },
                bond_amount: Uint128(300u128),
                pending_reward: Uint128::zero(),
                pending_withdraw: vec![],
                should_migrate: None,
            },],
        }
    );

    // factory deposit 100 reward tokens
    let msg = ExecuteMsg::DepositReward {
        rewards: vec![Asset {
            info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
            amount: Uint128(100u128),
        }],
    };
    let info = mock_info("rewarder", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    // Check reward info, pending reward should be zero because of withdrawal
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
                bond_amount: Uint128(300u128),
                pending_reward: Uint128(49u128),
                pending_withdraw: vec![
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: ORAI_DENOM.to_string()
                        },
                        amount: Uint128(99)
                    },
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: ATOM_DENOM.to_string()
                        },
                        amount: Uint128(199)
                    }
                ],
                should_migrate: None,
            },],
        }
    );

    // Check reward info, pending reward should be zero because of withdrawal for addr1
    let data = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::RewardInfo {
            asset_info: None,
            staker_addr: "addr1".into(),
        },
    )
    .unwrap();
    let res: RewardInfoResponse = from_binary(&data).unwrap();
    assert_eq!(
        res,
        RewardInfoResponse {
            staker_addr: "addr1".into(),
            reward_infos: vec![RewardInfoResponseItem {
                asset_info: AssetInfo::Token {
                    contract_addr: "asset".into()
                },
                bond_amount: Uint128(300u128),
                pending_reward: Uint128(49u128),
                pending_withdraw: vec![],
                should_migrate: None,
            },],
        }
    );
}
