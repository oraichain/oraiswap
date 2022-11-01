use crate::contract::{handle, init, query};
use crate::testing::mock_querier::mock_dependencies_with_querier;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    attr, coin, from_binary, to_binary, BankMsg, Coin, CosmosMsg, Decimal, StdError, Uint128,
    WasmMsg,
};
use cw20::{Cw20HandleMsg, Cw20ReceiveMsg};
use oraiswap::asset::{Asset, AssetInfo, ORAI_DENOM};
use oraiswap::mock_app::ATOM_DENOM;
use oraiswap::pair::HandleMsg as PairHandleMsg;
use oraiswap::staking::{
    Cw20HookMsg, HandleMsg, InitMsg, PoolInfoResponse, QueryMsg, RewardInfoResponse,
    RewardInfoResponseItem,
};

#[test]
fn test_bond_tokens() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        owner: Some("owner".into()),
        rewarder: "reward".into(),
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
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
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
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

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

    let info = mock_info("staking2", &[]);
    let res = handle(deps.as_mut(), mock_env(), info, msg);
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

    // will also add to the index the pending rewards from before the migration
    let msg = HandleMsg::UpdateRewardsPerSec {
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
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    // register asset
    let msg = HandleMsg::RegisterAsset {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        staking_token: "staking".into(),
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

    let msg = HandleMsg::DepositReward {
        rewards: vec![Asset {
            info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
            amount: Uint128(300u128),
        }],
    };
    let info = mock_info("rewarder", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    // will also add to the index the pending rewards from before the migration
    let msg = HandleMsg::UpdateRewardsPerSec {
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
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    // unbond 150 tokens; failed
    let msg = HandleMsg::Unbond {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        amount: Uint128(150u128),
    };

    let info = mock_info("addr", &[]);
    let res = handle(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    match res {
        StdError::GenericErr { msg, .. } => {
            assert_eq!(msg, "Cannot unbond more than bond amount");
        }
        _ => panic!("Must return generic error"),
    };

    // normal unbond
    let msg = HandleMsg::Unbond {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        amount: Uint128(100u128),
    };

    let info = mock_info("addr", &[]);
    let res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            WasmMsg::Execute {
                contract_addr: "staking".into(),
                msg: to_binary(&Cw20HandleMsg::Transfer {
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
    let mut deps = mock_dependencies_with_querier(&[]);
    deps.querier.with_pair_info("pair".into());
    deps.querier.with_pool_assets([
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128::from(100u128),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
            amount: Uint128::from(1u128),
        },
    ]);

    let msg = InitMsg {
        owner: Some("owner".into()),
        rewarder: "reward".into(),
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
        staking_token: "lptoken".into(),
    };

    let info = mock_info("owner", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    // no token asset
    let msg = HandleMsg::AutoStake {
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128(100u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128(100u128),
            },
        ],
        slippage_tolerance: None,
    };
    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128(100u128),
        }],
    );
    let res = handle(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res, StdError::generic_err("Missing token asset"));

    // no native asset
    let msg = HandleMsg::AutoStake {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: "asset".into(),
                },
                amount: Uint128::from(1u128),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: "asset".into(),
                },
                amount: Uint128::from(1u128),
            },
        ],
        slippage_tolerance: None,
    };
    let info = mock_info("addr0000", &[]);
    let res = handle(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res, StdError::generic_err("Missing native asset"));

    let msg = HandleMsg::AutoStake {
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128(100u128),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: "asset".into(),
                },
                amount: Uint128(1u128),
            },
        ],
        slippage_tolerance: None,
    };

    // attempt with no coins
    let info = mock_info("addr0000", &[]);
    let res = handle(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(
        res,
        StdError::generic_err(
            "Native token balance mismatch between the argument and the transferred"
        )
    );

    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128(100u128),
        }],
    );
    let res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            WasmMsg::Execute {
                contract_addr: "asset".into(),
                msg: to_binary(&Cw20HandleMsg::TransferFrom {
                    owner: "addr0000".into(),
                    recipient: MOCK_CONTRACT_ADDR.into(),
                    amount: Uint128(1u128),
                })
                .unwrap(),
                send: vec![],
            }
            .into(),
            WasmMsg::Execute {
                contract_addr: "asset".into(),
                msg: to_binary(&Cw20HandleMsg::IncreaseAllowance {
                    spender: "pair".into(),
                    amount: Uint128(1),
                    expires: None,
                })
                .unwrap(),
                send: vec![],
            }
            .into(),
            WasmMsg::Execute {
                contract_addr: "pair".into(),
                msg: to_binary(&PairHandleMsg::ProvideLiquidity {
                    assets: [
                        Asset {
                            info: AssetInfo::NativeToken {
                                denom: "uusd".to_string()
                            },
                            amount: Uint128(99u128),
                        },
                        Asset {
                            info: AssetInfo::Token {
                                contract_addr: "asset".into()
                            },
                            amount: Uint128(1u128),
                        },
                    ],
                    slippage_tolerance: None,
                    receiver: None,
                })
                .unwrap(),
                send: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128(99u128), // 1% tax
                }],
            }
            .into(),
            WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.into(),
                msg: to_binary(&HandleMsg::AutoStakeHook {
                    asset_info: AssetInfo::Token {
                        contract_addr: "asset".into()
                    },
                    staking_token: "lptoken".into(),
                    staker_addr: "addr0000".into(),
                    prev_staking_token_amount: Uint128(0),
                })
                .unwrap(),
                send: vec![],
            }
            .into()
        ]
    );

    deps.querier.with_token_balance(Uint128(100u128)); // recive 100 lptoken

    // wrong asset
    let msg = HandleMsg::AutoStakeHook {
        asset_info: AssetInfo::Token {
            contract_addr: "asset1".into(),
        },
        staking_token: "lptoken".into(),
        staker_addr: "addr0000".into(),
        prev_staking_token_amount: Uint128(0),
    };
    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap_err(); // pool not found error

    // valid msg
    let msg = HandleMsg::AutoStakeHook {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        staking_token: "lptoken".into(),
        staker_addr: "addr0000".into(),
        prev_staking_token_amount: Uint128(0),
    };

    // unauthorized attempt
    let info = mock_info("addr0000", &[]);
    let res = handle(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(res, StdError::generic_err("unauthorized"));

    // successfull attempt
    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "bond"),
            attr("staker_addr", "addr0000"),
            attr("asset_info", "asset"),
            attr("amount", "100"),
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
            staking_token: "lptoken".into(),
            total_bond_amount: Uint128(100u128),
            reward_index: Decimal::zero(),
            pending_reward: Uint128::zero(),
            migration_deprecated_staking_token: None,
            migration_index_snapshot: None,
        }
    );
}
