use std::str::FromStr;

use cosmwasm_std::{to_binary, Addr, Coin, Decimal, StdError, Uint128};
use oraiswap::create_entry_points_testing;
use oraiswap::testing::{AttributeUtil, MockApp, ATOM_DENOM};

use oraiswap::asset::{Asset, AssetInfo, ORAI_DENOM};
use oraiswap::limit_order::{
    Cw20HookMsg, ExecuteMsg, InstantiateMsg, LastOrderIdResponse, OrderBookMatchableResponse,
    OrderBookResponse, OrderBooksResponse, OrderDirection, OrderFilter, OrderResponse, OrderStatus,
    OrdersResponse, QueryMsg, TicksResponse,
};

use crate::jsonstr;
const USDT_DENOM: &str = "usdt";

#[test]
fn submit_order() {
    let mut app = MockApp::new(&[
        (
            &"addr0000".to_string(),
            &[
                Coin {
                    denom: ORAI_DENOM.to_string(),
                    amount: Uint128::from(1000000000u128),
                },
                Coin {
                    denom: USDT_DENOM.to_string(),
                    amount: Uint128::from(1000000000u128),
                },
            ],
        ),
        (
            &"addr0001".to_string(),
            &[
                Coin {
                    denom: ORAI_DENOM.to_string(),
                    amount: Uint128::from(1000000000u128),
                },
                Coin {
                    denom: USDT_DENOM.to_string(),
                    amount: Uint128::from(1000000000u128),
                },
            ],
        ),
    ]);

    app.set_token_contract(Box::new(create_entry_points_testing!(oraiswap_token)));

    app.set_token_balances(&[(
        &"asset".to_string(),
        &[(&"addr0000".to_string(), &Uint128::from(1000000000u128))],
    )]);

    let token_addr = app.get_token_addr("asset").unwrap();

    let msg = InstantiateMsg {
        name: None,
        version: None,
        admin: None,
        commission_rate: None,
        reward_address: None,
        spread_address:None,
    };
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

    // create order book for pair [orai, usdt]
    let msg = ExecuteMsg::CreateOrderBookPair {
        base_coin_info: AssetInfo::NativeToken {
            denom: ORAI_DENOM.to_string(),
        },
        quote_coin_info: AssetInfo::NativeToken {
            denom: USDT_DENOM.to_string(),
        },
        spread: None,
        min_quote_coin_amount: Uint128::from(10u128),
    };
    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[],
        )
        .unwrap();

    // Create an existed order book
    let msg = ExecuteMsg::CreateOrderBookPair {
        base_coin_info: AssetInfo::NativeToken {
            denom: ORAI_DENOM.to_string(),
        },
        quote_coin_info: AssetInfo::NativeToken {
            denom: USDT_DENOM.to_string(),
        },
        spread: None,
        min_quote_coin_amount: Uint128::zero(),
    };
    let _res = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[],
    );
    app.assert_fail(_res);

    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(1000000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(1000000u128),
            },
        ],
    };

    // offer asset is null
    let res = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[],
    );
    app.assert_fail(res);

    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(5u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(50u128),
            },
        ],
    };

    // Offer ammount 5 usdt (min 10 usdt) is too low
    let res = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(50u128),
        }],
    );
    app.assert_fail(res);

    // paid 150 usdt to get 150 orai
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(150u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(150u128),
            },
        ],
    };

    let _ = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(150u128),
            }],
        )
        .unwrap();

    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(150u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(0u128),
            },
        ],
    };

    // Asset must not be zero
    let res = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[Coin {
            denom: USDT_DENOM.to_string(),
            amount: Uint128::from(0u128),
        }],
    );
    app.assert_fail(res);

    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(11111111u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(12345678u128),
            },
        ],
    };

    // paid 11111111 usdt to get 12345678 orai
    let res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(11111111u128),
            }],
        )
        .unwrap();
    println!("submit 2 {:?}", res);

    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(20000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(70000u128),
            },
        ],
    };

    // paid 70000 orai to get 20000 usdt
    let res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(70000u128),
            }],
        )
        .unwrap();
    println!("submit 3 {:?}", res);

    let order_1 = OrderResponse {
        order_id: 1u64,
        bidder_addr: "addr0000".to_string(),
        offer_asset: Asset {
            amount: Uint128::from(150u128),
            info: AssetInfo::NativeToken {
                denom: USDT_DENOM.to_string(),
            },
        },
        ask_asset: Asset {
            amount: Uint128::from(150u128),
            info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
        },
        filled_offer_amount: Uint128::zero(),
        filled_ask_amount: Uint128::zero(),
        direction: OrderDirection::Buy,
        status: OrderStatus::Open,
    };

    let order_2 = OrderResponse {
        order_id: 2u64,
        bidder_addr: "addr0000".to_string(),
        offer_asset: Asset {
            amount: Uint128::from(11111111u128),
            info: AssetInfo::NativeToken {
                denom: USDT_DENOM.to_string(),
            },
        },
        ask_asset: Asset {
            amount: Uint128::from(12345678u128),
            info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
        },
        filled_offer_amount: Uint128::zero(),
        filled_ask_amount: Uint128::zero(),
        direction: OrderDirection::Buy,
        status: OrderStatus::Open,
    };

    let order_3 = OrderResponse {
        order_id: 3u64,
        bidder_addr: "addr0000".to_string(),
        offer_asset: Asset {
            amount: Uint128::from(70000u128),
            info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
        },
        ask_asset: Asset {
            amount: Uint128::from(20000u128),
            info: AssetInfo::NativeToken {
                denom: USDT_DENOM.to_string(),
            },
        },
        filled_offer_amount: Uint128::zero(),
        filled_ask_amount: Uint128::zero(),
        direction: OrderDirection::Sell,
        status: OrderStatus::Open,
    };

    assert_eq!(
        order_3.clone(),
        app.query::<OrderResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Order {
                order_id: 3,
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: USDT_DENOM.to_string(),
                    },
                ],
            }
        )
        .unwrap()
    );

    assert_eq!(
        order_2.clone(),
        app.query::<OrderResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Order {
                order_id: 2,
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: USDT_DENOM.to_string(),
                    },
                ],
            }
        )
        .unwrap()
    );

    assert_eq!(
        order_1.clone(),
        app.query::<OrderResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Order {
                order_id: 1,
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: USDT_DENOM.to_string(),
                    },
                ],
            }
        )
        .unwrap()
    );

    // create order book for pair [orai, token_addr]
    let msg = ExecuteMsg::CreateOrderBookPair {
        base_coin_info: AssetInfo::NativeToken {
            denom: ORAI_DENOM.to_string(),
        },
        quote_coin_info: AssetInfo::Token {
            contract_addr: token_addr.clone(),
        },
        spread: None,
        min_quote_coin_amount: Uint128::zero(),
    };
    let _res = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[],
    );

    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(1212121u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Buy,
            assets: [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    amount: Uint128::from(2121212u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addr.clone(),
                    },
                    amount: Uint128::from(1212121u128),
                },
            ],
        })
        .unwrap(),
    };

    let _ = app
        .execute(Addr::unchecked("addr0000"), token_addr.clone(), &msg, &[])
        .unwrap();

    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: token_addr.clone(),
                },
                amount: Uint128::from(1111111u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(1234567u128),
            },
        ],
    };

    // paid 1234567 orai to get 1111111 token
    let _ = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(1234567u128),
            }],
        )
        .unwrap();

    let order_4 = OrderResponse {
        order_id: 4u64,
        bidder_addr: "addr0000".to_string(),
        offer_asset: Asset {
            amount: Uint128::from(1212121u128),
            info: AssetInfo::Token {
                contract_addr: token_addr.clone(),
            },
        },
        ask_asset: Asset {
            amount: Uint128::from(2121212u128),
            info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
        },
        filled_offer_amount: Uint128::zero(),
        filled_ask_amount: Uint128::zero(),
        direction: OrderDirection::Buy,
        status: OrderStatus::Open,
    };

    let order_5 = OrderResponse {
        order_id: 5u64,
        bidder_addr: "addr0000".to_string(),
        offer_asset: Asset {
            amount: Uint128::from(1234567u128),
            info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
        },
        ask_asset: Asset {
            amount: Uint128::from(1111111u128),
            info: AssetInfo::Token {
                contract_addr: token_addr.clone(),
            },
        },
        filled_offer_amount: Uint128::zero(),
        filled_ask_amount: Uint128::zero(),
        direction: OrderDirection::Sell,
        status: OrderStatus::Open,
    };

    assert_eq!(
        order_4.clone(),
        app.query::<OrderResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Order {
                order_id: 4,
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::Token {
                        contract_addr: token_addr.clone(),
                    },
                ],
            }
        )
        .unwrap()
    );

    assert_eq!(
        order_5.clone(),
        app.query::<OrderResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Order {
                order_id: 5,
                asset_infos: [
                    AssetInfo::Token {
                        contract_addr: token_addr.clone(),
                    },
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                ],
            }
        )
        .unwrap()
    );
    assert_eq!(
        app.query::<LastOrderIdResponse, _>(limit_order_addr.clone(), &QueryMsg::LastOrderId {})
            .unwrap(),
        LastOrderIdResponse { last_order_id: 5 }
    );
}

#[test]
fn cancel_order_native_token() {
    let mut app = MockApp::new(&[
        (
            &"addr0000".to_string(),
            &[
                Coin {
                    denom: ORAI_DENOM.to_string(),
                    amount: Uint128::from(1000000000u128),
                },
                Coin {
                    denom: USDT_DENOM.to_string(),
                    amount: Uint128::from(1000000000u128),
                },
            ],
        ),
        (
            &"addr0001".to_string(),
            &[
                Coin {
                    denom: ORAI_DENOM.to_string(),
                    amount: Uint128::from(1000000000u128),
                },
                Coin {
                    denom: USDT_DENOM.to_string(),
                    amount: Uint128::from(1000000000u128),
                },
            ],
        ),
    ]);

    let msg = InstantiateMsg {
        name: None,
        version: None,
        admin: None,
        commission_rate: None,
        reward_address: None,
        spread_address:None,
    };
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

    // create order book for pair [orai, atom]
    let msg = ExecuteMsg::CreateOrderBookPair {
        base_coin_info: AssetInfo::NativeToken {
            denom: ORAI_DENOM.to_string(),
        },
        quote_coin_info: AssetInfo::NativeToken {
            denom: USDT_DENOM.to_string(),
        },
        spread: None,
        min_quote_coin_amount: Uint128::zero(),
    };
    let _res = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[],
    );

    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(500000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(6666666u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(6666666u128),
            }],
        )
        .unwrap();

    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(456789u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(6666666u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(456789u128),
            }],
        )
        .unwrap();

    let msg = ExecuteMsg::CancelOrder {
        order_id: 1,
        asset_infos: [
            AssetInfo::NativeToken {
                denom: USDT_DENOM.to_string(),
            },
            AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
        ],
    };

    // verfication failed
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
            ("pair", "orai - usdt"),
            ("order_id", "1"),
            ("direction", "Buy"),
            ("status", "Cancel"),
            ("bidder_addr", "addr0000"),
            ("offer_amount", "6666666"),
            ("ask_amount", "500000"),
            ("bidder_refund", &format!("6666666{}", USDT_DENOM)),
        ]
    );

    let mut address0_balances = app.query_all_balances(Addr::unchecked("addr0000")).unwrap();
    let mut address1_balances = app.query_all_balances(Addr::unchecked("addr0001")).unwrap();
    println!("round 1 - address0_balances: {:?}", address0_balances);
    println!("round 1 - address1_balances: {:?}", address1_balances);

    let mut expected_balances: Vec<Coin> = [
        Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(1000000000u128),
        },
        Coin {
            denom: USDT_DENOM.to_string(),
            amount: Uint128::from(1000000000u128),
        },
    ]
    .to_vec();
    assert_eq!(address0_balances, expected_balances,);
    expected_balances = [
        Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(999543211u128),
        },
        Coin {
            denom: USDT_DENOM.to_string(),
            amount: Uint128::from(1000000000u128),
        },
    ]
    .to_vec();
    assert_eq!(address1_balances, expected_balances,);

    // failed no order exists
    let res = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[],
    );
    app.assert_fail(res);

    let msg = ExecuteMsg::CancelOrder {
        order_id: 2,
        asset_infos: [
            AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
            AssetInfo::NativeToken {
                denom: USDT_DENOM.to_string(),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[],
        )
        .unwrap();

    address1_balances = app.query_all_balances(Addr::unchecked("addr0001")).unwrap();
    println!("round 2 - address1_balances: {:?}", address1_balances);
    expected_balances = [
        Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(1000000000u128),
        },
        Coin {
            denom: USDT_DENOM.to_string(),
            amount: Uint128::from(1000000000u128),
        },
    ]
    .to_vec();
    assert_eq!(address1_balances, expected_balances,);

    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(1234560u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(1000000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(1234560u128),
            }],
        )
        .unwrap();

    address0_balances = app.query_all_balances(Addr::unchecked("addr0000")).unwrap();
    println!("round 3 - address0_balances: {:?}", address0_balances);
    expected_balances = [
        Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(998765440u128),
        },
        Coin {
            denom: USDT_DENOM.to_string(),
            amount: Uint128::from(1000000000u128),
        },
    ]
    .to_vec();
    assert_eq!(address0_balances, expected_balances,);

    let msg = ExecuteMsg::CancelOrder {
        order_id: 3,
        asset_infos: [
            AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
            AssetInfo::NativeToken {
                denom: USDT_DENOM.to_string(),
            },
        ],
    };

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
            ("pair", "orai - usdt"),
            ("order_id", "3"),
            ("direction", "Sell"),
            ("status", "Cancel"),
            ("bidder_addr", "addr0000"),
            ("offer_amount", "1234560"),
            ("ask_amount", "1000000"),
            ("bidder_refund", &format!("1234560{}", ORAI_DENOM)),
        ]
    );
    address0_balances = app.query_all_balances(Addr::unchecked("addr0000")).unwrap();
    println!("round 4 - address0_balances: {:?}", address0_balances);
    expected_balances = [
        Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(1000000000u128),
        },
        Coin {
            denom: USDT_DENOM.to_string(),
            amount: Uint128::from(1000000000u128),
        },
    ]
    .to_vec();
    assert_eq!(address0_balances, expected_balances,);
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

    let msg = InstantiateMsg {
        name: None,
        version: None,
        admin: None,
        commission_rate: None,
        reward_address: None,
        spread_address:None,
    };
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

    // create order book for pair [token_addrs[1], token_addrs[0]]
    let msg = ExecuteMsg::CreateOrderBookPair {
        base_coin_info: AssetInfo::Token {
            contract_addr: token_addrs[1].clone(),
        },
        quote_coin_info: AssetInfo::Token {
            contract_addr: token_addrs[0].clone(),
        },
        spread: None,
        min_quote_coin_amount: Uint128::zero(),
    };
    let _res = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[],
    );

    // create order book for pair [orai, token_addrs[1]]
    let msg = ExecuteMsg::CreateOrderBookPair {
        base_coin_info: AssetInfo::Token {
            contract_addr: token_addrs[1].clone(),
        },
        quote_coin_info: AssetInfo::NativeToken {
            denom: ORAI_DENOM.to_string(),
        },
        spread: None,
        min_quote_coin_amount: Uint128::zero(),
    };
    let _res = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[],
    );

    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(1234567u128), // Fund must be equal to offer amount
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Buy,
            assets: [
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                    amount: Uint128::from(1234567u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[1].clone(),
                    },
                    amount: Uint128::from(4567890u128),
                },
            ],
        })
        .unwrap(),
    };

    let msg2 = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(3333335u128), // Fund must be equal to offer amount
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Sell,
            assets: [
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                    amount: Uint128::from(1212121u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[1].clone(),
                    },
                    amount: Uint128::from(3333335u128),
                },
            ],
        })
        .unwrap(),
    };

    let msg3 = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(3333336u128), // Fund must be equal to offer amount
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Sell,
            assets: [
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                    amount: Uint128::from(1212121u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[1].clone(),
                    },
                    amount: Uint128::from(3333335u128),
                },
            ],
        })
        .unwrap(),
    };

    let _ = app
        .execute(
            Addr::unchecked("addr0000"),
            token_addrs[0].clone(),
            &msg,
            &[],
        )
        .unwrap();

    let _ = app
        .execute(
            Addr::unchecked("addr0000"),
            token_addrs[1].clone(),
            &msg2,
            &[],
        )
        .unwrap();

    // provided and paid asset are different
    let res = app.execute(
        Addr::unchecked("addr0001"),
        token_addrs[1].clone(),
        &msg3,
        &[],
    );
    app.assert_fail(res);

    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(1223344u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Sell,
            assets: [
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[1].clone(),
                    },
                    amount: Uint128::from(1223344u128),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    amount: Uint128::from(2334455u128),
                },
            ],
        })
        .unwrap(),
    };

    let _ = app
        .execute(
            Addr::unchecked("addr0000"),
            token_addrs[0].clone(),
            &msg,
            &[],
        )
        .unwrap();

    let msg = ExecuteMsg::CancelOrder {
        order_id: 1,
        asset_infos: [
            AssetInfo::Token {
                contract_addr: token_addrs[0].clone(),
            },
            AssetInfo::Token {
                contract_addr: token_addrs[1].clone(),
            },
        ],
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
            ("pair", "contract1 - contract0"),
            ("order_id", "1"),
            ("direction", "Buy"),
            ("status", "Cancel"),
            ("bidder_addr", "addr0000"),
            ("offer_amount", "1234567"),
            ("ask_amount", "4567890"),
            ("bidder_refund", &format!("1234567{}", token_addrs[0])),
        ]
    );

    let msg = ExecuteMsg::CancelOrder {
        order_id: 2,
        asset_infos: [
            AssetInfo::Token {
                contract_addr: token_addrs[1].clone(),
            },
            AssetInfo::Token {
                contract_addr: token_addrs[0].clone(),
            },
        ],
    };

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
            ("pair", "contract1 - contract0"),
            ("order_id", "2"),
            ("direction", "Sell"),
            ("status", "Cancel"),
            ("bidder_addr", "addr0000"),
            ("offer_amount", "3333335"),
            ("ask_amount", "1212121"),
            ("bidder_refund", &format!("3333335{}", token_addrs[1])),
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
fn execute_pair_native_token() {
    let mut app = MockApp::new(&[
        (
            &"addr0000".to_string(),
            &[
                Coin {
                    denom: ORAI_DENOM.to_string(),
                    amount: Uint128::from(1000000u128),
                },
                Coin {
                    denom: USDT_DENOM.to_string(),
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
                    denom: USDT_DENOM.to_string(),
                    amount: Uint128::from(1000000u128),
                },
            ],
        ),
        (
            &"addr0002".to_string(),
            &[
                Coin {
                    denom: ORAI_DENOM.to_string(),
                    amount: Uint128::from(1000000u128),
                },
                Coin {
                    denom: USDT_DENOM.to_string(),
                    amount: Uint128::from(1000000u128),
                },
            ],
        ),
    ]);

    let msg = InstantiateMsg {
        name: None,
        version: None,
        admin: None,
        commission_rate: None,
        reward_address: None,
        spread_address:None,
    };
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

    // Create pair [orai, usdt] for order book
    let msg = ExecuteMsg::CreateOrderBookPair {
        base_coin_info: AssetInfo::NativeToken {
            denom: ORAI_DENOM.to_string(),
        },
        quote_coin_info: AssetInfo::NativeToken {
            denom: USDT_DENOM.to_string(),
        },
        spread: None,
        min_quote_coin_amount: Uint128::from(10u128),
    };

    let _res = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[],
    );

    /* <----------------------------------- order 1 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(10000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 2 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(9700u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(10000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 3 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(13000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(14000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(13000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 4 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(5000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
        ],
    };

    // offer usdt, ask for orai
    let _res = app
        .execute(
            Addr::unchecked("addr0002"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(5000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 5 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(4400u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(8800u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0002"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(4400u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 6 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(7000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(14000u128),
            },
        ],
    };

    // offer orai, ask for atom
    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(7000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 7 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(2000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(2000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(2000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 8 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(1500u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(1200u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(1200u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 9 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(5000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(10000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 10 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(7000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(6789u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(7000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 11 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(1500u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(1000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(1000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 12 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(1600u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(1000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(1000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 13 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(1500u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(1000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(1500u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 14 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(1600u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(1000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(1600u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 15 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(10000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 16 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(9700u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(10000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 17 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(14000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(13000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(13000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 18 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(5000u128),
            },
        ],
    };

    // offer usdt, ask for orai
    let _res = app
        .execute(
            Addr::unchecked("addr0002"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(5000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 19 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(8800u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(4400u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0002"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(4400u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 20 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(14000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(7000u128),
            },
        ],
    };

    // offer orai, ask for atom
    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(7000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 21 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(2000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(2000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(2000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 22 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(1500u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(1200u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(1200u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 23 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(5000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(10000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 24 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(7000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(6789u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(7000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 25 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(1500u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(1000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(1000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 26 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(1600u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(1000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(1000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 27 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(1500u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(1000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(1500u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 28 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(1600u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(1000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(1600u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 29 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(2000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(2000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(2000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 30 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(1500u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(1200u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(1200u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 31 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(1500u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(1200u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(1200u128),
            }],
        )
        .unwrap();

    let mut address0_balances = app.query_all_balances(Addr::unchecked("addr0000")).unwrap();
    let mut address1_balances = app.query_all_balances(Addr::unchecked("addr0001")).unwrap();
    let mut address2_balances = app.query_all_balances(Addr::unchecked("addr0002")).unwrap();
    let mut reward_balances = app.query_all_balances(Addr::unchecked("orai16stq6f4pnrfpz75n9ujv6qg3czcfa4qyjux5en")).unwrap();
    let mut spread_balances = app.query_all_balances(Addr::unchecked("orai139tjpfj0h6ld3wff7v2x92ntdewungfss0ml3n")).unwrap();

    println!("round 0 - address0's balances: {:?}", address0_balances);
    println!("round 0 - address1's balances: {:?}", address1_balances);
    println!("round 0 - address2's balances: {:?}", address2_balances);
    println!("round 0 - reward_balances's balances: {:?}", reward_balances);
    println!("round 0 - spread_balances's balances: {:?}\n\n", spread_balances);
    
    let mut expected_balances: Vec<Coin> = [
        Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(960000u128),
        },
        Coin {
            denom: USDT_DENOM.to_string(),
            amount: Uint128::from(971200u128),
        },
    ]
    .to_vec();
    assert_eq!(address0_balances, expected_balances,);
    expected_balances = [
        Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(973800u128),
        },
        Coin {
            denom: USDT_DENOM.to_string(),
            amount: Uint128::from(960000u128),
        },
    ]
    .to_vec();
    assert_eq!(address1_balances, expected_balances,);
    expected_balances = [
        Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(1000000u128),
        },
        Coin {
            denom: USDT_DENOM.to_string(),
            amount: Uint128::from(981200u128),
        },
    ]
    .to_vec();
    assert_eq!(address2_balances, expected_balances,);
    expected_balances = [
    ]
    .to_vec();
    assert_eq!(spread_balances, expected_balances);

    // assertion; native asset balance
    let msg = ExecuteMsg::ExecuteOrderBookPair {
        asset_infos: [
            AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
            AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
        ],
        limit: None,
    };

    // Native token balance mismatch between the argument and the transferred
    let res = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[],
    );
    app.assert_fail(res);

    // Excecute all orders
    let msg = ExecuteMsg::ExecuteOrderBookPair {
        asset_infos: [
            AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
            AssetInfo::NativeToken {
                denom: USDT_DENOM.to_string(),
            },
        ],
        limit: Some(10),
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[],
        )
        .unwrap();
    println!("[LOG] attribute - round 1 - {:?}", _res);

    address0_balances = app.query_all_balances(Addr::unchecked("addr0000")).unwrap();
    address1_balances = app.query_all_balances(Addr::unchecked("addr0001")).unwrap();
    address2_balances = app.query_all_balances(Addr::unchecked("addr0002")).unwrap();      
    reward_balances = app.query_all_balances(Addr::unchecked("orai16stq6f4pnrfpz75n9ujv6qg3czcfa4qyjux5en")).unwrap();
    spread_balances = app.query_all_balances(Addr::unchecked("orai139tjpfj0h6ld3wff7v2x92ntdewungfss0ml3n")).unwrap();

    println!("round 1 - address0's balances: {:?}", address0_balances);
    println!("round 1 - address1's balances: {:?}", address1_balances);
    println!("round 1 - address2's balances: {:?}", address2_balances);
    println!("round 1 - reward_balances's balances: {:?}", reward_balances);
    println!("round 1 - spread_balances's balances: {:?}\n\n", spread_balances);

    expected_balances = [
        Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(969390u128),
        },
        Coin {
            denom: USDT_DENOM.to_string(),
            amount: Uint128::from(977693u128),
        },
    ]
    .to_vec();
    assert_eq!(address0_balances, expected_balances,);
    expected_balances = [
        Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(973800u128),
        },
        Coin {
            denom: USDT_DENOM.to_string(),
            amount: Uint128::from(963224u128),
        },
    ]
    .to_vec();
    assert_eq!(address1_balances, expected_balances);
    expected_balances = [
        Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(1000000u128),
        },
        Coin {
            denom: USDT_DENOM.to_string(),
            amount: Uint128::from(981200u128),
        },
    ]
    .to_vec();
    assert_eq!(address2_balances, expected_balances);

    expected_balances = [
        Coin {
            denom: USDT_DENOM.to_string(),
            amount: Uint128::from(8400u128),
        },
    ]
    .to_vec();
    assert_eq!(spread_balances, expected_balances);

    let res = app
        .query::<OrderBookMatchableResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::OrderBookMatchable {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: USDT_DENOM.to_string(),
                    },
                ],
            },
        )
        .unwrap();

    println!("[LOG] orderbook matchable: {}", jsonstr!(res));
}

#[test]
fn execute_pair_cw20_token() {
    let mut app = MockApp::new(&[
        (
            &"addr0000".to_string(),
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(1000000u128),
            }],
        ),
        (
            &"addr0001".to_string(),
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(1000000u128),
            }],
        ),
        (
            &"addr0002".to_string(),
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(1000000u128),
            }],
        ),
    ]);

    app.set_token_contract(Box::new(create_entry_points_testing!(oraiswap_token)));

    let token_addrs = app.set_token_balances(&[(
        &"usdt".to_string(),
        &[
            (&"addr0000".to_string(), &Uint128::from(1000000u128)),
            (&"addr0001".to_string(), &Uint128::from(1000000u128)),
            (&"addr0002".to_string(), &Uint128::from(1000000u128)),
        ],
    )]);

    let msg = InstantiateMsg {
        name: None,
        version: None,
        admin: None,
        commission_rate: None,
        reward_address: None,
        spread_address:None,
    };
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

    // Create pair [orai, token_addrs[0]] for order book
    let msg = ExecuteMsg::CreateOrderBookPair {
        base_coin_info: AssetInfo::NativeToken {
            denom: ORAI_DENOM.to_string(),
        },
        quote_coin_info: AssetInfo::Token {
            contract_addr: token_addrs[0].clone(),
        },
        spread: None,
        min_quote_coin_amount: Uint128::from(10u128),
    };

    let _res = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[],
    );

    /* <----------------------------------- order 1 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: token_addrs[0].clone(),
                },
                amount: Uint128::from(10000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(10000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 2 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: token_addrs[0].clone(),
                },
                amount: Uint128::from(9700u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(10000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 3 -----------------------------------> */
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(13000u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Buy,
            assets: [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    amount: Uint128::from(13000u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                    amount: Uint128::from(13000u128),
                },
            ],
        })
        .unwrap(),
    };

    let _res = app.execute(
        Addr::unchecked("addr0001"),
        token_addrs[0].clone(),
        &msg,
        &[],
    );

    /* <----------------------------------- order 4 -----------------------------------> */
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(5000u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Buy,
            assets: [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    amount: Uint128::from(10000u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                    amount: Uint128::from(5000u128),
                },
            ],
        })
        .unwrap(),
    };

    // offer usdt, ask for orai
    let _res = app
        .execute(
            Addr::unchecked("addr0002"),
            token_addrs[0].clone(),
            &msg,
            &[],
        )
        .unwrap();

    /* <----------------------------------- order 5 -----------------------------------> */
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(4400u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Buy,
            assets: [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    amount: Uint128::from(8800u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                    amount: Uint128::from(4400u128),
                },
            ],
        })
        .unwrap(),
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0002"),
            token_addrs[0].clone(),
            &msg,
            &[],
        )
        .unwrap();

    /* <----------------------------------- order 6 -----------------------------------> */
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(7000u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Buy,
            assets: [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    amount: Uint128::from(14000u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                    amount: Uint128::from(7000u128),
                },
            ],
        })
        .unwrap(),
    };

    // offer orai, ask for usdt
    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            token_addrs[0].clone(),
            &msg,
            &[],
        )
        .unwrap();

    /* <----------------------------------- order 7 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(2000u128),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: token_addrs[0].clone(),
                },
                amount: Uint128::from(2000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(2000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 8 -----------------------------------> */
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(1200u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Buy,
            assets: [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    amount: Uint128::from(1500u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                    amount: Uint128::from(1200u128),
                },
            ],
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

    /* <----------------------------------- order 9 -----------------------------------> */
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(10000u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Buy,
            assets: [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    amount: Uint128::from(5000u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                    amount: Uint128::from(10000u128),
                },
            ],
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

    /* <----------------------------------- order 10 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(7000u128),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: token_addrs[0].clone(),
                },
                amount: Uint128::from(6789u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(7000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 11 -----------------------------------> */
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(1000u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Buy,
            assets: [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    amount: Uint128::from(1500u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                    amount: Uint128::from(1000u128),
                },
            ],
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

    /* <----------------------------------- order 12 -----------------------------------> */
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(1000u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Buy,
            assets: [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    amount: Uint128::from(1600u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                    amount: Uint128::from(1000u128),
                },
            ],
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

    /* <----------------------------------- order 13 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(1500u128),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: token_addrs[0].clone(),
                },
                amount: Uint128::from(1000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(1500u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 14 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(1600u128),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: token_addrs[0].clone(),
                },
                amount: Uint128::from(1000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(1600u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 15 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: token_addrs[0].clone(),
                },
                amount: Uint128::from(10000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(10000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 16 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: token_addrs[0].clone(),
                },
                amount: Uint128::from(9700u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(10000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 17 -----------------------------------> */
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(13000u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Buy,
            assets: [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    amount: Uint128::from(14000u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                    amount: Uint128::from(13000u128),
                },
            ],
        })
        .unwrap(),
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            token_addrs[0].clone(),
            &msg,
            &[],
        )
        .unwrap();

    /* <----------------------------------- order 18 -----------------------------------> */
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(5000u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Buy,
            assets: [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    amount: Uint128::from(10000u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                    amount: Uint128::from(5000u128),
                },
            ],
        })
        .unwrap(),
    };

    // offer usdt, ask for orai
    let _res = app
        .execute(
            Addr::unchecked("addr0002"),
            token_addrs[0].clone(),
            &msg,
            &[],
        )
        .unwrap();

    /* <----------------------------------- order 19 -----------------------------------> */
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(4400u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Buy,
            assets: [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    amount: Uint128::from(8800u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                    amount: Uint128::from(4400u128),
                },
            ],
        })
        .unwrap(),
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0002"),
            token_addrs[0].clone(),
            &msg,
            &[],
        )
        .unwrap();

    /* <----------------------------------- order 20 -----------------------------------> */
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(7000u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Buy,
            assets: [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    amount: Uint128::from(14000u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                    amount: Uint128::from(7000u128),
                },
            ],
        })
        .unwrap(),
    };

    // offer cw20 usdt, ask for orai
    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            token_addrs[0].clone(),
            &msg,
            &[],
        )
        .unwrap();

    /* <----------------------------------- order 21 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(2000u128),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: token_addrs[0].clone(),
                },
                amount: Uint128::from(2000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(2000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 22 -----------------------------------> */
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(1200u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Buy,
            assets: [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    amount: Uint128::from(1500u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                    amount: Uint128::from(1200u128),
                },
            ],
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

    /* <----------------------------------- order 23 -----------------------------------> */
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(10000u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Buy,
            assets: [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    amount: Uint128::from(5000u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                    amount: Uint128::from(10000u128),
                },
            ],
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

    /* <----------------------------------- order 24 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(7000u128),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: token_addrs[0].clone(),
                },
                amount: Uint128::from(6789u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(7000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 25 -----------------------------------> */
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(1000u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Buy,
            assets: [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    amount: Uint128::from(1500u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                    amount: Uint128::from(1000u128),
                },
            ],
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

    /* <----------------------------------- order 26 -----------------------------------> */
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(1000u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Buy,
            assets: [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    amount: Uint128::from(1600u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                    amount: Uint128::from(1000u128),
                },
            ],
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

    /* <----------------------------------- order 27 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(1500u128),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: token_addrs[0].clone(),
                },
                amount: Uint128::from(1000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(1500u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 28 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(1600u128),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: token_addrs[0].clone(),
                },
                amount: Uint128::from(1000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(1600u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 29 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(2000u128),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: token_addrs[0].clone(),
                },
                amount: Uint128::from(2000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(2000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 30 -----------------------------------> */
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(1200u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Buy,
            assets: [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    amount: Uint128::from(1500u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                    amount: Uint128::from(1200u128),
                },
            ],
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

    /* <----------------------------------- order 31 -----------------------------------> */
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(1200u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Buy,
            assets: [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    amount: Uint128::from(1500u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                    amount: Uint128::from(1200u128),
                },
            ],
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

    let mut address0_balances = app.query_all_balances(Addr::unchecked("addr0000")).unwrap();
    let mut address1_balances = app.query_all_balances(Addr::unchecked("addr0001")).unwrap();
    let mut address2_balances = app.query_all_balances(Addr::unchecked("addr0002")).unwrap();
    println!("round 0 - address0's balances: {:?}", address0_balances);
    println!("round 0 - address1's balances: {:?}", address1_balances);
    println!("round 0 - address2's balances: {:?}\n\n", address2_balances);

    let mut expected_balances: Vec<Coin> = [Coin {
        denom: ORAI_DENOM.to_string(),
        amount: Uint128::from(960000u128),
    }]
    .to_vec();
    assert_eq!(address0_balances, expected_balances,);
    expected_balances = [Coin {
        denom: ORAI_DENOM.to_string(),
        amount: Uint128::from(973800u128),
    }]
    .to_vec();
    assert_eq!(address1_balances, expected_balances,);
    expected_balances = [Coin {
        denom: ORAI_DENOM.to_string(),
        amount: Uint128::from(1000000u128),
    }]
    .to_vec();
    assert_eq!(address2_balances, expected_balances,);

    // assertion; native asset balance
    let msg = ExecuteMsg::ExecuteOrderBookPair {
        asset_infos: [
            AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
            AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
        ],
        limit: None,
    };

    // Native token balance mismatch between the argument and the transferred
    let res = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[],
    );
    app.assert_fail(res);

    // Excecute all orders
    let msg = ExecuteMsg::ExecuteOrderBookPair {
        asset_infos: [
            AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
            AssetInfo::Token {
                contract_addr: token_addrs[0].clone(),
            },
        ],
        limit: None,
    };

    let _ = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[],
    );

    address0_balances = app.query_all_balances(Addr::unchecked("addr0000")).unwrap();
    address1_balances = app.query_all_balances(Addr::unchecked("addr0001")).unwrap();
    address2_balances = app.query_all_balances(Addr::unchecked("addr0002")).unwrap();
    println!("round 1 - address0's balances: {:?}", address0_balances);
    println!("round 1 - address1's balances: {:?}", address1_balances);
    println!("round 1 - address2's balances: {:?}\n\n", address2_balances);

    expected_balances = [Coin {
        denom: ORAI_DENOM.to_string(),
        amount: Uint128::from(969390u128),
    }]
    .to_vec();
    assert_eq!(address0_balances, expected_balances,);
    expected_balances = [Coin {
        denom: ORAI_DENOM.to_string(),
        amount: Uint128::from(986487u128),
    }]
    .to_vec();
    assert_eq!(address1_balances, expected_balances,);
    expected_balances = [Coin {
        denom: ORAI_DENOM.to_string(),
        amount: Uint128::from(1000000u128),
    }]
    .to_vec();
    assert_eq!(address2_balances, expected_balances,);
}

/// Test for spread parameter of orderbook pair
/// Example: If pair ORAI/USDT has spread = 10%,
/// it mean matching engine will not match orders if buy_price <= (sell_price*(1 + 10%))
/// Therefore, we need to find the highest suitable buy price and lowest suitable sell price
/// Not the Highest and Lowest price in orderbook
#[test]
fn spread_test() {
    let mut app = MockApp::new(&[
        (
            &"addr0000".to_string(),
            &[
                Coin {
                    denom: ORAI_DENOM.to_string(),
                    amount: Uint128::from(1000000u128),
                },
                Coin {
                    denom: USDT_DENOM.to_string(),
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
                    denom: USDT_DENOM.to_string(),
                    amount: Uint128::from(1000000u128),
                },
            ],
        ),
        (
            &"addr0002".to_string(),
            &[
                Coin {
                    denom: ORAI_DENOM.to_string(),
                    amount: Uint128::from(1000000u128),
                },
                Coin {
                    denom: USDT_DENOM.to_string(),
                    amount: Uint128::from(1000000u128),
                },
            ],
        ),
    ]);

    let msg = InstantiateMsg {
        name: None,
        version: None,
        admin: None,
        commission_rate: None,
        reward_address: None,
        spread_address:None,
    };
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

    // Create pair [orai, usdt] for order book
    let msg = ExecuteMsg::CreateOrderBookPair {
        base_coin_info: AssetInfo::NativeToken {
            denom: ORAI_DENOM.to_string(),
        },
        quote_coin_info: AssetInfo::NativeToken {
            denom: USDT_DENOM.to_string(),
        },
        spread: Some(Decimal::percent(10)),
        min_quote_coin_amount: Uint128::from(10u128),
    };

    let _res = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[],
    );

    /* <----------------------------------- order 1 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(20000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(10000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 2 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(30000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(10000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 3 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(15000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(10000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 4 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(41000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(10000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 5 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(19000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(10000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 6 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(44800u128),
            },
        ],
    };

    // offer usdt, ask for orai
    let _res = app
        .execute(
            Addr::unchecked("addr0002"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(44800u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 7 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(14000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(28100u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(28100u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 8 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(50000u128),
            },
        ],
    };

    // offer usdt, ask for orai
    let _res = app
        .execute(
            Addr::unchecked("addr0002"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(50000u128),
            }],
        )
        .unwrap();

    let mut address0_balances = app.query_all_balances(Addr::unchecked("addr0000")).unwrap();
    let mut address1_balances = app.query_all_balances(Addr::unchecked("addr0001")).unwrap();
    let mut address2_balances = app.query_all_balances(Addr::unchecked("addr0002")).unwrap();
    println!("round 0 - address0's balances: {:?}", address0_balances);
    println!("round 0 - address1's balances: {:?}", address1_balances);
    println!("round 0 - address2's balances: {:?}\n\n", address2_balances);

    let mut expected_balances: Vec<Coin> = [
        Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(980000u128),
        },
        Coin {
            denom: USDT_DENOM.to_string(),
            amount: Uint128::from(1000000u128),
        },
    ]
    .to_vec();
    assert_eq!(address0_balances, expected_balances,);
    expected_balances = [
        Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(970000u128),
        },
        Coin {
            denom: USDT_DENOM.to_string(),
            amount: Uint128::from(971900u128),
        },
    ]
    .to_vec();
    assert_eq!(address1_balances, expected_balances,);
    expected_balances = [
        Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(1000000u128),
        },
        Coin {
            denom: USDT_DENOM.to_string(),
            amount: Uint128::from(905200u128),
        },
    ]
    .to_vec();
    assert_eq!(address2_balances, expected_balances,);

    // assertion; native asset balance
    let msg = ExecuteMsg::ExecuteOrderBookPair {
        asset_infos: [
            AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
            AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
        ],
        limit: None,
    };

    // Native token balance mismatch between the argument and the transferred
    let res = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[],
    );
    app.assert_fail(res);

    // Excecute all orders
    let msg = ExecuteMsg::ExecuteOrderBookPair {
        asset_infos: [
            AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
            AssetInfo::NativeToken {
                denom: USDT_DENOM.to_string(),
            },
        ],
        limit: None,
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[],
        )
        .unwrap();
    println!("[LOG] attribute - round 1 - {:?}", _res);

    address0_balances = app.query_all_balances(Addr::unchecked("addr0000")).unwrap();
    address1_balances = app.query_all_balances(Addr::unchecked("addr0001")).unwrap();
    address2_balances = app.query_all_balances(Addr::unchecked("addr0002")).unwrap();
    println!("round 1 - address0's balances: {:?}", address0_balances);
    println!("round 1 - address1's balances: {:?}", address1_balances);
    println!("round 1 - address2's balances: {:?}\n\n", address2_balances);

    expected_balances = [
        Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(980000u128),
        },
        Coin {
            denom: USDT_DENOM.to_string(),
            amount: Uint128::from(1019380u128),
        },
    ]
    .to_vec();
    assert_eq!(address0_balances, expected_balances,);
    expected_balances = [
        Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(979690u128),
        },
        Coin {
            denom: USDT_DENOM.to_string(),
            amount: Uint128::from(1004846u128),
        },
    ]
    .to_vec();
    assert_eq!(address1_balances, expected_balances,);
    expected_balances = [
        Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(1019380u128),
        },
        Coin {
            denom: USDT_DENOM.to_string(),
            amount: Uint128::from(905200u128),
        },
    ]
    .to_vec();
    assert_eq!(address2_balances, expected_balances,);
}

#[test]
fn reward_to_executor_test() {
    let mut app = MockApp::new(&[
        (
            &"addr0000".to_string(),
            &[
                Coin {
                    denom: ORAI_DENOM.to_string(),
                    amount: Uint128::from(1000000000u128),
                },
                Coin {
                    denom: USDT_DENOM.to_string(),
                    amount: Uint128::from(1000000000u128),
                },
            ],
        ),
        (
            &"addr0001".to_string(),
            &[
                Coin {
                    denom: ORAI_DENOM.to_string(),
                    amount: Uint128::from(1000000000u128),
                },
                Coin {
                    denom: USDT_DENOM.to_string(),
                    amount: Uint128::from(1000000000u128),
                },
            ],
        ),
    ]);

    let msg = InstantiateMsg {
        name: None,
        version: None,
        admin: None,
        commission_rate: None,
        reward_address: None,
        spread_address:None,
    };
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

    // Create pair [orai, usdt] for order book
    let msg = ExecuteMsg::CreateOrderBookPair {
        base_coin_info: AssetInfo::NativeToken {
            denom: ORAI_DENOM.to_string(),
        },
        quote_coin_info: AssetInfo::NativeToken {
            denom: USDT_DENOM.to_string(),
        },
        spread: Some(Decimal::percent(10)),
        min_quote_coin_amount: Uint128::from(10000u128),
    };

    let _res = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[],
    );

    /* <----------------------------------- order 1 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(103000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(618000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(103000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 2 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(610000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(100000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(100000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 3 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(100000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(600000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(600000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 4 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(610000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(100000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(610000u128),
            }],
        )
        .unwrap();

    let mut address0_balances = app.query_all_balances(Addr::unchecked("addr0000")).unwrap();
    let mut address1_balances = app.query_all_balances(Addr::unchecked("addr0001")).unwrap();
    println!("round 0 - address0's balances: {:?}", address0_balances);
    println!("round 0 - address1's balances: {:?}\n\n", address1_balances);

    let mut expected_balances: Vec<Coin> = [
        Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(1000000000u128),
        },
        Coin {
            denom: USDT_DENOM.to_string(),
            amount: Uint128::from(999797000u128),
        },
    ]
    .to_vec();
    assert_eq!(address0_balances, expected_balances,);
    expected_balances = [
        Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(998790000u128),
        },
        Coin {
            denom: USDT_DENOM.to_string(),
            amount: Uint128::from(1000000000u128),
        },
    ]
    .to_vec();
    assert_eq!(address1_balances, expected_balances,);

    // assertion; native asset balance
    let msg = ExecuteMsg::ExecuteOrderBookPair {
        asset_infos: [
            AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
            AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
        ],
        limit: None,
    };

    // Native token balance mismatch between the argument and the transferred
    let res = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[],
    );
    app.assert_fail(res);

    // Excecute all orders
    let msg = ExecuteMsg::ExecuteOrderBookPair {
        asset_infos: [
            AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
            AssetInfo::NativeToken {
                denom: USDT_DENOM.to_string(),
            },
        ],
        limit: None,
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[],
        )
        .unwrap();
    println!("[LOG] attribute - round 1 - {:?}", _res);

    address0_balances = app.query_all_balances(Addr::unchecked("addr0000")).unwrap();
    address1_balances = app.query_all_balances(Addr::unchecked("addr0001")).unwrap();
    println!("round 1 - address0's balances: {:?}", address0_balances);
    println!("round 1 - address1's balances: {:?}\n\n", address1_balances);

    expected_balances = [
        Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(1000617082u128),
        },
        Coin {
            denom: USDT_DENOM.to_string(),
            amount: Uint128::from(999797000u128),
        },
    ]
    .to_vec();
    assert_eq!(address0_balances, expected_balances,);
    expected_balances = [
        Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(998790000u128),
        },
        Coin {
            denom: USDT_DENOM.to_string(),
            amount: Uint128::from(1000101135u128),
        },
    ]
    .to_vec();
    assert_eq!(address1_balances, expected_balances,);
}

fn mock_basic_query_data() -> (MockApp, Addr) {
    let mut app = MockApp::new(&[
        (
            &"addr0000".to_string(),
            &[
                Coin {
                    denom: ORAI_DENOM.to_string(),
                    amount: Uint128::from(1000000u128),
                },
                Coin {
                    denom: USDT_DENOM.to_string(),
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
                    denom: USDT_DENOM.to_string(),
                    amount: Uint128::from(1000000u128),
                },
            ],
        ),
        (
            &"addr0002".to_string(),
            &[
                Coin {
                    denom: ORAI_DENOM.to_string(),
                    amount: Uint128::from(1000000u128),
                },
                Coin {
                    denom: USDT_DENOM.to_string(),
                    amount: Uint128::from(1000000u128),
                },
            ],
        ),
    ]);

    let msg = InstantiateMsg {
        name: None,
        version: None,
        admin: None,
        commission_rate: None,
        reward_address: None,
        spread_address:None,
    };
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

    // Create pair [orai, usdt] for order book
    let msg = ExecuteMsg::CreateOrderBookPair {
        base_coin_info: AssetInfo::NativeToken {
            denom: ORAI_DENOM.to_string(),
        },
        quote_coin_info: AssetInfo::NativeToken {
            denom: USDT_DENOM.to_string(),
        },
        spread: Some(Decimal::percent(10)),
        min_quote_coin_amount: Uint128::from(10u128),
    };

    let _res = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[],
    );
    (app, limit_order_addr)
}

#[test]
fn query_matchable() {
    let (mut app, limit_order_addr) = mock_basic_query_data();

    /* <----------------------------------- order 1 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(20000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(10000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 2 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(30000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(10000u128),
            }],
        )
        .unwrap();

    let res = app
        .query::<OrderBookMatchableResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::OrderBookMatchable {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: USDT_DENOM.to_string(),
                    },
                ],
            },
        )
        .unwrap();

    let expected_res = OrderBookMatchableResponse {
        is_matchable: false,
    };
    assert_eq!(res, expected_res);
    println!("[LOG] [1] orderbook matchable: {}", jsonstr!(res));

    /* <----------------------------------- order 3 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(44800u128),
            },
        ],
    };

    // offer usdt, ask for orai
    let _res = app
        .execute(
            Addr::unchecked("addr0002"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(44800u128),
            }],
        )
        .unwrap();

    let res = app
        .query::<OrderBookMatchableResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::OrderBookMatchable {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: USDT_DENOM.to_string(),
                    },
                ],
            },
        )
        .unwrap();

    let expected_res = OrderBookMatchableResponse {
        is_matchable: false,
    };
    assert_eq!(res, expected_res);
    println!("[LOG] [2] orderbook matchable: {}", jsonstr!(res));

    /* <----------------------------------- order 4 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(22000u128),
            },
        ],
    };

    // offer usdt, ask for orai
    let _res = app
        .execute(
            Addr::unchecked("addr0002"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: USDT_DENOM.to_string(),
                amount: Uint128::from(22000u128),
            }],
        )
        .unwrap();

    let res = app
        .query::<OrderBookMatchableResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::OrderBookMatchable {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: USDT_DENOM.to_string(),
                    },
                ],
            },
        )
        .unwrap();

    let expected_res = OrderBookMatchableResponse { is_matchable: true };
    assert_eq!(res, expected_res);
    println!("[LOG] [3] orderbook matchable: {}", jsonstr!(res));
}

#[test]
fn remove_orderbook_pair() {
    let mut app = MockApp::new(&[
        (
            &"addr0000".to_string(),
            &[
                Coin {
                    denom: ATOM_DENOM.to_string(),
                    amount: Uint128::from(1000000u128),
                },
                Coin {
                    denom: ORAI_DENOM.to_string(),
                    amount: Uint128::from(1000000u128),
                },
            ],
        ),
        (
            &"addr0001".to_string(),
            &[
                Coin {
                    denom: ATOM_DENOM.to_string(),
                    amount: Uint128::from(1000000u128),
                },
                Coin {
                    denom: ORAI_DENOM.to_string(),
                    amount: Uint128::from(1000000u128),
                },
            ],
        ),
        (
            &"addr0002".to_string(),
            &[
                Coin {
                    denom: ATOM_DENOM.to_string(),
                    amount: Uint128::from(1000000u128),
                },
                Coin {
                    denom: ORAI_DENOM.to_string(),
                    amount: Uint128::from(1000000u128),
                },
            ],
        ),
    ]);

    let msg = InstantiateMsg {
        name: None,
        version: None,
        admin: None,
        commission_rate: None,
        reward_address: None,
        spread_address:None,
    };

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

    // Create pair [orai, atom] for order book
    let msg = ExecuteMsg::CreateOrderBookPair {
        base_coin_info: AssetInfo::NativeToken {
            denom: ATOM_DENOM.to_string(),
        },
        quote_coin_info: AssetInfo::NativeToken {
            denom: ORAI_DENOM.to_string(),
        },
        spread: None,
        min_quote_coin_amount: Uint128::zero(),
    };

    let _res = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[],
    );

    /* <----------------------------------- order 1 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ATOM_DENOM.to_string(),
                },
                amount: Uint128::from(11111u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(12345u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ATOM_DENOM.to_string(),
                amount: Uint128::from(11111u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 2 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ATOM_DENOM.to_string(),
                },
                amount: Uint128::from(12222u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(9700u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ATOM_DENOM.to_string(),
                amount: Uint128::from(12222u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 3 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ATOM_DENOM.to_string(),
                },
                amount: Uint128::from(14000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(13000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(13000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 4 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ATOM_DENOM.to_string(),
                },
                amount: Uint128::from(1900u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(1499u128),
            },
        ],
    };

    // offer orai, ask for atom
    let _res = app
        .execute(
            Addr::unchecked("addr0002"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(1499u128),
            }],
        )
        .unwrap();

    let order_3 = OrderResponse {
        order_id: 3u64,
        bidder_addr: "addr0001".to_string(),
        offer_asset: Asset {
            amount: Uint128::from(13000u128),
            info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
        },
        ask_asset: Asset {
            amount: Uint128::from(14000u128),
            info: AssetInfo::NativeToken {
                denom: ATOM_DENOM.to_string(),
            },
        },
        filled_offer_amount: Uint128::zero(),
        filled_ask_amount: Uint128::zero(),
        direction: OrderDirection::Buy,
        status: OrderStatus::Open,
    };

    assert_eq!(
        order_3,
        app.query::<OrderResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Order {
                order_id: 3,
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ATOM_DENOM.to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                ],
            }
        )
        .unwrap()
    );

    // remove order book for pair [orai, atom]
    let msg = ExecuteMsg::RemoveOrderBookPair {
        asset_infos: [
            AssetInfo::NativeToken {
                denom: ATOM_DENOM.to_string(),
            },
            AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
        ],
    };

    let res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[],
        )
        .unwrap();

    println!("remove order book pair res: {:?}", res);

    let res = app
        .query::<OrdersResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Orders {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ATOM_DENOM.to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                ],
                direction: None,
                filter: OrderFilter::None,
                start_after: None,
                limit: None,
                order_by: None,
            },
        )
        .unwrap_err();
    assert_eq!(
        res,
        StdError::GenericErr {
            msg: "Querier contract error: oraiswap_limit_order::orderbook::OrderBook not found"
                .to_string()
        }
    );
    let res = app
        .query::<OrderResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Order {
                order_id: 3,
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ATOM_DENOM.to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                ],
            },
        )
        .unwrap_err();
    assert_eq!(
        res,
        StdError::GenericErr {
            msg: "Querier contract error: oraiswap_limit_order::orderbook::OrderBook not found"
                .to_string()
        }
    );
}

#[test]
fn orders_querier() {
    let mut app = MockApp::new(&[
        (
            &"addr0000".to_string(),
            &[
                Coin {
                    denom: ATOM_DENOM.to_string(),
                    amount: Uint128::from(1000000000u128),
                },
                Coin {
                    denom: ORAI_DENOM.to_string(),
                    amount: Uint128::from(1000000000u128),
                },
            ],
        ),
        (
            &"addr0001".to_string(),
            &[
                Coin {
                    denom: ATOM_DENOM.to_string(),
                    amount: Uint128::from(1000000000u128),
                },
                Coin {
                    denom: ORAI_DENOM.to_string(),
                    amount: Uint128::from(1000000000u128),
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

    let msg = InstantiateMsg {
        name: None,
        version: None,
        admin: None,
        commission_rate: None,
        reward_address: None,
        spread_address:None,
    };
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

    // create order book for pair [orai, atom]
    let msg = ExecuteMsg::CreateOrderBookPair {
        base_coin_info: AssetInfo::NativeToken {
            denom: ATOM_DENOM.to_string(),
        },
        quote_coin_info: AssetInfo::NativeToken {
            denom: ORAI_DENOM.to_string(),
        },
        spread: Some(Decimal::percent(10)),
        min_quote_coin_amount: Uint128::from(10u128),
    };
    let _res = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[],
    );

    // create order book for pair [token_addrs[1], token_addrs[0]]
    let msg = ExecuteMsg::CreateOrderBookPair {
        base_coin_info: AssetInfo::Token {
            contract_addr: token_addrs[1].clone(),
        },
        quote_coin_info: AssetInfo::Token {
            contract_addr: token_addrs[0].clone(),
        },
        spread: None,
        min_quote_coin_amount: Uint128::zero(),
    };
    let _res = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[],
    );

    // query orderbooks
    let res = app
        .query::<OrderBookResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::OrderBook {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: ATOM_DENOM.to_string(),
                    },
                ],
            },
        )
        .unwrap();
    println!("[LOG] 1st orderbooks :{}", jsonstr!(res));

    // query all orderbooks
    let res = app
        .query::<OrderBooksResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::OrderBooks {
                start_after: None,
                limit: None,
                order_by: None,
            },
        )
        .unwrap();

    println!("orderbooks :{}", jsonstr!(res));

    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ATOM_DENOM.to_string(),
                },
                amount: Uint128::from(1000000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(1000000u128),
            },
        ],
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
            direction: OrderDirection::Buy,
            assets: [
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[1].clone(),
                    },
                    amount: Uint128::from(1000000u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                    amount: Uint128::from(1000000u128),
                },
            ],
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

    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::from(12345678u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Sell,
            assets: [
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                    amount: Uint128::from(11223344u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[1].clone(),
                    },
                    amount: Uint128::from(12345678u128),
                },
            ],
        })
        .unwrap(),
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
            token_addrs[0].clone(),
            &msg,
            &[],
        )
        .unwrap();

    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::from(22334455u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Sell,
            assets: [
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[1].clone(),
                    },
                    amount: Uint128::from(22334455u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                    amount: Uint128::from(22000000u128),
                },
            ],
        })
        .unwrap(),
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0001"),
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
        status: OrderStatus::Open,
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
        status: OrderStatus::Open,
    };

    let all_order = OrdersResponse {
        orders: [
            OrderResponse {
                order_id: 4u64,
                direction: OrderDirection::Sell,
                bidder_addr: "addr0001".to_string(),
                offer_asset: Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[1].clone(),
                    },
                    amount: Uint128::from(22334455u128),
                },
                ask_asset: Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                    amount: Uint128::from(22000000u128),
                },
                filled_offer_amount: Uint128::zero(),
                filled_ask_amount: Uint128::zero(),
                status: OrderStatus::Open,
            },
            OrderResponse {
                order_id: 3u64,
                direction: OrderDirection::Sell,
                bidder_addr: "addr0001".to_string(),
                offer_asset: Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[1].clone(),
                    },
                    amount: Uint128::from(12345678u128),
                },
                ask_asset: Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                    amount: Uint128::from(11223344u128),
                },
                filled_offer_amount: Uint128::zero(),
                filled_ask_amount: Uint128::zero(),
                status: OrderStatus::Open,
            },
            OrderResponse {
                order_id: 2u64,
                direction: OrderDirection::Buy,
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
                status: OrderStatus::Open,
            },
        ]
        .to_vec(),
    };

    assert_eq!(
        OrdersResponse {
            orders: vec![order_2.clone(),],
        },
        app.query::<OrdersResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Orders {
                asset_infos: [
                    AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                    AssetInfo::Token {
                        contract_addr: token_addrs[1].clone(),
                    },
                ],
                direction: None,
                filter: OrderFilter::Bidder("addr0000".to_string()),
                start_after: None,
                limit: None,
                order_by: Some(1),
            }
        )
        .unwrap()
    );

    let test = app
        .query::<OrdersResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Orders {
                asset_infos: [
                    AssetInfo::Token {
                        contract_addr: token_addrs[1].clone(),
                    },
                    AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                ],
                direction: Some(OrderDirection::Buy),
                filter: OrderFilter::None,
                start_after: None,
                limit: None,
                order_by: None,
            },
        )
        .unwrap();
    println!("[LOG] [1] - query all buy order: {}", jsonstr!(test));

    let test = app
        .query::<OrdersResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Orders {
                asset_infos: [
                    AssetInfo::Token {
                        contract_addr: token_addrs[1].clone(),
                    },
                    AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                ],
                direction: Some(OrderDirection::Sell), //None
                filter: OrderFilter::None,
                start_after: None,
                limit: None,
                order_by: None,
            },
        )
        .unwrap();
    println!("[LOG] [2] - query all sell order: {}", jsonstr!(test));

    let test = app
        .query::<OrdersResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Orders {
                asset_infos: [
                    AssetInfo::Token {
                        contract_addr: token_addrs[1].clone(),
                    },
                    AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                ],
                direction: None,
                filter: OrderFilter::None,
                start_after: None,
                limit: None,
                order_by: None,
            },
        )
        .unwrap();
    println!("[LOG] [3] - query all order: {}", jsonstr!(test));

    assert_eq!(
        OrdersResponse {
            orders: vec![order_1.clone()],
        },
        app.query::<OrdersResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Orders {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: ATOM_DENOM.to_string(),
                    },
                ],
                direction: None,
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
        all_order.clone(),
        app.query::<OrdersResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Orders {
                asset_infos: [
                    AssetInfo::Token {
                        contract_addr: token_addrs[1].clone(),
                    },
                    AssetInfo::Token {
                        contract_addr: token_addrs[0].clone(),
                    },
                ],
                direction: None,
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
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ATOM_DENOM.to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                ],
                direction: None,
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
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: ATOM_DENOM.to_string(),
                    },
                ],
                direction: None,
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
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: ATOM_DENOM.to_string(),
                    },
                ],
                direction: None,
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
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: ATOM_DENOM.to_string(),
                    },
                ],
                direction: OrderDirection::Buy,
                start_after: None,
                end: None,
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
                    asset_infos: [
                        AssetInfo::NativeToken {
                            denom: ORAI_DENOM.to_string(),
                        },
                        AssetInfo::NativeToken {
                            denom: ATOM_DENOM.to_string(),
                        },
                    ],
                    direction: None,
                    filter: OrderFilter::Price(tick.price),
                    start_after: None,
                    limit: None,
                    order_by: Some(1),
                },
            )
            .unwrap();
        println!("{:?}", res);
    }
}

#[test]
fn test_query_ticks_start_after() {
    let (mut app, limit_order_addr) = mock_basic_query_data();

    /* <----------------------------------- order 1 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(20000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(10000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 2 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(30000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(10000u128),
            }],
        )
        .unwrap();

    let result = app
        .query::<TicksResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Ticks {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: USDT_DENOM.to_string(),
                    },
                ],
                direction: OrderDirection::Sell,
                start_after: Some(Decimal::from_str("3").unwrap()),
                end: None,
                limit: None,
                order_by: Some(2),
            },
        )
        .unwrap();
    assert_eq!(result.ticks.len(), 1);

    let result = app
        .query::<TicksResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Ticks {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: USDT_DENOM.to_string(),
                    },
                ],
                direction: OrderDirection::Sell,
                start_after: Some(Decimal::from_str("2").unwrap()),
                end: None,
                limit: None,
                order_by: Some(1),
            },
        )
        .unwrap();
    assert_eq!(result.ticks.len(), 1);
}

#[test]
fn test_unwrap_default_check_sub_uint128() {
    let result = Uint128::from(0u64)
        .checked_sub(Uint128::from(1u64))
        .unwrap_or_default();
    assert_eq!(result, Uint128::from(0u64));
}

#[test]
fn test_query_ticks_with_end() {
    let (mut app, limit_order_addr) = mock_basic_query_data();

    /* <----------------------------------- order 1 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(20000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(10000u128),
            }],
        )
        .unwrap();

    /* <----------------------------------- order 2 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(10000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(30000u128),
            },
        ],
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(10000u128),
            }],
        )
        .unwrap();

    let result = app
        .query::<TicksResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Ticks {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: USDT_DENOM.to_string(),
                    },
                ],
                direction: OrderDirection::Sell,
                start_after: Some(Decimal::from_str("3").unwrap()),
                end: Some(Decimal::from_str("2").unwrap()),
                limit: None,
                order_by: Some(2),
            },
        )
        .unwrap();
    assert_eq!(result.ticks.len(), 1);
    assert_eq!(result.ticks[0].price, Decimal::from_str("2").unwrap());

    let result = app
        .query::<TicksResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Ticks {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: USDT_DENOM.to_string(),
                    },
                ],
                direction: OrderDirection::Sell,
                start_after: Some(Decimal::from_str("2").unwrap()),
                end: Some(Decimal::from_str("3").unwrap()),
                limit: None,
                order_by: Some(1),
            },
        )
        .unwrap();
    assert_eq!(result.ticks.len(), 1);
    assert_eq!(result.ticks[0].price, Decimal::from_str("3").unwrap());
}
