use std::str::FromStr;

use cosmwasm_std::testing::mock_dependencies;
use cosmwasm_std::{to_binary, Addr, Coin, Decimal, StdError, Uint128};
use oraiswap::create_entry_points_testing;
use oraiswap::math::DecimalPlaces;
use oraiswap::testing::{AttributeUtil, MockApp, ATOM_DENOM};

use oraiswap::asset::{Asset, AssetInfo, AssetInfoRaw, ORAI_DENOM};
use oraiswap::limit_order::{
    BaseAmountResponse, ContractInfo, ContractInfoResponse, Cw20HookMsg, ExecuteMsg,
    InstantiateMsg, LastOrderIdResponse, OrderBookMatchableResponse, OrderBookResponse,
    OrderBooksResponse, OrderDirection, OrderFilter, OrderResponse, OrderStatus, OrdersResponse,
    QueryMsg, TicksResponse,
};

use crate::jsonstr;
use crate::order::{get_market_asset, get_paid_and_quote_assets};
use crate::orderbook::OrderBook;
const USDT_DENOM: &str = "usdt";

fn basic_fixture() -> (MockApp, Addr) {
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

    let msg = InstantiateMsg {
        name: None,
        version: None,
        admin: None,
        commission_rate: None,
        reward_address: None,
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
    (app, limit_order_addr)
}

#[test]
fn test_get_paid_and_quote_assets() {
    let deps = mock_dependencies();
    let asset_infos_raw = [
        AssetInfoRaw::NativeToken {
            denom: ORAI_DENOM.to_string(),
        },
        AssetInfoRaw::NativeToken {
            denom: USDT_DENOM.to_string(),
        },
    ];
    let assets = asset_infos_raw.clone().map(|info| Asset {
        info: info.to_normal(deps.as_ref().api).unwrap(),
        amount: Uint128::zero(),
    });
    let orderbook: OrderBook = OrderBook {
        base_coin_info: asset_infos_raw[0].clone(),
        quote_coin_info: asset_infos_raw[1].clone(),
        spread: None,
        min_quote_coin_amount: Uint128::zero(),
    };
    // case 1: buy with base coin = asset_infos[0]
    let (paid_assets, quote_asset) = get_paid_and_quote_assets(
        deps.as_ref().api,
        &orderbook,
        assets.clone(),
        OrderDirection::Buy,
    )
    .unwrap();
    assert_eq!(paid_assets[0].info, assets[1].info);
    assert_eq!(paid_assets[1].info, assets[0].info);
    assert_eq!(quote_asset.info, assets[1].info);

    // case 2: sell with base coin = assets[0]
    let (paid_assets, quote_asset) = get_paid_and_quote_assets(
        deps.as_ref().api,
        &orderbook,
        assets.clone(),
        OrderDirection::Sell,
    )
    .unwrap();
    assert_eq!(paid_assets[0].info, assets[0].info);
    assert_eq!(paid_assets[1].info, assets[1].info);
    assert_eq!(quote_asset.info, assets[1].info);

    // case 3: buy with base coin = asset_infos[1]
    let mut reverse_assets = assets.clone();
    reverse_assets.reverse();
    let (paid_assets, quote_asset) = get_paid_and_quote_assets(
        deps.as_ref().api,
        &orderbook,
        reverse_assets.clone(),
        OrderDirection::Buy,
    )
    .unwrap();
    assert_eq!(paid_assets[0].info, reverse_assets[0].info);
    assert_eq!(paid_assets[1].info, reverse_assets[1].info);
    assert_eq!(quote_asset.info, reverse_assets[0].info);

    // case 4: sell with base coin = asset_infos[1]
    let mut reverse_assets = assets.clone();
    reverse_assets.reverse();
    let (paid_assets, quote_asset) = get_paid_and_quote_assets(
        deps.as_ref().api,
        &orderbook,
        reverse_assets.clone(),
        OrderDirection::Sell,
    )
    .unwrap();
    assert_eq!(paid_assets[0].info, reverse_assets[1].info);
    assert_eq!(paid_assets[1].info, reverse_assets[0].info);
    assert_eq!(quote_asset.info, reverse_assets[0].info);
}

#[test]
fn test_get_market_assets_buy_side() {
    let deps = mock_dependencies();
    let asset_infos_raw = [
        AssetInfoRaw::NativeToken {
            denom: ORAI_DENOM.to_string(),
        },
        AssetInfoRaw::NativeToken {
            denom: USDT_DENOM.to_string(),
        },
    ];
    let assets = asset_infos_raw.clone().map(|info| Asset {
        info: info.to_normal(deps.as_ref().api).unwrap(),
        amount: Uint128::zero(),
    });
    let orderbook: OrderBook = OrderBook {
        base_coin_info: asset_infos_raw[0].clone(),
        quote_coin_info: asset_infos_raw[1].clone(),
        spread: None,
        min_quote_coin_amount: Uint128::zero(),
    };

    // case 1: buy with base_amount = 100_000, market_price = 1.0
    let market_price = Decimal::from_str("1.0").unwrap();
    let base_amount = Uint128::from(100_000u128);
    let expected_quote_amount = Uint128::from(base_amount * market_price);

    let (paid_assets, quote_asset) = get_market_asset(
        deps.as_ref().api,
        &orderbook,
        OrderDirection::Buy,
        market_price,
        base_amount,
    )
    .unwrap();
    // assert info
    assert_eq!(paid_assets[0].info, assets[1].info);
    assert_eq!(paid_assets[1].info, assets[0].info);
    assert_eq!(quote_asset.info, assets[1].info);

    // assert quote and base amount
    assert_eq!(quote_asset.amount, expected_quote_amount);
    assert_eq!(base_amount, paid_assets[1].amount);

    // case 3: buy with base_amount = 123_456_789, market_price = 1.234
    let market_price = Decimal::from_str("1.234").unwrap();
    let base_amount = Uint128::from(123_456_789u128);
    let expected_quote_amount = Uint128::from(base_amount * market_price);

    let (paid_assets, quote_asset) = get_market_asset(
        deps.as_ref().api,
        &orderbook,
        OrderDirection::Buy,
        market_price,
        base_amount,
    )
    .unwrap();
    // assert info
    assert_eq!(paid_assets[0].info, assets[1].info);
    assert_eq!(paid_assets[1].info, assets[0].info);
    assert_eq!(quote_asset.info, assets[1].info);

    // assert quote and base amount
    assert_eq!(quote_asset.amount, expected_quote_amount);
    assert_eq!(base_amount, paid_assets[1].amount);

    // case 3: buy with base_amount = 111_222, market_price = 2.0,
    let market_price = Decimal::from_str("2.0").unwrap();
    let base_amount = Uint128::from(111_222u128);
    let expected_quote_amount = Uint128::from(base_amount * market_price);

    let (paid_assets, quote_asset) = get_market_asset(
        deps.as_ref().api,
        &orderbook,
        OrderDirection::Buy,
        market_price,
        base_amount,
    )
    .unwrap();
    // assert info
    assert_eq!(paid_assets[0].info, assets[1].info);
    assert_eq!(paid_assets[1].info, assets[0].info);
    assert_eq!(quote_asset.info, assets[1].info);

    // assert quote and base amount
    assert_eq!(quote_asset.amount, expected_quote_amount);
    assert_eq!(base_amount, paid_assets[1].amount);
}

#[test]
fn test_get_market_assets_sell_side() {
    let deps = mock_dependencies();
    let asset_infos_raw = [
        AssetInfoRaw::NativeToken {
            denom: ORAI_DENOM.to_string(),
        },
        AssetInfoRaw::NativeToken {
            denom: USDT_DENOM.to_string(),
        },
    ];
    let assets = asset_infos_raw.clone().map(|info| Asset {
        info: info.to_normal(deps.as_ref().api).unwrap(),
        amount: Uint128::zero(),
    });
    let orderbook: OrderBook = OrderBook {
        base_coin_info: asset_infos_raw[0].clone(),
        quote_coin_info: asset_infos_raw[1].clone(),
        spread: None,
        min_quote_coin_amount: Uint128::zero(),
    };

    // case 1: sell with base_amount = 100_000, market_price = 3.0
    let market_price = Decimal::from_str("3.0").unwrap();
    let base_amount = Uint128::from(100_000u128);
    let expected_quote_amount = Uint128::from(base_amount * market_price);

    let (paid_assets, quote_asset) = get_market_asset(
        deps.as_ref().api,
        &orderbook,
        OrderDirection::Sell,
        market_price,
        base_amount,
    )
    .unwrap();
    // assert info
    assert_eq!(paid_assets[0].info, assets[0].info);
    assert_eq!(paid_assets[1].info, assets[1].info);
    assert_eq!(quote_asset.info, assets[1].info);

    // assert quote and base amount
    assert_eq!(quote_asset.amount, expected_quote_amount);
    assert_eq!(base_amount, paid_assets[0].amount);

    // case 2: buy with base_amount = 123_456_789, market_price = 1.234
    let market_price = Decimal::from_str("1.234").unwrap();
    let base_amount = Uint128::from(123_456_789u128);
    let expected_quote_amount = Uint128::from(base_amount * market_price);

    let (paid_assets, quote_asset) = get_market_asset(
        deps.as_ref().api,
        &orderbook,
        OrderDirection::Sell,
        market_price,
        base_amount,
    )
    .unwrap();
    // assert info
    assert_eq!(paid_assets[0].info, assets[0].info);
    assert_eq!(paid_assets[1].info, assets[1].info);
    assert_eq!(quote_asset.info, assets[1].info);

    // assert quote and base amount
    assert_eq!(quote_asset.amount, expected_quote_amount);
    assert_eq!(base_amount, paid_assets[0].amount);

    // case 3: buy with base_amount = 111_222, market_price = 2.0
    let market_price = Decimal::from_str("2.0").unwrap();
    let base_amount = Uint128::from(111_222u128);
    let expected_quote_amount = Uint128::from(base_amount * market_price);

    let (paid_assets, quote_asset) = get_market_asset(
        deps.as_ref().api,
        &orderbook,
        OrderDirection::Sell,
        market_price,
        base_amount,
    )
    .unwrap();
    // assert info
    assert_eq!(paid_assets[0].info, assets[0].info);
    assert_eq!(paid_assets[1].info, assets[1].info);
    assert_eq!(quote_asset.info, assets[1].info);

    // assert quote and base amount
    assert_eq!(quote_asset.amount, expected_quote_amount);
    assert_eq!(base_amount, paid_assets[0].amount);
}

#[test]
fn test_withdraw_token() {
    let (mut app, limit_order_addr) = basic_fixture();
    // case 1: try to withdraw tokens using non-admin addr => unauthorized
    let asset_infos = [
        AssetInfo::NativeToken {
            denom: ORAI_DENOM.to_string(),
        },
        AssetInfo::NativeToken {
            denom: USDT_DENOM.to_string(),
        },
    ];
    let asset = Asset {
        info: asset_infos.first().unwrap().clone(),
        amount: Uint128::from(10u128),
    };
    let update_msg = ExecuteMsg::WithdrawToken {
        asset: asset.clone(),
    };
    assert_eq!(
        app.execute(
            Addr::unchecked("theft"),
            limit_order_addr.clone(),
            &update_msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(10u128), // deposit some tokens into the contract so we can mock withdrawing tokens
            }]
        )
        .is_err(),
        true
    );

    // case 2: good case, admin should be able to withdraw tokens
    let result = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &update_msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(10u128), // deposit some tokens into the contract so we can mock withdrawing tokens
            }],
        )
        .unwrap();
    let info: ContractInfoResponse = app
        .query(limit_order_addr, &QueryMsg::ContractInfo {})
        .unwrap();
    for event in result.events {
        if event.ty.eq("wasm") {
            for attr in event.attributes.clone() {
                if attr.key.eq("action") {
                    assert_eq!(attr.value, "withdraw_token");
                }
                if attr.key.eq("token") {
                    assert_eq!(attr.value, asset.to_string());
                }
            }
        }
        if event.ty.eq("transfer") {
            for attr in event.attributes {
                if attr.key.eq("recipient") {
                    assert_eq!(attr.value, info.admin.to_string())
                }
            }
        }
    }
}

#[test]
fn test_update_orderbook_data() {
    let (mut app, limit_order_addr) = basic_fixture();
    // case 1: try to update orderbook spread with non-admin addr => unauthorized
    let asset_infos = [
        AssetInfo::NativeToken {
            denom: ORAI_DENOM.to_string(),
        },
        AssetInfo::NativeToken {
            denom: USDT_DENOM.to_string(),
        },
    ];
    let update_msg = ExecuteMsg::UpdateOrderbookPair {
        asset_infos: asset_infos.clone(),
        spread: Some(Decimal::from_str("0.1").unwrap()),
        min_quote_coin_amount: None,
    };
    assert_eq!(
        app.execute(
            Addr::unchecked("theft"),
            limit_order_addr.clone(),
            &update_msg,
            &[]
        )
        .is_err(),
        true
    );

    // case 2: good case, admin should update spread from None to something
    let spread = Decimal::from_str("0.1").unwrap();
    let update_msg = ExecuteMsg::UpdateOrderbookPair {
        asset_infos: asset_infos.clone(),
        spread: Some(spread),
        min_quote_coin_amount: None,
    };
    app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &update_msg,
        &[],
    )
    .unwrap();
    let orderbook: OrderBookResponse = app
        .query(
            limit_order_addr.clone(),
            &QueryMsg::OrderBook {
                asset_infos: asset_infos.clone(),
            },
        )
        .unwrap();
    assert_eq!(orderbook.spread, Some(spread));
    // double check, make sure other fields are still the same
    assert_eq!(orderbook.base_coin_info, asset_infos[0]);
    assert_eq!(orderbook.quote_coin_info, asset_infos[1]);
    assert_eq!(orderbook.min_quote_coin_amount, Uint128::from(10u128));
}

#[test]
fn test_query_mid_price() {
    let (mut app, limit_order_addr) = basic_fixture();
    let mid_price = app
        .query::<Decimal, _>(
            limit_order_addr.clone(),
            &QueryMsg::MidPrice {
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
    assert_eq!(mid_price, Decimal::zero());
    // paid 300 usdt to get 150 orai -> 1 ORAI = 2 USD
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
                amount: Uint128::from(300u128),
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
                amount: Uint128::from(300u128),
            }],
        )
        .unwrap();

    let mid_price = app
        .query::<Decimal, _>(
            limit_order_addr.clone(),
            &QueryMsg::MidPrice {
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
    assert_eq!(mid_price, Decimal::from_ratio(1u128, 1u128));
    // now we sell to get a different mid price
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
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
                amount: Uint128::from(1500u128),
            },
        ],
    };

    let _ = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(150u128),
            }],
        )
        .unwrap();

    let mid_price = app
        .query::<Decimal, _>(
            limit_order_addr.clone(),
            &QueryMsg::MidPrice {
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
    assert_eq!(mid_price, Decimal::from_ratio(6u128, 1u128));
}

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
fn submit_order_with_spread_native_token() {
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

    let msg = InstantiateMsg {
        name: None,
        version: None,
        admin: None,
        commission_rate: None,
        reward_address: None,
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
        spread: Some(Decimal::percent(10)),
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

    let mut assets = [
        Asset {
            info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
            amount: Uint128::from(100u128),
        },
        Asset {
            info: AssetInfo::NativeToken {
                denom: USDT_DENOM.to_string(),
            },
            amount: Uint128::from(500u128),
        },
    ];
    let asset_infos = assets.clone().map(|asset| asset.info);
    // CASE 1: submit first order on buy side => no check spread price, buy_price = 5
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: assets.clone(),
    };

    let _ = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: assets[1].info.to_string(),
                amount: assets[1].amount.clone(),
            }],
        )
        .unwrap();

    // query buy ticks - buy side has one tick = 5
    let ticks = app
        .query::<TicksResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Ticks {
                asset_infos: asset_infos.clone(),
                direction: OrderDirection::Buy,
                start_after: None,
                end: None,
                limit: None,
                order_by: Some(1),
            },
        )
        .unwrap();

    // assert price
    assert_eq!(
        ticks.ticks[0].price,
        Decimal::from_ratio(assets[1].amount, assets[0].amount)
    );

    // CASE 2: submit first order on sell side => no check spread price, sell_price = 6
    assets[1].amount = Uint128::from(600u128);
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: assets.clone(),
    };

    let _ = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: asset_infos[0].to_string(),
                amount: assets[0].amount,
            }],
        )
        .unwrap();

    // query sell ticks - sell side has one tick = 6
    let ticks = app
        .query::<TicksResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Ticks {
                asset_infos: asset_infos.clone(),
                direction: OrderDirection::Sell,
                start_after: None,
                end: None,
                limit: None,
                order_by: Some(2),
            },
        )
        .unwrap();

    // assert price
    assert_eq!(
        ticks.ticks[0].price,
        Decimal::from_ratio(assets[1].amount, assets[0].amount)
    );

    // CASE 3: submit buy order out of spread
    // buy with price = 6.7 (out of spread = 6.6) => buy with price ~ 6.6
    assets[1].amount = Uint128::from(670u128);
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: assets.clone(),
    };
    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: asset_infos[1].to_string(),
                amount: assets[1].amount,
            }],
        )
        .unwrap();

    // query buy ticks - buy side has:
    // 1. tick = 5
    // 2. tick ~ 6.6
    let ticks = app
        .query::<TicksResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Ticks {
                asset_infos: asset_infos.clone(),
                direction: OrderDirection::Buy,
                start_after: None,
                end: None,
                limit: None,
                order_by: Some(1),
            },
        )
        .unwrap();
    assert_eq!(ticks.ticks.len(), 2);
    // Second price ~ 6.6 because submit price = 6.7 is out of spread, price = lowest_sell_price * (1 + spread) = 6.6
    assert_eq!(
        ticks.ticks[1].price.limit_decimal_places(Some(1)).unwrap(),
        Decimal::from_ratio(66u128, 10u128)
    );

    // CASE 4: submit sell order out of spread
    // sell with price = 4.5 (out of spread = 5.97) => submit price ~ 5.97
    assets[1].amount = Uint128::from(450u128);
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: assets.clone(),
    };
    let _ = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: asset_infos[0].to_string(),
                amount: assets[0].amount,
            }],
        )
        .unwrap();

    // query sell ticks - buy side has:
    // 1. tick = 6
    // 2. tick ~ 5.97
    let ticks = app
        .query::<TicksResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Ticks {
                asset_infos: asset_infos.clone(),
                direction: OrderDirection::Sell,
                start_after: None,
                end: None,
                limit: None,
                order_by: Some(2),
            },
        )
        .unwrap();

    assert_eq!(ticks.ticks.len(), 2);

    // Second price ~ 5.94 because submit price = 4.5 is out of spread, price = lowest_sell_price * (1 + spread) = 5.94
    assert_eq!(
        ticks.ticks[1].price.limit_decimal_places(Some(2)).unwrap(),
        Decimal::from_ratio(597u128, 100u128)
    );

    // CASE 5: submit sell order in spread
    // buy with price = 6.5 (in spread = 6.6)
    assets[1].amount = Uint128::from(650u128);
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: assets.clone(),
    };
    let _ = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: asset_infos[1].to_string(),
                amount: assets[1].amount,
            }],
        )
        .unwrap();

    // query buy ticks - buy side has:
    // 1. tick = 5
    // 2. tick = 6.5
    // 3. tick ~ 6.6
    let ticks = app
        .query::<TicksResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Ticks {
                asset_infos: asset_infos.clone(),
                direction: OrderDirection::Buy,
                start_after: None,
                end: None,
                limit: None,
                order_by: Some(1),
            },
        )
        .unwrap();
    assert_eq!(ticks.ticks.len(), 3);
    // Fisrt price = 5
    assert_eq!(ticks.ticks[0].price, Decimal::from_ratio(500u128, 100u128));
    // Second price = 6.5 because of submit price in spread range
    assert_eq!(ticks.ticks[1].price, Decimal::from_ratio(650u128, 100u128));
    // Third price ~ 6.6
    assert_eq!(
        ticks.ticks[2].price.limit_decimal_places(Some(1)).unwrap(),
        Decimal::from_ratio(66u128, 10u128)
    );

    // CASE 6: submit sell order in spread
    // sell with price = 6 (in spread = 5.97)
    assets[1].amount = Uint128::from(600u128);
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: assets.clone(),
    };

    let _ = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: asset_infos[0].to_string(),
                amount: assets[0].amount,
            }],
        )
        .unwrap();

    // query sell ticks - buy side has:
    // 1. tick = 6 with 2 orders
    // 2. tick ~ 5.94
    let ticks = app
        .query::<TicksResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Ticks {
                asset_infos: asset_infos.clone(),
                direction: OrderDirection::Sell,
                start_after: None,
                end: None,
                limit: None,
                order_by: Some(2),
            },
        )
        .unwrap();

    assert_eq!(ticks.ticks.len(), 2);
    // first price
    assert_eq!(ticks.ticks[0].price, Decimal::from_ratio(600u128, 100u128));
    // Second price
    assert_eq!(
        ticks.ticks[1].price.limit_decimal_places(Some(2)).unwrap(),
        Decimal::from_ratio(597u128, 100u128)
    );
}

#[test]
fn submit_order_with_spread_cw20_token() {
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

    let usdt_token = app.set_token_balances(&[(
        &"asset".to_string(),
        &[
            (&"addr0000".to_string(), &Uint128::from(1000000000u128)),
            (&"addr0001".to_string(), &Uint128::from(1000000000u128)),
        ],
    )]);

    let msg = InstantiateMsg {
        name: None,
        version: None,
        admin: None,
        commission_rate: None,
        reward_address: None,
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

    // create order book for pair [orai, usdt_token]
    let msg = ExecuteMsg::CreateOrderBookPair {
        base_coin_info: AssetInfo::NativeToken {
            denom: ORAI_DENOM.to_string(),
        },
        quote_coin_info: AssetInfo::Token {
            contract_addr: usdt_token[0].clone(),
        },
        spread: Some(Decimal::percent(10)),
        min_quote_coin_amount: Uint128::zero(),
    };
    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[],
        )
        .unwrap();

    // CASE 1: submit first order on buy side => no check spread price, buy_price = 5
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(500u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Buy,
            assets: [
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: usdt_token[0].clone(),
                    },
                    amount: Uint128::from(500u128),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    amount: Uint128::from(100u128),
                },
            ],
        })
        .unwrap(),
    };

    let _ = app
        .execute(
            Addr::unchecked("addr0000"),
            usdt_token[0].clone(),
            &msg,
            &[],
        )
        .unwrap();

    // query buy ticks - buy side has one tick = 5
    let ticks = app
        .query::<TicksResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Ticks {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::Token {
                        contract_addr: usdt_token[0].clone(),
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

    // assert price
    assert_eq!(ticks.ticks[0].price, Decimal::from_ratio(500u128, 100u128));

    let orders = app
        .query::<OrdersResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Orders {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::Token {
                        contract_addr: usdt_token[0].clone(),
                    },
                ],
                direction: None,
                filter: OrderFilter::Price(ticks.ticks[0].price),
                start_after: None,
                limit: None,
                order_by: Some(1),
            },
        )
        .unwrap();

    let order_1 = OrderResponse {
        order_id: 1u64,
        bidder_addr: "addr0000".to_string(),
        offer_asset: Asset {
            amount: Uint128::from(500u128),
            info: AssetInfo::Token {
                contract_addr: usdt_token[0].clone(),
            },
        },
        ask_asset: Asset {
            amount: Uint128::from(100u128),
            info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
        },
        filled_offer_amount: Uint128::zero(),
        filled_ask_amount: Uint128::zero(),
        direction: OrderDirection::Buy,
        status: OrderStatus::Open,
    };
    assert_eq!(order_1.clone(), orders.orders[0]);

    // CASE 2: submit first order on sell side => no check spread price, sell_price = 6
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: usdt_token[0].clone(),
                },
                amount: Uint128::from(600u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(100u128),
            },
        ],
    };
    let _ = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(100u128),
            }],
        )
        .unwrap();

    // query sell ticks - sell side has one tick = 6
    let ticks = app
        .query::<TicksResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Ticks {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::Token {
                        contract_addr: usdt_token[0].clone(),
                    },
                ],
                direction: OrderDirection::Sell,
                start_after: None,
                end: None,
                limit: None,
                order_by: Some(2),
            },
        )
        .unwrap();

    // assert price
    assert_eq!(ticks.ticks[0].price, Decimal::from_ratio(600u128, 100u128));

    let order_2 = OrderResponse {
        order_id: 2u64,
        bidder_addr: "addr0001".to_string(),
        offer_asset: Asset {
            amount: Uint128::from(100u128),
            info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
        },
        ask_asset: Asset {
            amount: Uint128::from(600u128),
            info: AssetInfo::Token {
                contract_addr: usdt_token[0].clone(),
            },
        },
        filled_offer_amount: Uint128::zero(),
        filled_ask_amount: Uint128::zero(),
        direction: OrderDirection::Sell,
        status: OrderStatus::Open,
    };
    let orders = app
        .query::<OrdersResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Orders {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::Token {
                        contract_addr: usdt_token[0].clone(),
                    },
                ],
                direction: None,
                filter: OrderFilter::Price(ticks.ticks[0].price),
                start_after: None,
                limit: None,
                order_by: Some(1),
            },
        )
        .unwrap();

    // assert order
    assert_eq!(order_2.clone(), orders.orders[0]);

    // CASE 3: submit buy order out of spread
    // buy with price = 6.7 (out of spread = 6.6) => buy with price ~ 6.6
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(67000u128),
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
                        contract_addr: usdt_token[0].clone(),
                    },
                    amount: Uint128::from(67000u128),
                },
            ],
        })
        .unwrap(),
    };
    let _ = app
        .execute(
            Addr::unchecked("addr0000"),
            usdt_token[0].clone(),
            &msg,
            &[],
        )
        .unwrap();

    // query buy ticks - buy side has:
    // 1. tick = 5
    // 2. tick ~ 6.6
    let ticks = app
        .query::<TicksResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Ticks {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::Token {
                        contract_addr: usdt_token[0].clone(),
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

    // Fisrt price = 5
    assert_eq!(ticks.ticks[0].price, Decimal::from_ratio(500u128, 100u128));

    // Second price ~ 6.6 because submit price = 6.7 is out of spread, price = lowest_sell_price * (1 + spread) = 6.6
    assert_eq!(
        ticks.ticks[1].price,
        Decimal::from_ratio(67000u128, 10151u128)
    );

    let order_3 = OrderResponse {
        order_id: 3u64,
        bidder_addr: "addr0000".to_string(),
        offer_asset: Asset {
            amount: Uint128::from(67000u128),
            info: AssetInfo::Token {
                contract_addr: usdt_token[0].clone(),
            },
        },
        ask_asset: Asset {
            amount: Uint128::from(10151u128),
            info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
        },
        filled_offer_amount: Uint128::zero(),
        filled_ask_amount: Uint128::zero(),
        direction: OrderDirection::Buy,
        status: OrderStatus::Open,
    };

    let orders = app
        .query::<OrdersResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Orders {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::Token {
                        contract_addr: usdt_token[0].clone(),
                    },
                ],
                direction: None,
                filter: OrderFilter::Price(ticks.ticks[1].price),
                start_after: None,
                limit: None,
                order_by: Some(1),
            },
        )
        .unwrap();

    // assert order
    assert_eq!(order_3.clone(), orders.orders[0]);

    // CASE 4: submit sell order out of spread
    // sell with price = 4.5 (out of spread = 5.94) => submit price ~ 5.94
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
                    contract_addr: usdt_token[0].clone(),
                },
                amount: Uint128::from(45000u128),
            },
        ],
    };
    let _ = app
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

    // query sell ticks - buy side has:
    // 1. tick = 6
    // 2. tick ~ 5.94
    let ticks = app
        .query::<TicksResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Ticks {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::Token {
                        contract_addr: usdt_token[0].clone(),
                    },
                ],
                direction: OrderDirection::Sell,
                start_after: None,
                end: None,
                limit: None,
                order_by: Some(2),
            },
        )
        .unwrap();

    // first price
    assert_eq!(ticks.ticks[0].price, Decimal::from_ratio(600u128, 100u128));
    // Second price ~ 5.94 because submit price = 6.7 is out of spread, price = lowest_sell_price * (1 + spread) = 6.6
    assert_eq!(
        ticks.ticks[1].price,
        Decimal::from_ratio(59403u128, 10000u128)
    );

    let order_4 = OrderResponse {
        order_id: 4u64,
        bidder_addr: "addr0001".to_string(),
        offer_asset: Asset {
            amount: Uint128::from(10000u128),
            info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
        },
        ask_asset: Asset {
            amount: Uint128::from(59403u128),
            info: AssetInfo::Token {
                contract_addr: usdt_token[0].clone(),
            },
        },
        filled_offer_amount: Uint128::zero(),
        filled_ask_amount: Uint128::zero(),
        direction: OrderDirection::Sell,
        status: OrderStatus::Open,
    };
    let orders = app
        .query::<OrdersResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Orders {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::Token {
                        contract_addr: usdt_token[0].clone(),
                    },
                ],
                direction: None,
                filter: OrderFilter::Price(ticks.ticks[1].price),
                start_after: None,
                limit: None,
                order_by: Some(1),
            },
        )
        .unwrap();
    // assert order
    assert_eq!(order_4.clone(), orders.orders[0]);

    // CASE 5: submit sell order in spread
    // buy with price = 6.5 (in spread = 6.6)
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: Uint128::new(650u128),
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Buy,
            assets: [
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: usdt_token[0].clone(),
                    },
                    amount: Uint128::from(650u128),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    amount: Uint128::from(100u128),
                },
            ],
        })
        .unwrap(),
    };
    let _ = app
        .execute(
            Addr::unchecked("addr0000"),
            usdt_token[0].clone(),
            &msg,
            &[],
        )
        .unwrap();

    // query buy ticks - buy side has:
    // 1. tick = 5
    // 2. tick = 6.5
    // 3. tick ~ 6.6
    let ticks = app
        .query::<TicksResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Ticks {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::Token {
                        contract_addr: usdt_token[0].clone(),
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

    // Fisrt price = 5
    assert_eq!(ticks.ticks[0].price, Decimal::from_ratio(500u128, 100u128));

    // Second price = 6.5 because of submit price in spread range
    assert_eq!(ticks.ticks[1].price, Decimal::from_ratio(650u128, 100u128));

    // Third price ~ 6.6
    assert_eq!(
        ticks.ticks[2].price,
        Decimal::from_ratio(67000u128, 10151u128)
    );

    let order_5 = OrderResponse {
        order_id: 5u64,
        bidder_addr: "addr0000".to_string(),
        offer_asset: Asset {
            amount: Uint128::from(650u128),
            info: AssetInfo::Token {
                contract_addr: usdt_token[0].clone(),
            },
        },
        ask_asset: Asset {
            amount: Uint128::from(100u128),
            info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
        },
        filled_offer_amount: Uint128::zero(),
        filled_ask_amount: Uint128::zero(),
        direction: OrderDirection::Buy,
        status: OrderStatus::Open,
    };

    let orders = app
        .query::<OrdersResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Orders {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::Token {
                        contract_addr: usdt_token[0].clone(),
                    },
                ],
                direction: None,
                filter: OrderFilter::Price(ticks.ticks[1].price),
                start_after: None,
                limit: None,
                order_by: Some(1),
            },
        )
        .unwrap();

    // assert order
    assert_eq!(order_5.clone(), orders.orders[0]);

    // CASE 6: submit sell order in spread
    // sell with price = 6 (in spread = 5.94)
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: usdt_token[0].clone(),
                },
                amount: Uint128::from(600u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(100u128),
            },
        ],
    };
    let _ = app
        .execute(
            Addr::unchecked("addr0001"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(100u128),
            }],
        )
        .unwrap();

    // query sell ticks - buy side has:
    // 1. tick = 6 with 2 orders
    // 2. tick ~ 5.94
    let ticks = app
        .query::<TicksResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Ticks {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::Token {
                        contract_addr: usdt_token[0].clone(),
                    },
                ],
                direction: OrderDirection::Sell,
                start_after: None,
                end: None,
                limit: None,
                order_by: Some(2),
            },
        )
        .unwrap();

    // first price
    assert_eq!(ticks.ticks[0].price, Decimal::from_ratio(600u128, 100u128));
    // Second price
    assert_eq!(
        ticks.ticks[1].price,
        Decimal::from_ratio(59403u128, 10000u128)
    );

    let order_6 = OrderResponse {
        order_id: 6u64,
        bidder_addr: "addr0001".to_string(),
        offer_asset: Asset {
            amount: Uint128::from(100u128),
            info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
        },
        ask_asset: Asset {
            amount: Uint128::from(600u128),
            info: AssetInfo::Token {
                contract_addr: usdt_token[0].clone(),
            },
        },
        filled_offer_amount: Uint128::zero(),
        filled_ask_amount: Uint128::zero(),
        direction: OrderDirection::Sell,
        status: OrderStatus::Open,
    };
    let orders = app
        .query::<OrdersResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Orders {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    AssetInfo::Token {
                        contract_addr: usdt_token[0].clone(),
                    },
                ],
                direction: None,
                filter: OrderFilter::Price(ticks.ticks[0].price),
                start_after: None,
                limit: None,
                order_by: Some(1),
            },
        )
        .unwrap();
    // assert order
    assert_eq!(order_6.clone(), orders.orders[1]);
}

#[test]
fn submit_market_order_native_token() {
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

    let msg = InstantiateMsg {
        name: None,
        version: None,
        admin: None,
        commission_rate: None,
        reward_address: None,
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
        spread: Some(Decimal::percent(10)),
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

    let mut base_amount = Uint128::from(1_000_000u128);
    let mut quote_amount = Uint128::from(5_000_000u128);
    let mut assets = [
        Asset {
            info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
            amount: base_amount,
        },
        Asset {
            info: AssetInfo::NativeToken {
                denom: USDT_DENOM.to_string(),
            },
            amount: quote_amount,
        },
    ];
    let asset_infos = assets.clone().map(|asset| asset.info);

    // CASE 1: submit first order on buy side => no check spread price, buy_price = 5
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Buy,
        assets: assets.clone(),
    };

    let _ = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: assets[1].info.to_string(),
                amount: quote_amount,
            }],
        )
        .unwrap();

    // query buy ticks - buy side has one tick = 5
    let ticks = app
        .query::<TicksResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Ticks {
                asset_infos: asset_infos.clone(),
                direction: OrderDirection::Buy,
                start_after: None,
                end: None,
                limit: None,
                order_by: Some(1),
            },
        )
        .unwrap();

    // assert price
    assert_eq!(
        ticks.ticks[0].price,
        Decimal::from_ratio(quote_amount, base_amount)
    );

    // query price with base_amount
    let base_amount_res = app
        .query::<BaseAmountResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::PriceByBaseAmount {
                asset_infos: asset_infos.clone(),
                base_amount,
                direction: OrderDirection::Buy,
                slippage: None,
            },
        )
        .unwrap();

    // Order book has 1 limit buy order -> BaseAmountResponse.market_price = 0 & BaseAmountResponse.expected_base_amount = 0
    assert_eq!(
        base_amount_res,
        BaseAmountResponse {
            market_price: Decimal::zero(),
            expected_base_amount: Uint128::zero()
        }
    );

    // query price with base_amount
    let base_amount_res = app
        .query::<BaseAmountResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::PriceByBaseAmount {
                asset_infos: asset_infos.clone(),
                base_amount,
                direction: OrderDirection::Sell,
                slippage: None,
            },
        )
        .unwrap();

    // Order book has 1 limit buy order -> BaseAmountResponse.market_price = 5 & BaseAmountResponse.expected_base_amount = 100
    assert_eq!(
        base_amount_res,
        BaseAmountResponse {
            market_price: Decimal::from_ratio(quote_amount, base_amount),
            expected_base_amount: base_amount
        }
    );

    // CASE 2: submit market sell order
    let msg = ExecuteMsg::SubmitMarketOrder {
        direction: OrderDirection::Sell,
        asset_infos: asset_infos.clone(),
        base_amount,
        quote_amount,
        slippage: None,
    };

    let res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: asset_infos[0].to_string(),
                amount: base_amount,
            }],
        )
        .unwrap();
    println!("matching result: {:?}", res);

    // CASE 3: failure case - slippage larger than 1
    let base_amount_3 = Uint128::from(2_000_000u128);
    let quote_amount_3 = Uint128::from(base_amount_3 * base_amount_res.market_price);
    let msg = ExecuteMsg::SubmitMarketOrder {
        direction: OrderDirection::Buy,
        asset_infos: asset_infos.clone(),
        base_amount: base_amount_3,
        quote_amount: quote_amount_3,
        slippage: Some(Decimal::one()),
    };

    let res = app.execute(
        Addr::unchecked("addr0000"),
        limit_order_addr.clone(),
        &msg,
        &[Coin {
            denom: asset_infos[1].to_string(),
            amount: quote_amount_3,
        }],
    );
    app.assert_fail(res);

    // CASE 4: buy market order with slippage = 0.005
    base_amount = Uint128::from(800_000u128);
    quote_amount = Uint128::from(4_000_000u128);
    assets[0].amount = base_amount;
    assets[1].amount = quote_amount;
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: assets.clone(),
    };

    let _ = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: assets[0].info.to_string(),
                amount: base_amount,
            }],
        )
        .unwrap();

    let base_amount_4 = Uint128::from(999_123u128);
    let slippage = Decimal::from_str("0.005").unwrap();
    let base_amount_res_4 = app
        .query::<BaseAmountResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::PriceByBaseAmount {
                asset_infos: asset_infos.clone(),
                base_amount: base_amount_4,
                direction: OrderDirection::Buy,
                slippage: Some(slippage),
            },
        )
        .unwrap();
    assert_eq!(
        base_amount_res_4,
        BaseAmountResponse {
            market_price: Decimal::from_ratio(quote_amount, base_amount)
                .checked_mul(Decimal::one() + slippage)
                .unwrap(),
            expected_base_amount: base_amount
        }
    );

    let quote_amount_4 = Uint128::from(base_amount_4 * base_amount_res_4.market_price);
    let msg = ExecuteMsg::SubmitMarketOrder {
        direction: OrderDirection::Buy,
        asset_infos: asset_infos.clone(),
        base_amount: base_amount_4,
        quote_amount: quote_amount_4,
        slippage: Some(slippage),
    };

    let res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: asset_infos[1].to_string(),
                amount: quote_amount_4.into(),
            }],
        )
        .unwrap();

    println!("matching result: {:?}", res);
}

#[test]
fn submit_market_order_cw20_token() {
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

    let usdt_token = app.set_token_balances(&[(
        &"asset".to_string(),
        &[
            (&"addr0000".to_string(), &Uint128::from(1000000000u128)),
            (&"addr0001".to_string(), &Uint128::from(1000000000u128)),
        ],
    )]);

    let msg = InstantiateMsg {
        name: None,
        version: None,
        admin: None,
        commission_rate: None,
        reward_address: None,
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

    // create order book for pair [orai, usdt_token]
    let msg = ExecuteMsg::CreateOrderBookPair {
        base_coin_info: AssetInfo::NativeToken {
            denom: ORAI_DENOM.to_string(),
        },
        quote_coin_info: AssetInfo::Token {
            contract_addr: usdt_token[0].clone(),
        },
        spread: Some(Decimal::percent(10)),
        min_quote_coin_amount: Uint128::zero(),
    };
    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[],
        )
        .unwrap();

    let mut base_amount = Uint128::from(1_000_000u128);
    let mut quote_amount = Uint128::from(5_000_000u128);
    let mut assets = [
        Asset {
            info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
            amount: base_amount,
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: usdt_token[0].clone(),
            },
            amount: quote_amount,
        },
    ];
    let asset_infos = assets.clone().map(|asset| asset.info);

    // CASE 1: submit first order on buy side => no check spread price, buy_price = 5
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: quote_amount,
        msg: to_binary(&Cw20HookMsg::SubmitOrder {
            direction: OrderDirection::Buy,
            assets: assets.clone(),
        })
        .unwrap(),
    };

    let _ = app
        .execute(
            Addr::unchecked("addr0000"),
            usdt_token[0].clone(),
            &msg,
            &[],
        )
        .unwrap();

    // query buy ticks - buy side has one tick = 5
    let ticks = app
        .query::<TicksResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::Ticks {
                asset_infos: asset_infos.clone(),
                direction: OrderDirection::Buy,
                start_after: None,
                end: None,
                limit: None,
                order_by: Some(1),
            },
        )
        .unwrap();

    // assert price
    assert_eq!(
        ticks.ticks[0].price,
        Decimal::from_ratio(quote_amount, base_amount)
    );

    // query price with base_amount
    let base_amount_res = app
        .query::<BaseAmountResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::PriceByBaseAmount {
                asset_infos: asset_infos.clone(),
                base_amount,
                direction: OrderDirection::Buy,
                slippage: None,
            },
        )
        .unwrap();

    // Order book has 1 limit buy order -> BaseAmountResponse.market_price = 0 & BaseAmountResponse.expected_base_amount = 0
    assert_eq!(
        base_amount_res,
        BaseAmountResponse {
            market_price: Decimal::zero(),
            expected_base_amount: Uint128::zero()
        }
    );

    // query price with base_amount
    let base_amount_res = app
        .query::<BaseAmountResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::PriceByBaseAmount {
                asset_infos: asset_infos.clone(),
                base_amount,
                direction: OrderDirection::Sell,
                slippage: None,
            },
        )
        .unwrap();

    // Order book has 1 limit buy order -> BaseAmountResponse.market_price = 5 & BaseAmountResponse.expected_base_amount = 100
    assert_eq!(
        base_amount_res,
        BaseAmountResponse {
            market_price: Decimal::from_ratio(quote_amount, base_amount),
            expected_base_amount: base_amount
        }
    );

    // CASE 2: submit market sell order
    let msg = ExecuteMsg::SubmitMarketOrder {
        direction: OrderDirection::Sell,
        asset_infos: asset_infos.clone(),
        base_amount,
        quote_amount,
        slippage: None,
    };

    let res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: asset_infos[0].to_string(),
                amount: base_amount,
            }],
        )
        .unwrap();
    println!("matching result: {:?}", res);

    // CASE 3: failure case - slippage larger than 1
    let base_amount_3 = Uint128::from(2_000_000u128);
    let quote_amount_3 = Uint128::from(base_amount_3 * base_amount_res.market_price);
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: quote_amount,
        msg: to_binary(&Cw20HookMsg::SubmitMarketOrder {
            direction: OrderDirection::Buy,
            asset_infos: asset_infos.clone(),
            base_amount: base_amount_3,
            quote_amount: quote_amount_3,
            slippage: Some(Decimal::one()),
        })
        .unwrap(),
    };

    let res = app.execute(
        Addr::unchecked("addr0000"),
        usdt_token[0].clone(),
        &msg,
        &[],
    );
    app.assert_fail(res);

    // CASE 4: failure case - no buy depth
    let base_amount_4 = Uint128::from(2_000_000u128);
    let quote_amount_4 = Uint128::from(base_amount_3 * base_amount_res.market_price);
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: quote_amount,
        msg: to_binary(&Cw20HookMsg::SubmitMarketOrder {
            direction: OrderDirection::Buy,
            asset_infos: asset_infos.clone(),
            base_amount: base_amount_4,
            quote_amount: quote_amount_4,
            slippage: None,
        })
        .unwrap(),
    };

    let res = app.execute(
        Addr::unchecked("addr0000"),
        usdt_token[0].clone(),
        &msg,
        &[],
    );
    app.assert_fail(res);

    // CASE 5: buy market order with slippage = 0.005
    base_amount = Uint128::from(800_000u128);
    quote_amount = Uint128::from(4_000_000u128);
    assets[0].amount = base_amount;
    assets[1].amount = quote_amount;
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: assets.clone(),
    };

    let _ = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &msg,
            &[Coin {
                denom: assets[0].info.to_string(),
                amount: base_amount,
            }],
        )
        .unwrap();

    let base_amount_5 = Uint128::from(999_123u128);
    let slippage = Decimal::from_str("0.005").unwrap();
    let base_amount_res_5 = app
        .query::<BaseAmountResponse, _>(
            limit_order_addr.clone(),
            &QueryMsg::PriceByBaseAmount {
                asset_infos: asset_infos.clone(),
                base_amount: base_amount_5,
                direction: OrderDirection::Buy,
                slippage: Some(slippage),
            },
        )
        .unwrap();
    assert_eq!(
        base_amount_res_5,
        BaseAmountResponse {
            market_price: Decimal::from_ratio(quote_amount, base_amount)
                .checked_mul(Decimal::one() + slippage)
                .unwrap(),
            expected_base_amount: base_amount
        }
    );

    let quote_amount_5 = Uint128::from(base_amount_5 * base_amount_res_5.market_price);
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: limit_order_addr.to_string(),
        amount: quote_amount_5,
        msg: to_binary(&Cw20HookMsg::SubmitMarketOrder {
            direction: OrderDirection::Buy,
            asset_infos: asset_infos.clone(),
            base_amount: base_amount_5,
            quote_amount: quote_amount_5,
            slippage: Some(slippage),
        })
        .unwrap(),
    };

    let res = app
        .execute(
            Addr::unchecked("addr0000"),
            usdt_token[0].clone(),
            &msg,
            &[],
        )
        .unwrap();

    println!("matching result: {:?}", res);
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
    let mut reward_balances = app
        .query_all_balances(Addr::unchecked(
            "orai16stq6f4pnrfpz75n9ujv6qg3czcfa4qyjux5en",
        ))
        .unwrap();
    let mut spread_balances = app
        .query_all_balances(Addr::unchecked(
            "orai139tjpfj0h6ld3wff7v2x92ntdewungfss0ml3n",
        ))
        .unwrap();

    println!("round 0 - address0's balances: {:?}", address0_balances);
    println!("round 0 - address1's balances: {:?}", address1_balances);
    println!("round 0 - address2's balances: {:?}", address2_balances);
    println!(
        "round 0 - reward_balances's balances: {:?}",
        reward_balances
    );
    println!(
        "round 0 - spread_balances's balances: {:?}\n\n",
        spread_balances
    );

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
    expected_balances = [].to_vec();
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
    reward_balances = app
        .query_all_balances(Addr::unchecked(
            "orai16stq6f4pnrfpz75n9ujv6qg3czcfa4qyjux5en",
        ))
        .unwrap();
    spread_balances = app
        .query_all_balances(Addr::unchecked(
            "orai139tjpfj0h6ld3wff7v2x92ntdewungfss0ml3n",
        ))
        .unwrap();

    println!("round 1 - address0's balances: {:?}", address0_balances);
    println!("round 1 - address1's balances: {:?}", address1_balances);
    println!("round 1 - address2's balances: {:?}", address2_balances);
    println!(
        "round 1 - reward_balances's balances: {:?}",
        reward_balances
    );
    println!(
        "round 1 - spread_balances's balances: {:?}\n\n",
        spread_balances
    );

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
        min_quote_coin_amount: Uint128::from(50u128),
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
fn simple_matching_test() {
    let mut app: MockApp = MockApp::new(&[
        (
            &"addr0000".to_string(),
            &[
                Coin {
                    denom: ORAI_DENOM.to_string(),
                    amount: Uint128::from(10000000000u128),
                },
                Coin {
                    denom: USDT_DENOM.to_string(),
                    amount: Uint128::from(10000000000u128),
                },
            ],
        ),
        (
            &"addr0001".to_string(),
            &[
                Coin {
                    denom: ORAI_DENOM.to_string(),
                    amount: Uint128::from(10000000000u128),
                },
                Coin {
                    denom: USDT_DENOM.to_string(),
                    amount: Uint128::from(10000000000u128),
                },
            ],
        ),
        (
            &"addr0002".to_string(),
            &[
                Coin {
                    denom: ORAI_DENOM.to_string(),
                    amount: Uint128::from(10000000000u128),
                },
                Coin {
                    denom: USDT_DENOM.to_string(),
                    amount: Uint128::from(10000000000u128),
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
        spread: Some(Decimal::percent(1)),
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
                amount: Uint128::from(10000000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(75123400u128),
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
                amount: Uint128::from(10000000u128),
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
                amount: Uint128::from(100000000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(752000000u128),
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
                amount: Uint128::from(752000000u128),
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
            amount: Uint128::from(9990000000u128),
        },
        Coin {
            denom: USDT_DENOM.to_string(),
            amount: Uint128::from(10000000000u128),
        },
    ]
    .to_vec();
    assert_eq!(address0_balances, expected_balances,);

    expected_balances = [
        Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(10000000000u128),
        },
        Coin {
            denom: USDT_DENOM.to_string(),
            amount: Uint128::from(9248000000u128),
        },
    ]
    .to_vec();
    assert_eq!(address2_balances, expected_balances);

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

    // Excecute all orders
    let execute_msg = ExecuteMsg::ExecuteOrderBookPair {
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
            &execute_msg,
            &[],
        )
        .unwrap();
    println!("[LOG] attribute - round 1 - {:?}", _res);

    /* <----------------------------------- order 3 -----------------------------------> */
    let msg = ExecuteMsg::SubmitOrder {
        direction: OrderDirection::Sell,
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(100000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: USDT_DENOM.to_string(),
                },
                amount: Uint128::from(751234u128),
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
                amount: Uint128::from(100000u128),
            }],
        )
        .unwrap();

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            limit_order_addr.clone(),
            &execute_msg,
            &[],
        )
        .unwrap();
    println!("[LOG] attribute - round 2 - {:?}", _res);

    address0_balances = app.query_all_balances(Addr::unchecked("addr0000")).unwrap();
    address1_balances = app.query_all_balances(Addr::unchecked("addr0001")).unwrap();
    address2_balances = app.query_all_balances(Addr::unchecked("addr0002")).unwrap();
    println!("round 1 - address0's balances: {:?}", address0_balances);
    println!("round 1 - address1's balances: {:?}", address1_balances);
    println!("round 1 - address2's balances: {:?}\n\n", address2_balances);

    expected_balances = [
        Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(9989900000u128),
        },
        Coin {
            denom: USDT_DENOM.to_string(),
            amount: Uint128::from(10075794254u128),
        },
    ]
    .to_vec();
    assert_eq!(address0_balances, expected_balances);
    expected_balances = [
        Coin {
            denom: ORAI_DENOM.to_string(),
            amount: Uint128::from(10010089300u128),
        },
        Coin {
            denom: USDT_DENOM.to_string(),
            amount: Uint128::from(9248000000u128),
        },
    ]
    .to_vec();
    assert_eq!(address2_balances, expected_balances);

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

    let expected_res = OrderBookMatchableResponse { is_matchable: true };
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
                amount: Uint128::from(21000u128),
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
                amount: Uint128::from(21000u128),
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
