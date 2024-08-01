use cosmwasm_std::testing::MOCK_CONTRACT_ADDR;
use cosmwasm_std::{attr, to_json_binary, Addr, Coin, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;
use oraiswap::asset::{Asset, AssetInfo, ORAI_DENOM};
use oraiswap::create_entry_points_testing;
use oraiswap::pair::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, PairResponse};
use oraiswap::testing::{MockApp, ATOM_DENOM};

#[test]
fn provide_liquidity_both_native() {
    let mut app = MockApp::new(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        &[
            Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(200u128),
            },
            Coin {
                denom: ATOM_DENOM.to_string(),
                amount: Uint128::from(200u128),
            },
        ],
    )]);

    app.set_oracle_contract(Box::new(create_entry_points_testing!(oraiswap_oracle)));

    app.set_token_contract(Box::new(create_entry_points_testing!(oraiswap_token)));

    app.set_token_balances(&[
        ("liquidity", &[(&MOCK_CONTRACT_ADDR.to_string(), 0)]),
        ("asset", &[]),
    ])
    .unwrap();

    let msg = InstantiateMsg {
        oracle_addr: app.oracle_addr.clone(),
        asset_infos: [
            AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
            AssetInfo::NativeToken {
                denom: ATOM_DENOM.to_string(),
            },
        ],
        token_code_id: app.token_id(),
        commission_rate: None,
        admin: None,
    };

    // we can just call .unwrap() to assert this was a success
    let code_id = app.upload(Box::new(
        create_entry_points_testing!(crate).with_reply_empty(crate::contract::reply),
    ));

    let pair_addr = app
        .instantiate(code_id, Addr::unchecked("owner"), &msg, &[], "pair")
        .unwrap();

    // successfully provide liquidity for the exist pool
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ATOM_DENOM.to_string(),
                },
                amount: Uint128::from(100u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(100u128),
            },
        ],
        slippage_tolerance: None,
        receiver: None,
    };

    let res = app
        .execute(
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            pair_addr,
            &msg,
            &[
                Coin {
                    denom: ORAI_DENOM.to_string(),
                    amount: Uint128::from(100u128),
                },
                Coin {
                    denom: ATOM_DENOM.to_string(),
                    amount: Uint128::from(100u128),
                },
            ],
        )
        .unwrap();

    println!("{:?}", res);
}

#[test]
fn provide_liquidity() {
    // provide more liquidity 1:2, which is not proportional to 1:1,
    // then it must accept 1:1 and treat left amount as donation
    let mut app = MockApp::new(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        &[Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(400u128),
        }],
    )]);
    app.set_token_contract(Box::new(create_entry_points_testing!(oraiswap_token)));

    app.set_oracle_contract(Box::new(create_entry_points_testing!(oraiswap_oracle)));

    app.set_token_balances(&[
        ("liquidity", &[(&MOCK_CONTRACT_ADDR.to_string(), 1000u128)]),
        ("asset", &[(&MOCK_CONTRACT_ADDR.to_string(), 1000u128)]),
    ])
    .unwrap();

    let asset_addr = app.get_token_addr("asset").unwrap();

    let msg = InstantiateMsg {
        oracle_addr: app.oracle_addr.clone(),
        asset_infos: [
            AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
            AssetInfo::Token {
                contract_addr: asset_addr.clone(),
            },
        ],
        token_code_id: app.token_id(),
        commission_rate: None,
        admin: None,
    };

    // we can just call .unwrap() to assert this was a success
    let code_id = app.upload(Box::new(
        create_entry_points_testing!(crate).with_reply_empty(crate::contract::reply),
    ));
    let pair_addr = app
        .instantiate(code_id, Addr::unchecked("owner"), &msg, &[], "pair")
        .unwrap();

    // set allowance
    app.execute(
        Addr::unchecked(MOCK_CONTRACT_ADDR),
        asset_addr.clone(),
        &cw20::Cw20ExecuteMsg::IncreaseAllowance {
            spender: pair_addr.to_string(),
            amount: Uint128::from(100u128),
            expires: None,
        },
        &[],
    )
    .unwrap();

    // successfully provide liquidity for the exist pool
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: asset_addr.clone(),
                },
                amount: Uint128::from(100u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(100u128),
            },
        ],
        slippage_tolerance: None,
        receiver: None,
    };

    let _res = app
        .execute(
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            pair_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(100u128),
            }],
        )
        .unwrap();

    // set allowance one more 100
    app.execute(
        Addr::unchecked(MOCK_CONTRACT_ADDR),
        asset_addr.clone(),
        &cw20::Cw20ExecuteMsg::IncreaseAllowance {
            spender: pair_addr.to_string(),
            amount: Uint128::from(100u128),
            expires: None,
        },
        &[],
    )
    .unwrap();

    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: asset_addr.clone(),
                },
                amount: Uint128::from(100u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(200u128),
            },
        ],
        slippage_tolerance: None,
        receiver: Some(Addr::unchecked("staking0000")), // try changing receiver
    };

    // only accept 100, then 50 share will be generated with 100 * (100 / 200)
    let _res = app
        .execute(
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            pair_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(200u128),
            }],
        )
        .unwrap();

    // check wrong argument
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: asset_addr.clone(),
                },
                amount: Uint128::from(100u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(50u128),
            },
        ],
        slippage_tolerance: None,
        receiver: None,
    };

    let error = app
        .execute(
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            pair_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(100u128),
            }],
        )
        .unwrap_err();

    println!("provide_liquididty {}", error.root_cause().to_string());
}

#[test]
fn withdraw_liquidity() {
    let mut app = MockApp::new(&[(
        "addr0000",
        &[Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(1000u128),
        }],
    )]);

    app.set_oracle_contract(Box::new(create_entry_points_testing!(oraiswap_oracle)));

    app.set_tax(Decimal::zero(), &[(&ORAI_DENOM.to_string(), 1000000u128)]);

    app.set_token_contract(Box::new(create_entry_points_testing!(oraiswap_token)));

    app.set_token_balances(&[("liquidity", &[("addr0000", 1000u128)])])
        .unwrap();

    let liquidity_addr = app.get_token_addr("liquidity").unwrap();

    let msg = InstantiateMsg {
        oracle_addr: app.oracle_addr.clone(),
        asset_infos: [
            AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
            AssetInfo::Token {
                contract_addr: liquidity_addr.clone(),
            },
        ],
        token_code_id: app.token_id(),
        commission_rate: None,
        admin: None,
    };

    let pair_id = app.upload(Box::new(
        create_entry_points_testing!(crate).with_reply_empty(crate::contract::reply),
    ));
    // we can just call .unwrap() to assert this was a success
    let pair_addr = app
        .instantiate(pair_id, Addr::unchecked("addr0000"), &msg, &[], "pair")
        .unwrap();

    // set allowance one more 100
    app.execute(
        Addr::unchecked("addr0000"),
        liquidity_addr.clone(),
        &cw20::Cw20ExecuteMsg::IncreaseAllowance {
            spender: pair_addr.to_string(),
            amount: Uint128::from(1000u128),
            expires: None,
        },
        &[],
    )
    .unwrap();

    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: liquidity_addr.clone(),
                },
                amount: Uint128::from(100u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(100u128),
            },
        ],
        slippage_tolerance: None,
        // we send lq token to pair and later call it directly to test
        receiver: Some(pair_addr.clone()),
    };

    // only accept 100, then 50 share will be generated with 100 * (100 / 200)
    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            pair_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(100u128),
            }],
        )
        .unwrap();

    // withdraw liquidity
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".into(),
        msg: to_json_binary(&Cw20HookMsg::WithdrawLiquidity {}).unwrap(),
        amount: Uint128::from(100u128),
    });

    let PairResponse { info: pair_info } = app
        .query(pair_addr.clone(), &oraiswap::pair::QueryMsg::Pair {})
        .unwrap();

    let res = app
        .execute(pair_info.liquidity_token, pair_addr.clone(), &msg, &[])
        .unwrap();

    let attributes = res.custom_attrs(1);
    let log_withdrawn_share = attributes.get(2).expect("no log");
    let log_refund_assets = attributes.get(3).expect("no log");

    assert_eq!(
        log_withdrawn_share,
        &attr("withdrawn_share", 100u128.to_string())
    );
    assert_eq!(
        log_refund_assets,
        &attr(
            "refund_assets",
            format!("100{}, 100{}", ORAI_DENOM, liquidity_addr)
        )
    );
}

#[test]
fn test_pool_whitelist_for_trader() {
    // provide more liquidity 1:2, which is not proportional to 1:1,
    // then it must accept 1:1 and treat left amount as donation
    let mut app = MockApp::new(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        &[Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(400u128),
        }],
    )]);
    app.set_token_contract(Box::new(create_entry_points_testing!(oraiswap_token)));

    app.set_oracle_contract(Box::new(create_entry_points_testing!(oraiswap_oracle)));

    app.set_token_balances(&[
        ("liquidity", &[(&MOCK_CONTRACT_ADDR.to_string(), 1000u128)]),
        ("asset", &[(&MOCK_CONTRACT_ADDR.to_string(), 1000u128)]),
        ("asset", &[("addr0000", 1000u128)]),
    ])
    .unwrap();

    let asset_addr = app.get_token_addr("asset").unwrap();

    let msg = InstantiateMsg {
        oracle_addr: app.oracle_addr.clone(),
        asset_infos: [
            AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
            AssetInfo::Token {
                contract_addr: asset_addr.clone(),
            },
        ],
        token_code_id: app.token_id(),
        commission_rate: None,
        admin: Some(Addr::unchecked("admin")),
    };

    // we can just call .unwrap() to assert this was a success
    let code_id = app.upload(Box::new(
        create_entry_points_testing!(crate).with_reply_empty(crate::contract::reply),
    ));
    let pair_addr = app
        .instantiate(code_id, Addr::unchecked("owner"), &msg, &[], "pair")
        .unwrap();

    // before enable, everyone can interactive with pool
    // set allowance
    app.execute(
        Addr::unchecked(MOCK_CONTRACT_ADDR),
        asset_addr.clone(),
        &cw20::Cw20ExecuteMsg::IncreaseAllowance {
            spender: pair_addr.to_string(),
            amount: Uint128::from(1000u128),
            expires: None,
        },
        &[],
    )
    .unwrap();
    app.execute(
        Addr::unchecked("addr0000"),
        asset_addr.clone(),
        &cw20::Cw20ExecuteMsg::IncreaseAllowance {
            spender: pair_addr.to_string(),
            amount: Uint128::from(1000u128),
            expires: None,
        },
        &[],
    )
    .unwrap();

    // successfully provide liquidity for the exist pool
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: asset_addr.clone(),
                },
                amount: Uint128::from(100u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(100u128),
            },
        ],
        slippage_tolerance: None,
        receiver: None,
    };

    let _res = app
        .execute(
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            pair_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(100u128),
            }],
        )
        .unwrap();
    // enable whitelisted pool fail
    let error = app
        .execute(
            Addr::unchecked("addr000"),
            pair_addr.clone(),
            &ExecuteMsg::EnableWhitelist { status: true },
            &[],
        )
        .unwrap_err();
    assert!(error.root_cause().to_string().contains("Unauthorized"));

    // enable whitelisted pool success
    app.execute(
        Addr::unchecked("admin"),
        pair_addr.clone(),
        &ExecuteMsg::EnableWhitelist { status: true },
        &[],
    )
    .unwrap();

    // try whitelist some trader
    app.execute(
        Addr::unchecked("admin"),
        pair_addr.clone(),
        &ExecuteMsg::RegisterTrader {
            traders: vec![Addr::unchecked(MOCK_CONTRACT_ADDR)],
        },
        &[],
    )
    .unwrap();

    // after enable, only whitelisted trader can trade
    let msg = ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: asset_addr.clone(),
                },
                amount: Uint128::from(100u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(100u128),
            },
        ],
        slippage_tolerance: None,
        receiver: None,
    };

    let error = app
        .execute(
            Addr::unchecked("addr0000"),
            pair_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(100u128),
            }],
        )
        .unwrap_err();
    assert!(error.root_cause().to_string().contains("Cannot Sub with 0"));

    // whitelist trader can join poll
    app.execute(
        Addr::unchecked(MOCK_CONTRACT_ADDR),
        pair_addr.clone(),
        &msg,
        &[Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(100u128),
        }],
    )
    .unwrap();

    // try swap failed with unregistered account
    let swap_msg = ExecuteMsg::Swap {
        offer_asset: Asset {
            info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
            amount: Uint128::from(100u128),
        },
        belief_price: None,
        max_spread: None,
        to: None,
    };

    let error = app
        .execute(
            Addr::unchecked("addr0000"),
            pair_addr.clone(),
            &swap_msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(100u128),
            }],
        )
        .unwrap_err();
    assert!(error.root_cause().to_string().contains("Cannot Sub with 0"));

    // success swap
    app.execute(
        Addr::unchecked(MOCK_CONTRACT_ADDR),
        pair_addr.clone(),
        &swap_msg,
        &[Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(100u128),
        }],
    )
    .unwrap();
}
