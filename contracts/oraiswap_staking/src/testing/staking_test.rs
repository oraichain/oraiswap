use crate::contract::{handle, init, query};
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{
    attr, coin, from_binary, to_binary, BankMsg, Coin, CosmosMsg, Decimal, StdError, Uint128,
    WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use oraiswap::asset::{Asset, AssetInfo, PairInfo, ORAI_DENOM};
use oraiswap::mock_app::{MockApp, ATOM_DENOM};
use oraiswap::staking::{
    Cw20HookMsg, ExecuteMsg, InstantiateMsg, PoolInfoResponse, QueryMsg, RewardInfoResponse,
    RewardInfoResponseItem,
};

#[test]
fn test_bond_tokens() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        owner: Some("owner".into()),
        rewarder: "reward".into(),
        minter: Some("mint".into()),
        oracle_addr: "oracle".into(),
        factory_addr: "factory".into(),
        base_denom: None,
    };

    let info = mock_info("addr", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::RegisterAsset {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        staking_token: "staking".into(),
    };

    let info = mock_info("owner", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

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
    let data = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::RewardInfo {
            asset_info: Some(AssetInfo::Token {
                contract_addr: "asset".into(),
            }),
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
                pending_reward: Uint128::zero(),
                pending_withdraw: vec![],
                bond_amount: Uint128(100u128),
                should_migrate: None,
            }],
        }
    );

    let data = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::PoolInfo {
            asset_info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
        },
    )
    .unwrap();

    let pool_info: PoolInfoResponse = from_binary(&data).unwrap();
    assert_eq!(
        pool_info,
        PoolInfoResponse {
            asset_info: AssetInfo::Token {
                contract_addr: "asset".into()
            },
            staking_token: "staking".into(),
            total_bond_amount: Uint128(100u128),
            reward_index: Decimal::zero(),
            pending_reward: Uint128::zero(),
            migration_deprecated_staking_token: None,
            migration_index_snapshot: None,
        }
    );

    // bond 100 more tokens from other account
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr2".into(),
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

    let data = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::PoolInfo {
            asset_info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
        },
    )
    .unwrap();
    let pool_info: PoolInfoResponse = from_binary(&data).unwrap();
    assert_eq!(
        pool_info,
        PoolInfoResponse {
            asset_info: AssetInfo::Token {
                contract_addr: "asset".into()
            },
            staking_token: "staking".into(),
            total_bond_amount: Uint128(200u128),
            reward_index: Decimal::zero(),
            pending_reward: Uint128::zero(),
            migration_deprecated_staking_token: None,
            migration_index_snapshot: None,
        }
    );

    // failed with unauthorized
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

    let info = mock_info("staking2", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "unauthorized"),
        _ => panic!("Must return unauthorized error"),
    }
}

#[test]
fn test_unbond() {
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

    // register asset
    let msg = ExecuteMsg::RegisterAsset {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        staking_token: "staking".into(),
    };

    let info = mock_info("owner", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

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
            amount: Uint128(300u128),
        }],
    };
    let info = mock_info("rewarder", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

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
                amount: 100u128.into(),
            },
        ],
    };
    let info = mock_info("owner", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // unbond 150 tokens; failed
    let msg = ExecuteMsg::Unbond {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        amount: Uint128(150u128),
    };

    let info = mock_info("addr", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    match res {
        StdError::GenericErr { msg, .. } => {
            assert_eq!(msg, "Cannot unbond more than bond amount");
        }
        _ => panic!("Must return generic error"),
    };

    // normal unbond
    let msg = ExecuteMsg::Unbond {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        amount: Uint128(100u128),
    };

    let info = mock_info("addr", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            WasmMsg::Execute {
                contract_addr: "staking".into(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "addr".into(),
                    amount: Uint128(100u128),
                })
                .unwrap(),
                send: vec![],
            }
            .into(),
            CosmosMsg::Bank(BankMsg::Send {
                from_address: "cosmos2contract".into(),
                to_address: "addr".into(),
                amount: vec![coin(99u128, ORAI_DENOM)],
            })
            .into(),
            CosmosMsg::Bank(BankMsg::Send {
                from_address: "cosmos2contract".into(),
                to_address: "addr".into(),
                amount: vec![coin(199u128, ATOM_DENOM)],
            })
            .into()
        ]
    );

    let data = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::PoolInfo {
            asset_info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
        },
    )
    .unwrap();
    let pool_info: PoolInfoResponse = from_binary(&data).unwrap();
    assert_eq!(
        pool_info,
        PoolInfoResponse {
            asset_info: AssetInfo::Token {
                contract_addr: "asset".into()
            },
            staking_token: "staking".into(),
            total_bond_amount: Uint128::zero(),
            reward_index: Decimal::from_ratio(300u128, 100u128),
            pending_reward: Uint128::zero(),
            migration_deprecated_staking_token: None,
            migration_index_snapshot: None,
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
            reward_infos: vec![],
        }
    );
}

#[test]
fn test_auto_stake() {
    let mut app = MockApp::new();

    app.set_oracle_contract(oraiswap_oracle::testutils::contract());

    app.set_token_contract(oraiswap_token::testutils::contract());

    app.set_factory_and_pair_contract(
        oraiswap_factory::testutils::contract(),
        oraiswap_pair::testutils::contract(),
    );

    app.set_balance("addr".into(), &[coin(10000000000u128, ORAI_DENOM)]);

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

    let asset_infos = [
        AssetInfo::NativeToken {
            denom: ORAI_DENOM.to_string(),
        },
        AssetInfo::Token {
            contract_addr: asset_addr.clone(),
        },
    ];

    // create pair
    let pair_addr = app.set_pair(asset_infos.clone()).unwrap();

    let pair_info: PairInfo = app
        .query(pair_addr.clone(), &oraiswap::pair::QueryMsg::Pair {})
        .unwrap();

    // set allowance
    app.execute(
        "addr".into(),
        asset_addr.clone(),
        &oraiswap_token::msg::ExecuteMsg::IncreaseAllowance {
            spender: pair_addr.clone(),
            amount: Uint128::from(100u128),
            expires: None,
        },
        &[],
    )
    .unwrap();

    // provide liquidity
    // successfully provide liquidity for the exist pool
    let msg = oraiswap::pair::ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(100u128),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: asset_addr.clone(),
                },
                amount: Uint128::from(100u128),
            },
        ],
        slippage_tolerance: None,
        receiver: None,
    };

    let _res = app
        .execute(
            "addr".into(),
            pair_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(100u128),
            }],
        )
        .unwrap();

    let code_id = app.upload(crate::testutils::contract());

    let msg = InstantiateMsg {
        owner: Some("owner".into()),
        rewarder: reward_addr.clone(),
        minter: Some("mint".into()),
        oracle_addr: app.oracle_addr.clone(),
        factory_addr: app.factory_addr.clone(),
        base_denom: None,
    };

    let staking_addr = app
        .instantiate(code_id, "addr".into(), &msg, &[], "staking")
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

    let msg = ExecuteMsg::RegisterAsset {
        asset_info: AssetInfo::Token {
            contract_addr: asset_addr.clone(),
        },
        staking_token: pair_info.liquidity_token.clone(),
    };

    let _res = app
        .execute("owner".into(), staking_addr.clone(), &msg, &[])
        .unwrap();

    // no token asset
    let msg = ExecuteMsg::AutoStake {
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128(100u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128(100u128),
            },
        ],
        slippage_tolerance: None,
    };

    let res = app
        .execute(
            "addr".into(),
            staking_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128(100u128),
            }],
        )
        .unwrap_err();
    assert_eq!(
        res,
        StdError::generic_err("Missing token asset").to_string()
    );

    // no native asset
    let msg = ExecuteMsg::AutoStake {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: asset_addr.clone(),
                },
                amount: Uint128::from(1u128),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: asset_addr.clone(),
                },
                amount: Uint128::from(1u128),
            },
        ],
        slippage_tolerance: None,
    };

    let res = app
        .execute("addr".into(), staking_addr.clone(), &msg, &[])
        .unwrap_err();
    assert_eq!(
        res,
        StdError::generic_err("Missing native asset").to_string()
    );

    let msg = ExecuteMsg::AutoStake {
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128(100u128),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: asset_addr.clone(),
                },
                amount: Uint128(1u128),
            },
        ],
        slippage_tolerance: None,
    };

    // attempt with no coins
    let res = app
        .execute("addr".into(), staking_addr.clone(), &msg, &[])
        .unwrap_err();
    assert_eq!(
        res,
        StdError::generic_err(
            "Native token balance mismatch between the argument and the transferred"
        )
        .to_string()
    );

    let _res = app
        .execute(
            "addr".into(),
            staking_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128(100u128),
            }],
        )
        .unwrap();

    // wrong asset
    let msg = ExecuteMsg::AutoStakeHook {
        asset_info: AssetInfo::Token {
            contract_addr: "asset1".into(),
        },
        staking_token: pair_info.liquidity_token.clone(),
        staker_addr: "addr".into(),
        prev_staking_token_amount: Uint128(0),
    };
    let res = app
        .execute(staking_addr.clone(), staking_addr.clone(), &msg, &[])
        .unwrap_err();
    // pool not found error
    assert_eq!(res.contains("PoolInfo not found"), true);

    // valid msg
    let msg = ExecuteMsg::AutoStakeHook {
        asset_info: AssetInfo::Token {
            contract_addr: asset_addr.clone(),
        },
        staking_token: pair_info.liquidity_token.clone(),
        staker_addr: "addr".into(),
        prev_staking_token_amount: Uint128(0),
    };

    // unauthorized attempt
    let res = app
        .execute("addr".into(), staking_addr.clone(), &msg, &[])
        .unwrap_err();
    assert_eq!(res, StdError::generic_err("unauthorized").to_string());

    // successfull attempt

    let res = app
        .execute(staking_addr.clone(), staking_addr.clone(), &msg, &[])
        .unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "bond"),
            attr("staker_addr", "addr"),
            attr("asset_info", asset_addr.as_str()),
            attr("amount", "1"),
        ]
    );

    let pool_info: PoolInfoResponse = app
        .query(
            staking_addr.clone(),
            &QueryMsg::PoolInfo {
                asset_info: AssetInfo::Token {
                    contract_addr: asset_addr.clone(),
                },
            },
        )
        .unwrap();

    assert_eq!(
        pool_info,
        PoolInfoResponse {
            asset_info: AssetInfo::Token {
                contract_addr: asset_addr.clone()
            },
            staking_token: pair_info.liquidity_token.clone(),
            total_bond_amount: Uint128(1u128),
            reward_index: Decimal::zero(),
            pending_reward: Uint128::zero(),
            migration_deprecated_staking_token: None,
            migration_index_snapshot: None,
        }
    );
}
