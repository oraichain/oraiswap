use std::ops::Add;

use crate::contract::{handle, init, query};
use crate::state::{read_pool_info, rewards_read, store_pool_info, PoolInfo, RewardInfo};
use crate::testing::mock_querier::mock_dependencies_with_querier;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{from_binary, to_binary, Api, Decimal, StdError, Uint128, WasmMsg};
use cw20::{Cw20HandleMsg, Cw20ReceiveMsg};
use oraiswap::asset::{Asset, AssetInfo};
use oraiswap::staking::{
    Cw20HookMsg, HandleMsg, InitMsg, PoolInfoResponse, QueryMsg, RewardInfoResponse,
    RewardInfoResponseItem,
};

#[test]
fn test_deposit_reward() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        owner: Some("owner".into()),
        oraix_token: "reward".into(),
        minter: Some("mint".into()),
        oracle_contract: "oracle".into(),
        oraiswap_factory: "oraiswap_factory".into(),
        base_denom: None,
        premium_min_update_interval: Some(3600),
        short_reward_bound: None,
    };

    let info = mock_info("addr", &[]);
    let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = HandleMsg::RegisterAsset {
        asset_token: "asset".into(),
        staking_token: "staking".into(),
    };

    let info = mock_info("owner", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    // store 3% premium rate
    let token_raw = deps.api.canonical_address(&"asset".into()).unwrap();
    let pool_info = read_pool_info(&deps.storage, &token_raw).unwrap();
    store_pool_info(
        &mut deps.storage,
        &token_raw,
        &PoolInfo {
            premium_rate: Decimal::percent(2),
            short_reward_weight: Decimal::percent(20),
            ..pool_info
        },
    )
    .unwrap();

    // bond 100 tokens
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: "addr".into(),
        amount: Uint128(100u128),
        msg: to_binary(&Cw20HookMsg::Bond {
            asset_token: "asset".into(),
        })
        .ok(),
    });
    let info = mock_info("staking", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    // bond 100 short token
    let msg = HandleMsg::IncreaseShortToken {
        asset_token: "asset".into(),
        staker_addr: "addr".into(),
        amount: Uint128(100u128),
    };
    let info = mock_info("mint", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    // factory deposit 100 reward tokens
    // premium is 0, so rewards distributed 80:20
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: "factory".into(),
        amount: Uint128(100u128),
        msg: to_binary(&Cw20HookMsg::DepositReward {
            rewards: vec![("asset".into(), Uint128(100u128))],
        })
        .ok(),
    });
    let info = mock_info("reward", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    // Check pool state
    let res: PoolInfoResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::PoolInfo {
                asset_token: "asset".into(),
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
            total_short_amount: Uint128(100u128),
            reward_index: Decimal::from_ratio(80u128, 100u128),
            short_reward_index: Decimal::from_ratio(20u128, 100u128),
            ..res
        }
    );

    // if premium_rate is over threshold, distribution weight should be 60:40
    let asset_token_raw = deps.api.canonical_address(&"asset".into()).unwrap();
    let pool_info: PoolInfo = read_pool_info(&deps.storage, &asset_token_raw).unwrap();
    store_pool_info(
        &mut deps.storage,
        &asset_token_raw,
        &PoolInfo {
            reward_index: Decimal::zero(),
            short_reward_index: Decimal::zero(),
            premium_rate: Decimal::percent(10),
            short_reward_weight: Decimal::percent(40),
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
                asset_token: "asset".into(),
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
            total_short_amount: Uint128(100u128),
            reward_index: Decimal::from_ratio(60u128, 100u128),
            short_reward_index: Decimal::from_ratio(40u128, 100u128),
            ..res
        }
    );
}

#[test]
fn test_deposit_reward_when_no_bonding() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        owner: Some("owner".into()),
        oraix_token: "reward".into(),
        minter: Some("mint".into()),
        oracle_contract: "oracle".into(),
        oraiswap_factory: "oraiswap_factory".into(),
        base_denom: None,
        premium_min_update_interval: Some(3600),
        short_reward_bound: None,
    };

    let info = mock_info("addr", &[]);
    let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = HandleMsg::RegisterAsset {
        asset_token: "asset".into(),
        staking_token: "staking".into(),
    };

    let info = mock_info("owner", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    // store 3% premium rate
    let token_raw = deps.api.canonical_address(&"asset".into()).unwrap();
    let pool_info = read_pool_info(&deps.storage, &token_raw).unwrap();
    store_pool_info(
        &mut deps.storage,
        &token_raw,
        &PoolInfo {
            premium_rate: Decimal::percent(2),
            short_reward_weight: Decimal::percent(20),
            ..pool_info
        },
    )
    .unwrap();

    // factory deposit 100 reward tokens
    // premium is 0, so rewards distributed 80:20
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: "factory".into(),
        amount: Uint128(100u128),
        msg: to_binary(&Cw20HookMsg::DepositReward {
            rewards: vec![("asset".into(), Uint128(100u128))],
        })
        .ok(),
    });
    let info = mock_info("reward", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    // Check pool state
    let res: PoolInfoResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::PoolInfo {
                asset_token: "asset".into(),
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
            short_reward_index: Decimal::zero(),
            pending_reward: Uint128(80u128),
            short_pending_reward: Uint128(20u128),
            ..res
        }
    );

    // if premium_rate is over threshold, distribution weight should be 60:40
    let asset_token_raw = deps.api.canonical_address(&"asset".into()).unwrap();
    let pool_info: PoolInfo = read_pool_info(&deps.storage, &asset_token_raw).unwrap();
    store_pool_info(
        &mut deps.storage,
        &asset_token_raw,
        &PoolInfo {
            pending_reward: Uint128::zero(),
            short_pending_reward: Uint128::zero(),
            premium_rate: Decimal::percent(10),
            short_reward_weight: Decimal::percent(40),
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
                asset_token: "asset".into(),
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
            short_reward_index: Decimal::zero(),
            pending_reward: Uint128(60u128),
            short_pending_reward: Uint128(40u128),
            ..res
        }
    );
}

#[test]
fn test_before_share_changes() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        owner: Some("owner".into()),
        oraix_token: "reward".into(),
        minter: Some("mint".into()),
        oracle_contract: "oracle".into(),
        oraiswap_factory: "oraiswap_factory".into(),
        base_denom: None,
        premium_min_update_interval: Some(3600),
        short_reward_bound: None,
    };

    let info = mock_info("addr", &[]);
    let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = HandleMsg::RegisterAsset {
        asset_token: "asset".into(),
        staking_token: "staking".into(),
    };

    let info = mock_info("owner", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    // store 3% premium rate
    let token_raw = deps.api.canonical_address(&"asset".into()).unwrap();
    let pool_info = read_pool_info(&deps.storage, &token_raw).unwrap();
    store_pool_info(
        &mut deps.storage,
        &token_raw,
        &PoolInfo {
            premium_rate: Decimal::percent(2),
            short_reward_weight: Decimal::percent(20),
            ..pool_info
        },
    )
    .unwrap();

    // bond 100 tokens
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: "addr".into(),
        amount: Uint128(100u128),
        msg: to_binary(&Cw20HookMsg::Bond {
            asset_token: "asset".into(),
        })
        .ok(),
    });
    let info = mock_info("staking", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    // bond 100 short token
    let msg = HandleMsg::IncreaseShortToken {
        asset_token: "asset".into(),
        staker_addr: "addr".into(),
        amount: Uint128(100u128),
    };
    let info = mock_info("mint", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    // factory deposit 100 reward tokens
    // premium is 0, so rewards distributed 80:20
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: "factory".into(),
        amount: Uint128(100u128),
        msg: to_binary(&Cw20HookMsg::DepositReward {
            rewards: vec![("asset".into(), Uint128(100u128))],
        })
        .ok(),
    });

    let info = mock_info("reward", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    let asset_raw = deps.api.canonical_address(&"asset".into()).unwrap();
    let addr_raw = deps.api.canonical_address(&"addr".into()).unwrap();
    let reward_bucket = rewards_read(&deps.storage, &addr_raw, false);
    let reward_info: RewardInfo = reward_bucket.load(asset_raw.as_slice()).unwrap();
    assert_eq!(
        RewardInfo {
            pending_reward: Uint128::zero(),
            bond_amount: Uint128(100u128),
            index: Decimal::zero(),
        },
        reward_info
    );

    // bond 100 more tokens
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: "addr".into(),
        amount: Uint128(100u128),
        msg: to_binary(&Cw20HookMsg::Bond {
            asset_token: "asset".into(),
        })
        .ok(),
    });
    let info = mock_info("staking", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    let reward_bucket = rewards_read(&deps.storage, &addr_raw, false);
    let reward_info: RewardInfo = reward_bucket.load(asset_raw.as_slice()).unwrap();
    assert_eq!(
        RewardInfo {
            pending_reward: Uint128(80u128),
            bond_amount: Uint128(200u128),
            index: Decimal::from_ratio(80u128, 100u128),
        },
        reward_info
    );

    // factory deposit 100 reward tokens; = 0.8 + 0.4 = 1.2 is reward_index
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: "factory".into(),
        amount: Uint128(100u128),
        msg: to_binary(&Cw20HookMsg::DepositReward {
            rewards: vec![("asset".into(), Uint128(100u128))],
        })
        .ok(),
    });
    let info = mock_info("reward", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    // unbond
    let msg = HandleMsg::Unbond {
        asset_token: "asset".into(),
        amount: Uint128(100u128),
    };
    let info = mock_info("addr", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    let reward_bucket = rewards_read(&deps.storage, &addr_raw, false);
    let reward_info: RewardInfo = reward_bucket.load(asset_raw.as_slice()).unwrap();
    assert_eq!(
        RewardInfo {
            pending_reward: Uint128(160u128),
            bond_amount: Uint128(100u128),
            index: Decimal::from_ratio(120u128, 100u128),
        },
        reward_info
    );
}

#[test]
fn test_withdraw() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        owner: Some("owner".into()),
        oraix_token: "reward".into(),
        minter: Some("mint".into()),
        oracle_contract: "oracle".into(),
        oraiswap_factory: "oraiswap_factory".into(),
        base_denom: None,
        premium_min_update_interval: Some(3600),
        short_reward_bound: None,
    };

    let info = mock_info("addr", &[]);
    let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = HandleMsg::RegisterAsset {
        asset_token: "asset".into(),
        staking_token: "staking".into(),
    };

    let info = mock_info("owner", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    // store 3% premium rate
    let token_raw = deps.api.canonical_address(&"asset".into()).unwrap();
    let pool_info = read_pool_info(&deps.storage, &token_raw).unwrap();
    store_pool_info(
        &mut deps.storage,
        &token_raw,
        &PoolInfo {
            premium_rate: Decimal::percent(2),
            short_reward_weight: Decimal::percent(20),
            ..pool_info
        },
    )
    .unwrap();

    // bond 100 tokens
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: "addr".into(),
        amount: Uint128(100u128),
        msg: to_binary(&Cw20HookMsg::Bond {
            asset_token: "asset".into(),
        })
        .ok(),
    });
    let info = mock_info("staking", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    // factory deposit 100 reward tokens
    // premium_rate is zero; distribute weight => 80:20
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: "factory".into(),
        amount: Uint128(100u128),
        msg: to_binary(&Cw20HookMsg::DepositReward {
            rewards: vec![("asset".into(), Uint128(100u128))],
        })
        .ok(),
    });
    let info = mock_info("reward", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = HandleMsg::Withdraw {
        asset_token: Some("asset".into()),
    };
    let info = mock_info("addr", &[]);
    let res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![WasmMsg::Execute {
            contract_addr: "reward".into(),
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: "addr".into(),
                amount: Uint128(80u128),
            })
            .unwrap(),
            send: vec![],
        }
        .into()]
    );
}

#[test]
fn withdraw_multiple_rewards() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        owner: Some("owner".into()),
        oraix_token: "reward".into(),
        minter: Some("mint".into()),
        oracle_contract: "oracle".into(),
        oraiswap_factory: "oraiswap_factory".into(),
        base_denom: None,
        premium_min_update_interval: Some(3600),
        short_reward_bound: None,
    };

    let info = mock_info("addr", &[]);
    let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = HandleMsg::RegisterAsset {
        asset_token: "asset".into(),
        staking_token: "staking".into(),
    };

    let info = mock_info("owner", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = HandleMsg::RegisterAsset {
        asset_token: "asset2".into(),
        staking_token: "staking2".into(),
    };

    let info = mock_info("owner", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    // store 3% premium rate
    let token_raw = deps.api.canonical_address(&"asset".into()).unwrap();
    let pool_info = read_pool_info(&deps.storage, &token_raw).unwrap();
    store_pool_info(
        &mut deps.storage,
        &token_raw,
        &PoolInfo {
            premium_rate: Decimal::percent(2),
            short_reward_weight: Decimal::percent(20),
            ..pool_info
        },
    )
    .unwrap();

    // store 3% premium rate for asset2
    let token_raw = deps.api.canonical_address(&"asset2".into()).unwrap();
    let pool_info = read_pool_info(&deps.storage, &token_raw).unwrap();
    store_pool_info(
        &mut deps.storage,
        &token_raw,
        &PoolInfo {
            premium_rate: Decimal::percent(2),
            short_reward_weight: Decimal::percent(20),
            ..pool_info
        },
    )
    .unwrap();

    // bond 100 tokens
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: "addr".into(),
        amount: Uint128(100u128),
        msg: to_binary(&Cw20HookMsg::Bond {
            asset_token: "asset".into(),
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
            asset_token: "asset2".into(),
        })
        .ok(),
    });
    let info = mock_info("staking2", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    // bond 50 short token
    let msg = HandleMsg::IncreaseShortToken {
        asset_token: "asset".into(),
        staker_addr: "addr".into(),
        amount: Uint128(50u128),
    };
    let info = mock_info("mint", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    // factory deposit asset
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: "factory".into(),
        amount: Uint128(300u128),
        msg: to_binary(&Cw20HookMsg::DepositReward {
            rewards: vec![
                ("asset".into(), Uint128(100u128)),
                ("asset2".into(), Uint128(200u128)),
            ],
        })
        .ok(),
    });
    let info = mock_info("reward", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    let data = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::RewardInfo {
            asset_token: None,
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
                    asset_token: "asset".into(),
                    bond_amount: Uint128(100u128),
                    pending_reward: Uint128(80u128),
                    is_short: false,
                    should_migrate: None,
                },
                RewardInfoResponseItem {
                    asset_token: "asset2".into(),
                    bond_amount: Uint128(1000u128),
                    pending_reward: Uint128(160u128),
                    is_short: false,
                    should_migrate: None,
                },
                RewardInfoResponseItem {
                    asset_token: "asset".into(),
                    bond_amount: Uint128(50u128),
                    pending_reward: Uint128(20u128),
                    is_short: true,
                    should_migrate: None,
                },
            ],
        }
    );

    // withdraw all
    let msg = HandleMsg::Withdraw { asset_token: None };
    let info = mock_info("addr", &[]);
    let res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![WasmMsg::Execute {
            contract_addr: "reward".into(),
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: "addr".into(),
                amount: Uint128(260u128),
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
            asset_token: None,
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
                    asset_token: "asset".into(),
                    bond_amount: Uint128(100u128),
                    pending_reward: Uint128::zero(),
                    is_short: false,
                    should_migrate: None,
                },
                RewardInfoResponseItem {
                    asset_token: "asset2".into(),
                    bond_amount: Uint128(1000u128),
                    pending_reward: Uint128::zero(),
                    is_short: false,
                    should_migrate: None,
                },
                RewardInfoResponseItem {
                    asset_token: "asset".into(),
                    bond_amount: Uint128(50u128),
                    pending_reward: Uint128::zero(),
                    is_short: true,
                    should_migrate: None,
                },
            ],
        }
    );
}

#[test]
fn test_adjust_premium() {
    let mut deps = mock_dependencies_with_querier(&[]);

    // oraiswap price 100
    // oracle price 100
    // premium zero
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
    deps.querier
        .with_oracle_price(Decimal::from_ratio(100u128, 1u128));

    let msg = InitMsg {
        owner: Some("owner".into()),
        oraix_token: "reward".into(),
        minter: Some("mint".into()),
        oracle_contract: "oracle".into(),
        oraiswap_factory: "oraiswap_factory".into(),
        base_denom: None,
        premium_min_update_interval: Some(3600),
        short_reward_bound: None,
    };

    let info = mock_info("addr", &[]);
    let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = HandleMsg::RegisterAsset {
        asset_token: "asset".into(),
        staking_token: "staking".into(),
    };

    let info = mock_info("owner", &[]);
    let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = HandleMsg::AdjustPremium {
        asset_tokens: vec!["asset".into()],
    };
    let mut env = mock_env();
    let info = mock_info("addr", &[]);
    let _ = handle(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

    // Check pool state
    let res: PoolInfoResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::PoolInfo {
                asset_token: "asset".into(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(res.premium_rate, Decimal::zero());
    assert_eq!(res.premium_updated_time, env.block.time);

    // oraiswap price = 90
    // premium rate = 0
    deps.querier.with_pool_assets([
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128::from(90u128),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
            amount: Uint128::from(1u128),
        },
    ]);

    // assert premium update interval
    let res = handle(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(
            msg,
            "cannot adjust premium before premium_min_update_interval passed"
        ),
        _ => panic!("DO NOT ENTER HERE"),
    }

    env.block.time = env.block.time.add(3600);
    let _ = handle(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

    // Check pool state
    let res: PoolInfoResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::PoolInfo {
                asset_token: "asset".into(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(res.premium_rate, Decimal::zero());
    assert_eq!(res.premium_updated_time, env.block.time);

    // oraiswap price = 105
    // premium rate = 5%
    deps.querier.with_pool_assets([
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128::from(105u128),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
            amount: Uint128::from(1u128),
        },
    ]);

    env.block.time = env.block.time.add(3600);
    let _ = handle(deps.as_mut(), env.clone(), info, msg).unwrap();

    // Check pool state
    let res: PoolInfoResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::PoolInfo {
                asset_token: "asset".into(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(res.premium_rate, Decimal::percent(5));
    assert_eq!(res.premium_updated_time, env.block.time);
}
