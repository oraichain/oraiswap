use std::str::FromStr;

use cosmwasm_std::{to_binary, Addr, Coin, Decimal, Uint128};
use oraiswap::create_entry_points_testing;
use oraiswap::testing::{AttributeUtil, MockApp, ATOM_DENOM};

use oraiswap::asset::{Asset, AssetInfo, ORAI_DENOM};
use oraiswap::limit_order::{
    Cw20HookMsg, ExecuteMsg, InstantiateMsg, LastOrderIdResponse, OrderDirection, OrderFilter,
    OrderResponse, OrdersResponse, QueryMsg, TicksResponse,
};

#[test]
fn submit_order() {
    let mut app = MockApp::new(&[(
        &"addr0000".to_string(),
        &[Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(1000000000u128),
        }],
    )]);

    app.set_token_contract(Box::new(create_entry_points_testing!(oraiswap_token)));

    let msg = InstantiateMsg {};
    let code_id = app.upload(Box::new(create_entry_points_testing!(crate)));
    let limit_order_addr = app
        .instantiate(
            code_id,
            Addr::unchecked("addr0000"),
            &msg,
            &[],
            "limit order",
        )
        .unwrap();

    app.set_token_balances(&[(
        &"asset".to_string(),
        &[(&"addr0000".to_string(), &Uint128::from(1000000000u128))],
    )]);
    let token_addr = app.get_token_addr("asset").unwrap();

    let msg = ExecuteMsg::SubmitOrder {
        direction: None,
        offer_asset: Asset {
            amount: Uint128::from(1000000u128),
            info: AssetInfo::Token {
                contract_addr: token_addr.clone(),
            },
        },
        ask_asset: Asset {
            amount: Uint128::from(1000000u128),
            info: AssetInfo::Token {
                contract_addr: token_addr.clone(),
            },
        },
    };

    let res = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[],
    );

    app.assert_fail(res);

    let msg = ExecuteMsg::SubmitOrder {
        direction: None,
        offer_asset: Asset {
            amount: Uint128::from(1000000u128),
            info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
        },
        ask_asset: Asset {
            amount: Uint128::from(1000000u128),
            info: AssetInfo::Token {
                contract_addr: token_addr.clone(),
            },
        },
    };

    let res = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[],
    );
    app.assert_fail(res);

    let msg = ExecuteMsg::SubmitOrder {
        direction: None,
        offer_asset: Asset {
            amount: Uint128::from(1000000u128),
            info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
        },
        ask_asset: Asset {
            amount: Uint128::from(1000000u128),
            info: AssetInfo::Token {
                contract_addr: token_addr.clone(),
            },
        },
    };

    let res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(1000000u128),
            }],
        )
        .unwrap();

    assert_eq!(
        res.get_attributes(1),
        vec![
            ("action", "submit_order"),
            ("order_id", "1"),
            ("bidder_addr", "addr0000"),
            ("offer_asset", &format!("1000000{}", ORAI_DENOM)),
            ("ask_asset", &format!("1000000{}", token_addr)),
            ("total_orders", "1")
        ]
    );

    assert_eq!(
        app.query::<LastOrderIdResponse, _>(limit_order_addr.clone(), &QueryMsg::LastOrderId {})
            .unwrap(),
        LastOrderIdResponse { last_order_id: 1 }
    );

    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(1000000u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: None,
            ask_asset: Asset {
                amount: Uint128::from(1000000u128),
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
            },
        })
        .unwrap(),
    };

    let res = app
        .execute(Addr::unchecked("addr0000"), token_addr.clone(), &msg, &[])
        .unwrap();

    assert_eq!(
        // due to hook from token contract, the index is 3
        res.get_attributes(3),
        vec![
            ("action", "submit_order"),
            ("order_id", "2"),
            ("bidder_addr", "addr0000"),
            ("offer_asset", &format!("1000000{}", token_addr)),
            ("ask_asset", &format!("1000000{}", ORAI_DENOM)),
            ("total_orders", "2")
        ]
    );
    assert_eq!(
        app.query::<LastOrderIdResponse, _>(limit_order_addr.clone(), &QueryMsg::LastOrderId {})
            .unwrap(),
        LastOrderIdResponse { last_order_id: 2 }
    );
}

#[test]
fn cancel_order_native_token() {
    let mut app = MockApp::new(&[(
        &"addr0000".to_string(),
        &[Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(1000000000u128),
        }],
    )]);
    app.set_token_contract(Box::new(create_entry_points_testing!(oraiswap_token)));

    app.set_token_balances(&[(
        &"asset".to_string(),
        &[(&"addr0000".to_string(), &Uint128::from(1000000000u128))],
    )]);

    let token_addr = app.get_token_addr("asset").unwrap();

    let msg = InstantiateMsg {};
    let code_id = app.upload(Box::new(create_entry_points_testing!(crate)));
    let limit_order_addr = app
        .instantiate(
            code_id,
            Addr::unchecked("addr0000"),
            &msg,
            &[],
            "limit order",
        )
        .unwrap();

    let msg = ExecuteMsg::SubmitOrder {
        direction: None,
        offer_asset: Asset {
            amount: Uint128::from(1000000u128),
            info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
        },
        ask_asset: Asset {
            amount: Uint128::from(1000000u128),
            info: AssetInfo::Token {
                contract_addr: token_addr.clone(),
            },
        },
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(1000000u128),
            }],
        )
        .unwrap();

    let msg = ExecuteMsg::CancelOrder {
        order_id: 1,
        offer_info: AssetInfo::NativeToken {
            denom: ORAI_DENOM.to_string(),
        },
        ask_info: AssetInfo::Token {
            contract_addr: token_addr.clone(),
        },
    };

    // failed verfication failed
    let res = app.execute(
        Addr::unchecked("addr0001"),
        limit_order_addr.clone(),
        &msg,
        &[],
    );
    app.assert_fail(res);

    let res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[],
        )
        .unwrap();
    assert_eq!(
        res.get_attributes(1),
        vec![
            ("action", "cancel_order"),
            ("order_id", "1"),
            ("bidder_refund", &format!("1000000{}", ORAI_DENOM)),
            ("total_orders", "0")
        ]
    );

    // failed no order exists
    let res = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[],
    );
    app.assert_fail(res);
}

#[test]
fn cancel_order_token() {
    let mut app = MockApp::new(&[(
        &"addr0000".to_string(),
        &[Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(1000000000u128),
        }],
    )]);
    app.set_token_contract(Box::new(create_entry_points_testing!(oraiswap_token)));

    app.set_token_balances(&[(
        &"asset".to_string(),
        &[(&"addr0000".to_string(), &Uint128::from(1000000000u128))],
    )]);

    let token_addr = app.get_token_addr("asset").unwrap();

    let msg = InstantiateMsg {};
    let code_id = app.upload(Box::new(create_entry_points_testing!(crate)));
    let limit_order_addr = app
        .instantiate(
            code_id,
            Addr::unchecked("addr0000"),
            &msg,
            &[],
            "limit order",
        )
        .unwrap();

    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(1000000u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: None,
            ask_asset: Asset {
                amount: Uint128::from(1000000u128),
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
            },
        })
        .unwrap(),
    };

    let _res = app
        .execute(Addr::unchecked("addr0000"), token_addr.clone(), &msg, &[])
        .unwrap();

    let msg = ExecuteMsg::CancelOrder {
        order_id: 1,
        ask_info: AssetInfo::NativeToken {
            denom: ORAI_DENOM.to_string(),
        },
        offer_info: AssetInfo::Token {
            contract_addr: token_addr.clone(),
        },
    };

    // failed verfication failed
    let res = app.execute(
        Addr::unchecked("addr0001"),
        limit_order_addr.clone(),
        &msg,
        &[],
    );
    app.assert_fail(res);

    let res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[],
        )
        .unwrap();

    assert_eq!(
        res.get_attributes(1),
        vec![
            ("action", "cancel_order"),
            ("order_id", "1"),
            ("bidder_refund", &format!("1000000{}", token_addr)),
            ("total_orders", "0")
        ]
    );

    // failed no order exists
    let res = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[],
    );
    app.assert_fail(res);
}

#[test]
fn execute_order_native_token() {
    let mut app = MockApp::new(&[
        (
            &"addr0000".to_string(),
            &[
                Coin {
                    denom: ORAI_DENOM.to_string(),
                    amount: Uint128::from(1000000u128),
                },
                Coin {
                    denom: ATOM_DENOM.to_string(),
                    amount: Uint128::from(1000000u128),
                },
            ],
        ),
        (
            &"addr0001".to_string(),
            &[
                Coin {
                    denom: ORAI_DENOM.to_string(),
                    amount: Uint128::from(1000000u128),
                },
                Coin {
                    denom: ATOM_DENOM.to_string(),
                    amount: Uint128::from(1000000u128),
                },
            ],
        ),
    ]);

    let msg = InstantiateMsg {};
    let code_id = app.upload(Box::new(create_entry_points_testing!(crate)));
    let limit_order_addr = app
        .instantiate(
            code_id,
            Addr::unchecked("addr0000"),
            &msg,
            &[],
            "limit order",
        )
        .unwrap();

    let msg = ExecuteMsg::SubmitOrder {
        direction: None,
        offer_asset: Asset {
            amount: Uint128::from(1000000u128),
            info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
        },
        ask_asset: Asset {
            amount: Uint128::from(1000000u128),
            info: AssetInfo::NativeToken {
                denom: ATOM_DENOM.to_string(),
            },
        },
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(1000000u128),
            }],
        )
        .unwrap();

    // assertion; native asset balance
    let msg = ExecuteMsg::ExecuteOrder {
        offer_info: AssetInfo::NativeToken {
            denom: ORAI_DENOM.to_string(),
        },
        ask_asset: Asset {
            amount: Uint128::new(500000u128),
            info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
        },
        order_id: 1u64,
    };

    // Native token balance mismatch between the argument and the transferred
    let res = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );
    app.assert_fail(res);

    // cannot execute order with other asset
    let res = app.execute(
        Addr::unchecked("addr0001"),
        limit_order_addr.clone(),
        &msg,
        &[Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(500000u128),
        }],
    );
    app.assert_fail(res);

    // partial execute
    let msg = ExecuteMsg::ExecuteOrder {
        offer_info: AssetInfo::NativeToken {
            denom: ORAI_DENOM.to_string(),
        },
        ask_asset: Asset {
            amount: Uint128::new(500000u128),
            info: AssetInfo::NativeToken {
                denom: ATOM_DENOM.to_string(),
            },
        },
        order_id: 1u64,
    };
    let res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ATOM_DENOM.to_string(),
                amount: Uint128::from(500000u128),
            }],
        )
        .unwrap();
    assert_eq!(
        res.get_attributes(1),
        vec![
            ("action", "execute_order"),
            ("order_id", "1"),
            ("executor_receive", &format!("500000{}", ORAI_DENOM)),
            ("bidder_receive", &format!("500000{}", ATOM_DENOM)),
            ("total_orders", "1")
        ]
    );

    let resp: OrderResponse = app
        .query(
            limit_order_addr.clone(),
            &QueryMsg::Order {
                order_id: 1,
                ask_info: AssetInfo::NativeToken {
                    denom: ATOM_DENOM.to_string(),
                },
                offer_info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
            },
        )
        .unwrap();
    assert_eq!(resp.filled_ask_amount, Uint128::new(500000u128));
    assert_eq!(resp.filled_offer_amount, Uint128::new(500000u128));

    // fill left amount
    let res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ATOM_DENOM.to_string(),
                amount: Uint128::from(500000u128),
            }],
        )
        .unwrap();
    assert_eq!(
        res.get_attributes(1),
        vec![
            ("action", "execute_order"),
            ("order_id", "1"),
            ("executor_receive", &format!("500000{}", ORAI_DENOM)),
            ("bidder_receive", &format!("500000{}", ATOM_DENOM)),
            ("total_orders", "0")
        ]
    );

    // no more existed
    assert!(app
        .query::<OrderResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Order {
                order_id: 1,
                ask_info: AssetInfo::NativeToken {
                    denom: ATOM_DENOM.to_string(),
                },
                offer_info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
            }
        )
        .is_err());
}

#[test]
fn execute_order_token() {
    let mut app = MockApp::new(&[
        (
            &"addr0000".to_string(),
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(1000000000u128),
            }],
        ),
        (
            &"addr0001".to_string(),
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(1000000u128),
            }],
        ),
    ]);
    app.set_token_contract(Box::new(create_entry_points_testing!(oraiswap_token)));

    let token_addrs = app.set_token_balances(&[
        (
            &"assetA".to_string(),
            &[
                (&"addr0000".to_string(), &Uint128::from(1000000000u128)),
                (&"addr0001".to_string(), &Uint128::from(1000000000u128)),
            ],
        ),
        (
            &"assetB".to_string(),
            &[
                (&"addr0000".to_string(), &Uint128::from(1000000000u128)),
                (&"addr0001".to_string(), &Uint128::from(1000000000u128)),
            ],
        ),
    ]);

    let msg = InstantiateMsg {};
    let code_id = app.upload(Box::new(create_entry_points_testing!(crate)));
    let limit_order_addr = app
        .instantiate(
            code_id,
            Addr::unchecked("addr0000"),
            &msg,
            &[],
            "limit order",
        )
        .unwrap();

    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(1000000u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: None,
            ask_asset: Asset {
                amount: Uint128::from(1000000u128),
                info: AssetInfo::Token {
                    contract_addr: token_addrs[1].clone(),
                },
            },
        })
        .unwrap(),
    };
    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            token_addrs[0].clone(),
            &msg,
            &[],
        )
        .unwrap();

    // cannot execute order with other asset
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(500000u128),
        msg: to_binary(&Cw20HookMsg::ExecuteOrder {
            offer_info: AssetInfo::Token {
                contract_addr: token_addrs[0].clone(),
            },
            order_id: 1u64,
        })
        .unwrap(),
    };
    let res = app.execute(
        Addr::unchecked("addr0001"),
        token_addrs[0].clone(),
        &msg,
        &[],
    );
    // invalid asset given
    app.assert_fail(res);

    // partial execute
    let res = app
        .execute(
            Addr::unchecked("addr0001"),
            token_addrs[1].clone(),
            &msg,
            &[],
        )
        .unwrap();
    assert_eq!(
        res.get_attributes(3),
        vec![
            ("action", "execute_order"),
            ("order_id", "1"),
            ("executor_receive", &format!("500000{}", token_addrs[0])),
            ("bidder_receive", &format!("500000{}", token_addrs[1])),
            ("total_orders", "1")
        ]
    );

    let resp: OrderResponse = app
        .query(
            limit_order_addr.clone(),
            &QueryMsg::Order {
                offer_info: AssetInfo::Token {
                    contract_addr: token_addrs[0].clone(),
                },
                ask_info: AssetInfo::Token {
                    contract_addr: token_addrs[1].clone(),
                },
                order_id: 1,
            },
        )
        .unwrap();

    assert_eq!(resp.filled_ask_amount, Uint128::new(500000u128));
    assert_eq!(resp.filled_offer_amount, Uint128::new(500000u128));

    // fill left amount
    let res = app
        .execute(
            Addr::unchecked("addr0000"),
            token_addrs[1].clone(),
            &msg,
            &[],
        )
        .unwrap();
    assert_eq!(
        res.get_attributes(3),
        vec![
            ("action", "execute_order"),
            ("order_id", "1"),
            ("executor_receive", &format!("500000{}", token_addrs[0])),
            ("bidder_receive", &format!("500000{}", token_addrs[1])),
            ("total_orders", "0")
        ]
    );

    assert!(app
        .query::<OrderResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Order {
                order_id: 1,
                offer_info: AssetInfo::Token {
                    contract_addr: token_addrs[0].clone(),
                },
                ask_info: AssetInfo::Token {
                    contract_addr: token_addrs[1].clone(),
                },
            }
        )
        .is_err())
}

#[test]
fn orders_querier() {
    let mut app = MockApp::new(&[
        (
            &"addr0000".to_string(),
            &[
                Coin {
                    denom: ORAI_DENOM.to_string(),
                    amount: Uint128::from(1000000u128),
                },
                Coin {
                    denom: ATOM_DENOM.to_string(),
                    amount: Uint128::from(1000000u128),
                },
            ],
        ),
        (
            &"addr0001".to_string(),
            &[
                Coin {
                    denom: ORAI_DENOM.to_string(),
                    amount: Uint128::from(1000000u128),
                },
                Coin {
                    denom: ATOM_DENOM.to_string(),
                    amount: Uint128::from(1000000u128),
                },
            ],
        ),
    ]);

    app.set_token_contract(Box::new(create_entry_points_testing!(oraiswap_token)));

    let token_addrs = app.set_token_balances(&[
        (
            &"assetA".to_string(),
            &[
                (&"addr0000".to_string(), &Uint128::from(1000000000u128)),
                (&"addr0001".to_string(), &Uint128::from(1000000000u128)),
            ],
        ),
        (
            &"assetB".to_string(),
            &[
                (&"addr0000".to_string(), &Uint128::from(1000000000u128)),
                (&"addr0001".to_string(), &Uint128::from(1000000000u128)),
            ],
        ),
    ]);

    let msg = InstantiateMsg {};
    let code_id = app.upload(Box::new(create_entry_points_testing!(crate)));
    let limit_order_addr = app
        .instantiate(
            code_id,
            Addr::unchecked("addr0000"),
            &msg,
            &[],
            "limit order",
        )
        .unwrap();

    let msg = ExecuteMsg::SubmitOrder {
        direction: None,
        offer_asset: Asset {
            amount: Uint128::from(1000000u128),
            info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
        },
        ask_asset: Asset {
            amount: Uint128::from(1000000u128),
            info: AssetInfo::NativeToken {
                denom: ATOM_DENOM.to_string(),
            },
        },
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(1000000u128),
            }],
        )
        .unwrap();

    // user sends token therefore no need to set allowance for limit order contract
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::from(1000000u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: None,
            ask_asset: Asset {
                amount: Uint128::from(1000000u128),
                info: AssetInfo::Token {
                    contract_addr: token_addrs[1].clone(),
                },
            },
        })
        .unwrap(),
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            token_addrs[0].clone(),
            &msg,
            &[],
        )
        .unwrap();

    let order_1 = OrderResponse {
        order_id: 1u64,
        bidder_addr: "addr0000".to_string(),
        offer_asset: Asset {
            amount: Uint128::from(1000000u128),
            info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
        },
        ask_asset: Asset {
            amount: Uint128::from(1000000u128),
            info: AssetInfo::NativeToken {
                denom: ATOM_DENOM.to_string(),
            },
        },
        filled_offer_amount: Uint128::zero(),
        filled_ask_amount: Uint128::zero(),
        direction: OrderDirection::Buy,
    };

    let order_2 = OrderResponse {
        order_id: 2u64,
        bidder_addr: "addr0000".to_string(),
        offer_asset: Asset {
            amount: Uint128::from(1000000u128),
            info: AssetInfo::Token {
                contract_addr: token_addrs[0].clone(),
            },
        },
        ask_asset: Asset {
            amount: Uint128::from(1000000u128),
            info: AssetInfo::Token {
                contract_addr: token_addrs[1].clone(),
            },
        },
        filled_offer_amount: Uint128::zero(),
        filled_ask_amount: Uint128::zero(),
        direction: OrderDirection::Buy,
    };

    assert_eq!(
        OrdersResponse {
            orders: vec![order_2.clone(),],
        },
        app.query::<OrdersResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Orders {
                offer_info: AssetInfo::Token {
                    contract_addr: token_addrs[0].clone(),
                },
                ask_info: AssetInfo::Token {
                    contract_addr: token_addrs[1].clone(),
                },
                filter: OrderFilter::Bidder("addr0000".to_string()),
                start_after: None,
                limit: None,
                order_by: Some(1),
            }
        )
        .unwrap()
    );

    assert_eq!(
        OrdersResponse {
            orders: vec![order_1.clone()],
        },
        app.query::<OrdersResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Orders {
                offer_info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                ask_info: AssetInfo::NativeToken {
                    denom: ATOM_DENOM.to_string(),
                },
                filter: OrderFilter::None,
                start_after: None,
                limit: None,
                order_by: Some(1),
            }
        )
        .unwrap()
    );

    // DESC test
    assert_eq!(
        OrdersResponse {
            orders: vec![order_2.clone()],
        },
        app.query::<OrdersResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Orders {
                offer_info: AssetInfo::Token {
                    contract_addr: token_addrs[0].clone(),
                },
                ask_info: AssetInfo::Token {
                    contract_addr: token_addrs[1].clone(),
                },
                filter: OrderFilter::None,
                start_after: None,
                limit: None,
                order_by: Some(2),
            }
        )
        .unwrap()
    );

    // different bidder
    assert_eq!(
        OrdersResponse { orders: vec![] },
        app.query::<OrdersResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Orders {
                offer_info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                ask_info: AssetInfo::NativeToken {
                    denom: ATOM_DENOM.to_string(),
                },
                filter: OrderFilter::Bidder("addr0001".to_string()),
                start_after: None,
                limit: None,
                order_by: None,
            }
        )
        .unwrap()
    );

    // start after DESC
    assert_eq!(
        OrdersResponse {
            orders: vec![order_1],
        },
        app.query::<OrdersResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Orders {
                offer_info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                ask_info: AssetInfo::NativeToken {
                    denom: ATOM_DENOM.to_string(),
                },
                filter: OrderFilter::None,
                start_after: Some(2u64),
                limit: None,
                order_by: Some(2),
            }
        )
        .unwrap()
    );

    // start after ASC
    assert_eq!(
        OrdersResponse { orders: vec![] },
        app.query::<OrdersResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Orders {
                offer_info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                ask_info: AssetInfo::NativeToken {
                    denom: ATOM_DENOM.to_string(),
                },
                filter: OrderFilter::None,
                start_after: Some(1u64),
                limit: None,
                order_by: Some(1),
            }
        )
        .unwrap()
    );

    // query all ticks
    let res = app
        .query::<TicksResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Ticks {
                offer_info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                ask_info: AssetInfo::NativeToken {
                    denom: ATOM_DENOM.to_string(),
                },
                start_after: None,
                limit: None,
                order_by: Some(1),
            },
        )
        .unwrap();

    for tick in res.ticks {
        let res = app
            .query::<OrdersResponse, _>(
                limit_order_addr.clone(),
                &QueryMsg::Orders {
                    offer_info: AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    ask_info: AssetInfo::NativeToken {
                        denom: ATOM_DENOM.to_string(),
                    },
                    filter: OrderFilter::Price(Decimal::from_str(&tick.price).unwrap()),
                    start_after: None,
                    limit: None,
                    order_by: Some(1),
                },
            )
            .unwrap();
        println!("{:?}", res);
    }
}
