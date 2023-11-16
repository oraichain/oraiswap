use crate::contract::{execute, instantiate, query};
use crate::state::{read_pool_info, rewards_read, store_pool_info, PoolInfo, RewardInfo};
use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};
use cosmwasm_std::{coin, from_json, to_json_binary, Addr, Api, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;
use oraiswap::asset::{Asset, AssetInfo, ORAI_DENOM};
use oraiswap::create_entry_points_testing;
use oraiswap::staking::{
    Cw20HookMsg, ExecuteMsg, InstantiateMsg, PoolInfoResponse, QueryMsg, RewardInfoResponse,
    RewardInfoResponseItem, RewardMsg,
};
use oraiswap::testing::{MockApp, ATOM_DENOM};

#[test]
fn test_deposit_reward() {
    let mut deps = mock_dependencies_with_balance(&[
        coin(10000000000u128, ORAI_DENOM),
        coin(20000000000u128, ATOM_DENOM),
    ]);

    let msg = InstantiateMsg {
        owner: Some(Addr::unchecked("owner")),
        rewarder: Addr::unchecked("rewarder"),
        minter: Some(Addr::unchecked("mint")),
        oracle_addr: Addr::unchecked("oracle"),
        factory_addr: Addr::unchecked("factory"),
        base_denom: None,
    };

    let info = mock_info("addr", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // will also add to the index the pending rewards from before the migration
    let msg = ExecuteMsg::UpdateRewardsPerSec {
        staking_token: Addr::unchecked("staking"),
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
        staking_token: Addr::unchecked("staking"),
    };

    let info = mock_info("owner", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let token_raw = deps.api.addr_canonicalize("asset").unwrap();
    let pool_info = read_pool_info(&deps.storage, &token_raw).unwrap();
    store_pool_info(&mut deps.storage, &token_raw, &pool_info).unwrap();

    // bond 100 tokens
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr".into(),
        amount: Uint128::from(100u128),
        msg: to_json_binary(&Cw20HookMsg::Bond {}).unwrap(),
    });
    let info = mock_info("staking", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // factory deposit 100 reward tokens
    let msg = ExecuteMsg::DepositReward {
        rewards: vec![RewardMsg {
            staking_token: Addr::unchecked("asset"),
            total_accumulation_amount: Uint128::from(100u128),
        }],
    };
    let info = mock_info("rewarder", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    // Check pool state
    let res: PoolInfoResponse = from_json(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::PoolInfo {
                staking_token: Addr::unchecked("staking"),
            },
        )
        .unwrap(),
    )
    .unwrap();
    let res_cmp = res.clone();
    assert_eq!(
        res_cmp,
        PoolInfoResponse {
            total_bond_amount: Uint128::from(100u128),
            reward_index: Decimal::from_ratio(100u128, 100u128),
            ..res
        }
    );

    let asset_key = deps.api.addr_canonicalize("asset").unwrap();
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

    let res: PoolInfoResponse = from_json(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::PoolInfo {
                staking_token: Addr::unchecked("staking"),
            },
        )
        .unwrap(),
    )
    .unwrap();
    let res_cmp = res.clone();
    assert_eq!(
        res_cmp,
        PoolInfoResponse {
            total_bond_amount: Uint128::from(100u128),
            reward_index: Decimal::from_ratio(100u128, 100u128),
            ..res
        }
    );
}

#[test]
fn test_deposit_reward_when_no_bonding() {
    let mut deps = mock_dependencies_with_balance(&[
        coin(10000000000u128, ORAI_DENOM),
        coin(20000000000u128, ATOM_DENOM),
    ]);

    let msg = InstantiateMsg {
        owner: Some(Addr::unchecked("owner")),
        rewarder: Addr::unchecked("rewarder"),
        minter: Some(Addr::unchecked("mint")),
        oracle_addr: Addr::unchecked("oracle"),
        factory_addr: Addr::unchecked("factory"),
        base_denom: None,
    };

    let info = mock_info("addr", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // will also add to the index the pending rewards from before the migration
    let msg = ExecuteMsg::UpdateRewardsPerSec {
        staking_token: Addr::unchecked("staking"),
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
        staking_token: Addr::unchecked("staking"),
    };

    let info = mock_info("owner", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let token_raw = deps.api.addr_canonicalize("asset").unwrap();
    let pool_info = read_pool_info(&deps.storage, &token_raw).unwrap();
    store_pool_info(&mut deps.storage, &token_raw, &pool_info).unwrap();

    // factory deposit 100 reward tokens
    let msg = ExecuteMsg::DepositReward {
        rewards: vec![RewardMsg {
            staking_token: Addr::unchecked("asset"),
            total_accumulation_amount: Uint128::from(100u128),
        }],
    };
    let info = mock_info("rewarder", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    // Check pool state
    let res: PoolInfoResponse = from_json(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::PoolInfo {
                staking_token: Addr::unchecked("staking"),
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
            pending_reward: Uint128::from(100u128),
            ..res
        }
    );

    let asset_key = deps.api.addr_canonicalize("asset").unwrap();
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

    let res: PoolInfoResponse = from_json(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::PoolInfo {
                staking_token: Addr::unchecked("staking"),
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
            pending_reward: Uint128::from(100u128),
            ..res
        }
    );
}

#[test]
fn test_before_share_changes() {
    let mut deps = mock_dependencies_with_balance(&[
        coin(10000000000u128, ORAI_DENOM),
        coin(20000000000u128, ATOM_DENOM),
    ]);

    let msg = InstantiateMsg {
        owner: Some(Addr::unchecked("owner")),
        rewarder: Addr::unchecked("rewarder"),
        minter: Some(Addr::unchecked("mint")),
        oracle_addr: Addr::unchecked("oracle"),
        factory_addr: Addr::unchecked("factory"),
        base_denom: None,
    };

    let info = mock_info("addr", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // will also add to the index the pending rewards from before the migration
    let msg = ExecuteMsg::UpdateRewardsPerSec {
        staking_token: Addr::unchecked("staking"),
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
        staking_token: Addr::unchecked("staking"),
    };

    let info = mock_info("owner", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let token_raw = deps.api.addr_canonicalize("asset").unwrap();
    let pool_info = read_pool_info(&deps.storage, &token_raw).unwrap();
    store_pool_info(&mut deps.storage, &token_raw, &pool_info).unwrap();

    // bond 100 tokens
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr".into(),
        amount: Uint128::from(100u128),
        msg: to_json_binary(&Cw20HookMsg::Bond {}).unwrap(),
    });
    let info = mock_info("staking", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::DepositReward {
        rewards: vec![RewardMsg {
            staking_token: Addr::unchecked("asset"),
            total_accumulation_amount: Uint128::from(100u128),
        }],
    };

    let info = mock_info("rewarder", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let asset_key = deps.api.addr_canonicalize("asset").unwrap();
    let addr_raw = deps.api.addr_canonicalize("addr").unwrap();
    let reward_bucket = rewards_read(&deps.storage, &addr_raw);
    let reward_info: RewardInfo = reward_bucket.load(asset_key.as_slice()).unwrap();
    assert_eq!(
        RewardInfo {
            pending_reward: Uint128::zero(),
            bond_amount: Uint128::from(100u128),
            index: Decimal::zero(),
            native_token: false,
            pending_withdraw: vec![],
        },
        reward_info
    );

    // bond 100 more tokens
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr".into(),
        amount: Uint128::from(100u128),
        msg: to_json_binary(&Cw20HookMsg::Bond {}).unwrap(),
    });
    let info = mock_info("staking", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let reward_bucket = rewards_read(&deps.storage, &addr_raw);
    let reward_info: RewardInfo = reward_bucket.load(asset_key.as_slice()).unwrap();
    assert_eq!(
        RewardInfo {
            pending_reward: Uint128::from(100u128),
            bond_amount: Uint128::from(200u128),
            index: Decimal::from_ratio(100u128, 100u128),
            native_token: false,
            pending_withdraw: vec![],
        },
        reward_info
    );

    // factory deposit 100 reward tokens; = 0.8 + 0.4 = 1.2 is reward_index
    let msg = ExecuteMsg::DepositReward {
        rewards: vec![RewardMsg {
            staking_token: Addr::unchecked("asset"),
            total_accumulation_amount: Uint128::from(100u128),
        }],
    };
    let info = mock_info("rewarder", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // unbond
    let msg = ExecuteMsg::Unbond {
        staking_token: Addr::unchecked("staking"),
        amount: Uint128::from(100u128),
    };
    let info = mock_info("addr", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let reward_bucket = rewards_read(&deps.storage, &addr_raw);
    let reward_info: RewardInfo = reward_bucket.load(asset_key.as_slice()).unwrap();
    assert_eq!(
        RewardInfo {
            pending_reward: Uint128::from(200u128),
            bond_amount: Uint128::from(100u128),
            index: Decimal::from_ratio(150u128, 100u128),
            native_token: false,
            pending_withdraw: vec![],
        },
        reward_info
    );
}

#[test]
fn test_withdraw() {
    let mut app = MockApp::new(&[(
        &"addr".to_string(),
        &[
            coin(10000000000u128, ORAI_DENOM),
            coin(20000000000u128, ATOM_DENOM),
        ],
    )]);

    app.set_oracle_contract(Box::new(create_entry_points_testing!(oraiswap_oracle)));

    app.set_token_contract(Box::new(create_entry_points_testing!(oraiswap_token)));

    app.set_factory_and_pair_contract(
        Box::new(
            create_entry_points_testing!(oraiswap_factory)
                .with_reply(oraiswap_factory::contract::reply),
        ),
        Box::new(
            create_entry_points_testing!(oraiswap_pair).with_reply(oraiswap_pair::contract::reply),
        ),
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
        owner: Some(Addr::unchecked("owner")),
        rewarder: reward_addr.clone(),
        minter: Some(Addr::unchecked("mint")),
        oracle_addr: app.oracle_addr.clone(),
        factory_addr: app.factory_addr.clone(),
        base_denom: None,
    };

    let code_id = app.upload(Box::new(create_entry_points_testing!(crate)));

    let staking_addr = app
        .instantiate(code_id, Addr::unchecked("addr"), &msg, &[], "staking")
        .unwrap();

    // funding some balances to the staking contract from rewarder
    app.set_balances_from(
        Addr::unchecked("addr"),
        &[
            (
                &ORAI_DENOM.to_string(),
                &[(&staking_addr.to_string(), &Uint128::from(10000000000u128))],
            ),
            (
                &ATOM_DENOM.to_string(),
                &[(&staking_addr.to_string(), &Uint128::from(20000000000u128))],
            ),
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
        staking_token: Addr::unchecked("staking"),
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
        .execute(Addr::unchecked("owner"), staking_addr.clone(), &msg, &[])
        .unwrap();

    let lp_addr = app.create_token("lptoken");

    let msg = ExecuteMsg::RegisterAsset {
        staking_token: lp_addr.clone(),
    };

    let _res = app
        .execute(Addr::unchecked("owner"), staking_addr.clone(), &msg, &[])
        .unwrap();

    // bond 100 tokens
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr".into(),
        amount: Uint128::from(100u128),
        msg: to_json_binary(&Cw20HookMsg::Bond {}).unwrap(),
    });

    let _res = app
        .execute(lp_addr.clone(), staking_addr.clone(), &msg, &[])
        .unwrap();

    let msg = ExecuteMsg::DepositReward {
        rewards: vec![RewardMsg {
            staking_token: asset_addr.clone(),
            total_accumulation_amount: Uint128::from(100u128),
        }],
    };

    let _res = app
        .execute(reward_addr.clone(), staking_addr.clone(), &msg, &[])
        .unwrap();

    // set allowance
    app.execute(
        Addr::unchecked("addr"),
        asset_addr.clone(),
        &cw20::Cw20ExecuteMsg::IncreaseAllowance {
            spender: staking_addr.to_string(),
            amount: Uint128::from(100u128),
            expires: None,
        },
        &[],
    )
    .unwrap();

    let msg = ExecuteMsg::Withdraw {
        staking_token: Some(Addr::unchecked("staking")),
    };

    let res = app
        .execute(Addr::unchecked("addr"), staking_addr.clone(), &msg, &[])
        .unwrap();

    println!("{:?}", res);
}

#[test]
fn test_update_rewards_per_sec() {
    let mut deps = mock_dependencies_with_balance(&[
        coin(10000000000u128, ORAI_DENOM),
        coin(20000000000u128, ATOM_DENOM),
    ]);

    let msg = InstantiateMsg {
        owner: Some(Addr::unchecked("owner")),
        rewarder: Addr::unchecked("rewarder"),
        minter: Some(Addr::unchecked("mint")),
        oracle_addr: Addr::unchecked("oracle"),
        factory_addr: Addr::unchecked("factory"),
        base_denom: None,
    };

    let info = mock_info("addr", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // will also add to the index the pending rewards from before the migration
    let msg = ExecuteMsg::UpdateRewardsPerSec {
        staking_token: Addr::unchecked("staking"),
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
        staking_token: Addr::unchecked("staking"),
    };

    let info = mock_info("owner", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let token_raw = deps.api.addr_canonicalize("asset").unwrap();
    let pool_info = read_pool_info(&deps.storage, &token_raw).unwrap();
    store_pool_info(&mut deps.storage, &token_raw, &pool_info).unwrap();

    // bond 100 tokens
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr".into(),
        amount: Uint128::from(300u128),
        msg: to_json_binary(&Cw20HookMsg::Bond {}).unwrap(),
    });
    let info = mock_info("staking", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // factory deposit 300 reward tokens
    let msg = ExecuteMsg::DepositReward {
        rewards: vec![RewardMsg {
            staking_token: Addr::unchecked("asset"),
            total_accumulation_amount: Uint128::from(300u128),
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
            staking_token: Addr::unchecked("staking"),
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
        rewards: vec![RewardMsg {
            staking_token: Addr::unchecked("asset"),
            total_accumulation_amount: Uint128::from(100u128),
        }],
    };
    let info = mock_info("rewarder", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    // Check reward info, pending reward should be zero because of withdrawal
    let data = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::RewardInfo {
            staking_token: None,
            staker_addr: Addr::unchecked("addr"),
        },
    )
    .unwrap();
    let res: RewardInfoResponse = from_json(&data).unwrap();
    assert_eq!(
        res,
        RewardInfoResponse {
            staker_addr: Addr::unchecked("addr"),
            reward_infos: vec![RewardInfoResponseItem {
                staking_token: Addr::unchecked("staking"),
                bond_amount: Uint128::from(300u128),
                pending_reward: Uint128::from(99u128),
                pending_withdraw: vec![
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: ORAI_DENOM.to_string()
                        },
                        amount: Uint128::from(99u128)
                    },
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: ATOM_DENOM.to_string()
                        },
                        amount: Uint128::from(199u128)
                    }
                ],
                should_migrate: None,
            },],
        }
    );
}

#[test]
fn test_update_rewards_per_sec_with_multiple_bond() {
    let mut deps = mock_dependencies_with_balance(&[
        coin(10000000000u128, ORAI_DENOM),
        coin(20000000000u128, ATOM_DENOM),
    ]);

    let msg = InstantiateMsg {
        owner: Some(Addr::unchecked("owner")),
        rewarder: Addr::unchecked("rewarder"),
        minter: Some(Addr::unchecked("mint")),
        oracle_addr: Addr::unchecked("oracle"),
        factory_addr: Addr::unchecked("factory"),
        base_denom: None,
    };

    let info = mock_info("addr", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // will also add to the index the pending rewards from before the migration
    let msg = ExecuteMsg::UpdateRewardsPerSec {
        staking_token: Addr::unchecked("staking"),
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
        staking_token: Addr::unchecked("staking"),
    };

    let info = mock_info("owner", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let token_raw = deps.api.addr_canonicalize("asset").unwrap();
    let pool_info = read_pool_info(&deps.storage, &token_raw).unwrap();
    store_pool_info(&mut deps.storage, &token_raw, &pool_info).unwrap();

    // bond 100 tokens
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr".into(),
        amount: Uint128::from(300u128),
        msg: to_json_binary(&Cw20HookMsg::Bond {}).unwrap(),
    });
    let info = mock_info("staking", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // factory deposit 300 reward tokens
    let msg = ExecuteMsg::DepositReward {
        rewards: vec![RewardMsg {
            staking_token: Addr::unchecked("asset"),
            total_accumulation_amount: Uint128::from(300u128),
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
            staking_token: Addr::unchecked("staking"),
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
        amount: Uint128::from(300u128),
        msg: to_json_binary(&Cw20HookMsg::Bond {}).unwrap(),
    });
    let info = mock_info("staking", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let data = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::RewardInfo {
            staking_token: None,
            staker_addr: Addr::unchecked("addr1"),
        },
    )
    .unwrap();
    let res: RewardInfoResponse = from_json(&data).unwrap();
    assert_eq!(
        res,
        RewardInfoResponse {
            staker_addr: Addr::unchecked("addr1"),
            reward_infos: vec![RewardInfoResponseItem {
                staking_token: Addr::unchecked("staking"),
                bond_amount: Uint128::from(300u128),
                pending_reward: Uint128::zero(),
                pending_withdraw: vec![],
                should_migrate: None,
            },],
        }
    );

    // factory deposit 100 reward tokens
    let msg = ExecuteMsg::DepositReward {
        rewards: vec![RewardMsg {
            staking_token: Addr::unchecked("asset"),
            total_accumulation_amount: Uint128::from(100u128),
        }],
    };
    let info = mock_info("rewarder", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    // Check reward info, pending reward should be zero because of withdrawal
    let data = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::RewardInfo {
            staking_token: None,
            staker_addr: Addr::unchecked("addr"),
        },
    )
    .unwrap();
    let res: RewardInfoResponse = from_json(&data).unwrap();
    assert_eq!(
        res,
        RewardInfoResponse {
            staker_addr: Addr::unchecked("addr"),
            reward_infos: vec![RewardInfoResponseItem {
                staking_token: Addr::unchecked("staking"),
                bond_amount: Uint128::from(300u128),
                pending_reward: Uint128::from(49u128),
                pending_withdraw: vec![
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: ORAI_DENOM.to_string()
                        },
                        amount: Uint128::from(99u128)
                    },
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: ATOM_DENOM.to_string()
                        },
                        amount: Uint128::from(199u128)
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
            staking_token: None,
            staker_addr: Addr::unchecked("addr1"),
        },
    )
    .unwrap();
    let res: RewardInfoResponse = from_json(&data).unwrap();
    assert_eq!(
        res,
        RewardInfoResponse {
            staker_addr: Addr::unchecked("addr1"),
            reward_infos: vec![RewardInfoResponseItem {
                staking_token: Addr::unchecked("staking"),
                bond_amount: Uint128::from(300u128),
                pending_reward: Uint128::from(49u128),
                pending_withdraw: vec![],
                should_migrate: None,
            },],
        }
    );
}
