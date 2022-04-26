use crate::contract::{handle, init, query};
use crate::state::{read_pool_info, rewards_read, store_pool_info, PoolInfo, RewardInfo};
use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{coin, coins, from_binary, to_binary, Api, BankMsg, Decimal, Uint128, WasmMsg};
use cw20::{Cw20HandleMsg, Cw20ReceiveMsg};
use oraiswap::asset::{Asset, AssetInfo, ORAI_DENOM};
use oraiswap::mock_querier::{mock_dependencies, ATOM_DENOM};
use oraiswap::staking::{
    Cw20HookMsg, HandleMsg, InitMsg, PoolInfoResponse, QueryMsg, RewardInfoResponse,
    RewardInfoResponseItem,
};

#[test]
fn test_deposit_reward() {
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

    let msg = HandleMsg::RegisterAsset {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        staking_token: "staking".into(),
    };

    let info = mock_info("owner", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    let token_raw = deps.api.canonical_address(&"asset".into()).unwrap();
    let pool_info = read_pool_info(&deps.storage, &token_raw).unwrap();
    store_pool_info(&mut deps.storage, &token_raw, &pool_info).unwrap();

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

    // factory deposit 100 reward tokens
    let msg = HandleMsg::DepositReward {
        rewards: vec![Asset {
            info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
            amount: Uint128(100u128),
        }],
    };
    let info = mock_info("rewarder", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

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

    let asset_key = deps.api.canonical_address(&"asset".into()).unwrap();
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

    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

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

    let msg = HandleMsg::RegisterAsset {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        staking_token: "staking".into(),
    };

    let info = mock_info("owner", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    let token_raw = deps.api.canonical_address(&"asset".into()).unwrap();
    let pool_info = read_pool_info(&deps.storage, &token_raw).unwrap();
    store_pool_info(&mut deps.storage, &token_raw, &pool_info).unwrap();

    // factory deposit 100 reward tokens
    let msg = HandleMsg::DepositReward {
        rewards: vec![Asset {
            info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
            amount: Uint128(100u128),
        }],
    };
    let info = mock_info("rewarder", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

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

    let asset_key = deps.api.canonical_address(&"asset".into()).unwrap();
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

    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

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

    let msg = HandleMsg::RegisterAsset {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        staking_token: "staking".into(),
    };

    let info = mock_info("owner", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    let token_raw = deps.api.canonical_address(&"asset".into()).unwrap();
    let pool_info = read_pool_info(&deps.storage, &token_raw).unwrap();
    store_pool_info(&mut deps.storage, &token_raw, &pool_info).unwrap();

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
            amount: Uint128(100u128),
        }],
    };

    let info = mock_info("rewarder", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    let asset_key = deps.api.canonical_address(&"asset".into()).unwrap();
    let addr_raw = deps.api.canonical_address(&"addr".into()).unwrap();
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

    // unbond
    let msg = HandleMsg::Unbond {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        amount: Uint128(100u128),
    };
    let info = mock_info("addr", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

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
    let mut deps = mock_dependencies(&[
        coin(10000000000u128, ORAI_DENOM),
        coin(20000000000u128, ATOM_DENOM),
    ]);

    // update other contract token balance
    deps.querier.with_token_balances(&[(
        &"reward".to_string(),
        &[(
            &MOCK_CONTRACT_ADDR.to_string(),
            &Uint128::from(10000000000u128),
        )],
    )]);

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
            Asset {
                info: AssetInfo::Token {
                    contract_addr: "reward".into(),
                },
                amount: 200u128.into(),
            },
        ],
    };
    let info = mock_info("owner", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = HandleMsg::RegisterAsset {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        staking_token: "staking".into(),
    };

    let info = mock_info("owner", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    let token_raw = deps.api.canonical_address(&"asset".into()).unwrap();
    let pool_info = read_pool_info(&deps.storage, &token_raw).unwrap();
    store_pool_info(&mut deps.storage, &token_raw, &pool_info).unwrap();

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
            amount: Uint128(100u128),
        }],
    };
    let info = mock_info("rewarder", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = HandleMsg::Withdraw {
        asset_info: Some(AssetInfo::Token {
            contract_addr: "asset".into(),
        }),
    };
    let info = mock_info("addr", &[]);
    let res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![
            BankMsg::Send {
                from_address: MOCK_CONTRACT_ADDR.into(),
                to_address: "addr".into(),
                amount: coins(20u128, ORAI_DENOM.to_string())
            }
            .into(),
            BankMsg::Send {
                from_address: MOCK_CONTRACT_ADDR.into(),
                to_address: "addr".into(),
                amount: coins(40u128, ATOM_DENOM.to_string())
            }
            .into(),
            WasmMsg::Execute {
                contract_addr: "reward".into(),
                msg: to_binary(&Cw20HandleMsg::Transfer {
                    recipient: "addr".into(),
                    amount: Uint128(40u128),
                })
                .unwrap(),
                send: vec![],
            }
            .into()
        ]
    );
}

#[test]
fn withdraw_multiple_rewards() {
    let mut deps = mock_dependencies(&[
        coin(10000000000u128, ORAI_DENOM),
        coin(20000000000u128, ATOM_DENOM),
    ]);

    // update other contract token balance
    deps.querier.with_token_balances(&[(
        &"reward".to_string(),
        &[(
            &MOCK_CONTRACT_ADDR.to_string(),
            &Uint128::from(10000000000u128),
        )],
    )]);

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
            Asset {
                info: AssetInfo::Token {
                    contract_addr: "reward".into(),
                },
                amount: 200u128.into(),
            },
        ],
    };
    let info = mock_info("owner", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = HandleMsg::UpdateRewardsPerSec {
        asset_info: AssetInfo::Token {
            contract_addr: "asset2".into(),
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
                    contract_addr: "reward".into(),
                },
                amount: 200u128.into(),
            },
        ],
    };
    let info = mock_info("owner", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = HandleMsg::RegisterAsset {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        staking_token: "staking".into(),
    };

    let info = mock_info("owner", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = HandleMsg::RegisterAsset {
        asset_info: AssetInfo::Token {
            contract_addr: "asset2".into(),
        },
        staking_token: "staking2".into(),
    };

    let info = mock_info("owner", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    let token_raw = deps.api.canonical_address(&"asset".into()).unwrap();
    let pool_info = read_pool_info(&deps.storage, &token_raw).unwrap();
    store_pool_info(&mut deps.storage, &token_raw, &pool_info).unwrap();

    let token_raw = deps.api.canonical_address(&"asset2".into()).unwrap();
    let pool_info = read_pool_info(&deps.storage, &token_raw).unwrap();
    store_pool_info(&mut deps.storage, &token_raw, &pool_info).unwrap();

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

    // bond second 1000 tokens
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: "addr".into(),
        amount: Uint128(1000u128),
        msg: to_binary(&Cw20HookMsg::Bond {
            asset_info: AssetInfo::Token {
                contract_addr: "asset2".into(),
            },
        })
        .ok(),
    });
    let info = mock_info("staking2", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    // factory deposit asset
    let msg = HandleMsg::DepositReward {
        rewards: vec![
            Asset {
                info: AssetInfo::Token {
                    contract_addr: "asset".into(),
                },
                amount: Uint128(100u128),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: "asset2".into(),
                },
                amount: Uint128(200u128),
            },
        ],
    };
    let info = mock_info("rewarder", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

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
            reward_infos: vec![
                RewardInfoResponseItem {
                    asset_info: AssetInfo::Token {
                        contract_addr: "asset".into()
                    },
                    bond_amount: Uint128(100u128),
                    pending_reward: Uint128(100u128),
                    pending_withdraw: vec![],
                    should_migrate: None,
                },
                RewardInfoResponseItem {
                    asset_info: AssetInfo::Token {
                        contract_addr: "asset2".into()
                    },
                    bond_amount: Uint128(1000u128),
                    pending_reward: Uint128(200u128),
                    pending_withdraw: vec![],
                    should_migrate: None,
                },
            ],
        }
    );

    // withdraw all
    let msg = HandleMsg::Withdraw { asset_info: None };
    let info = mock_info("addr", &[]);
    let res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![
            BankMsg::Send {
                from_address: MOCK_CONTRACT_ADDR.into(),
                to_address: "addr".into(),
                amount: coins(60u128, ORAI_DENOM.to_string())
            }
            .into(),
            BankMsg::Send {
                from_address: MOCK_CONTRACT_ADDR.into(),
                to_address: "addr".into(),
                amount: coins(120u128, ATOM_DENOM.to_string())
            }
            .into(),
            WasmMsg::Execute {
                contract_addr: "reward".into(),
                msg: to_binary(&Cw20HandleMsg::Transfer {
                    recipient: "addr".into(),
                    amount: Uint128(120u128),
                })
                .unwrap(),
                send: vec![],
            }
            .into()
        ]
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
            reward_infos: vec![
                RewardInfoResponseItem {
                    asset_info: AssetInfo::Token {
                        contract_addr: "asset".into()
                    },
                    bond_amount: Uint128(100u128),
                    pending_reward: Uint128::zero(),
                    pending_withdraw: vec![],
                    should_migrate: None,
                },
                RewardInfoResponseItem {
                    asset_info: AssetInfo::Token {
                        contract_addr: "asset2".into()
                    },
                    bond_amount: Uint128(1000u128),
                    pending_reward: Uint128::zero(),
                    pending_withdraw: vec![],
                    should_migrate: None,
                },
            ],
        }
    );
}

#[test]
fn test_update_rewards_per_sec() {
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

    let msg = HandleMsg::RegisterAsset {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        staking_token: "staking".into(),
    };

    let info = mock_info("owner", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    let token_raw = deps.api.canonical_address(&"asset".into()).unwrap();
    let pool_info = read_pool_info(&deps.storage, &token_raw).unwrap();
    store_pool_info(&mut deps.storage, &token_raw, &pool_info).unwrap();

    // bond 100 tokens
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
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
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    // factory deposit 300 reward tokens
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

    // change rewards per second for the pool
    let _res = handle(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        HandleMsg::UpdateRewardsPerSec {
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
    let msg = HandleMsg::DepositReward {
        rewards: vec![Asset {
            info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
            amount: Uint128(100u128),
        }],
    };
    let info = mock_info("rewarder", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

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
