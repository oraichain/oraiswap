use std::str::FromStr;

use cosmwasm_std::{testing::mock_dependencies, Api, Decimal};
use oraiswap::{
    asset::{AssetInfoRaw, ORAI_DENOM},
    limit_order::OrderDirection,
    testing::ATOM_DENOM,
};

use crate::orderbook::{Order, OrderBook};

#[test]
fn initialize() {
    let deps = mock_dependencies();

    let buy_info = AssetInfoRaw::NativeToken {
        denom: ORAI_DENOM.to_string(),
    };
    let sell_info = AssetInfoRaw::NativeToken {
        denom: ATOM_DENOM.to_string(),
    };
    let bidder_addr = deps.api.addr_canonicalize("addr0000").unwrap();

    let orders = vec![
        Order::new(
            1u64,
            bidder_addr.clone(),
            OrderDirection::Buy,
            Decimal::from_str("10.01").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            2u64,
            bidder_addr.clone(),
            OrderDirection::Buy,
            Decimal::from_str("10.00").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            3u64,
            bidder_addr.clone(),
            OrderDirection::Buy,
            Decimal::from_str("9.999").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            4u64,
            bidder_addr.clone(),
            OrderDirection::Sell,
            Decimal::from_str("9.999").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            5u64,
            bidder_addr.clone(),
            OrderDirection::Buy,
            Decimal::from_str("9.998").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            6u64,
            bidder_addr.clone(),
            OrderDirection::Sell,
            Decimal::from_str("9.998").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            7u64,
            bidder_addr.clone(),
            OrderDirection::Sell,
            Decimal::from_str("9.997").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            8u64,
            bidder_addr.clone(),
            OrderDirection::Sell,
            Decimal::from_str("9.996").unwrap(),
            10000u128.into(),
        ),
    ];

    let ob = OrderBook::new(buy_info, sell_info, orders);

    let (highest, found) = ob.highest_price();
    assert!(found);
    assert_eq!(highest, Decimal::from_str("10.01").unwrap());

    let (lowest, found) = ob.lowest_price();
    assert!(found);
    assert_eq!(lowest, Decimal::from_str("9.996").unwrap());
}

#[test]
fn buy_orders_at() {
    let deps = mock_dependencies();

    let buy_info = AssetInfoRaw::NativeToken {
        denom: ORAI_DENOM.to_string(),
    };
    let sell_info = AssetInfoRaw::NativeToken {
        denom: ATOM_DENOM.to_string(),
    };
    let bidder_addr = deps.api.addr_canonicalize("addr0000").unwrap();
    let orders = vec![
        Order::new(
            1u64,
            bidder_addr.clone(),
            OrderDirection::Sell,
            Decimal::from_str("1.1").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            2u64,
            bidder_addr.clone(),
            OrderDirection::Buy,
            Decimal::from_str("1.1").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            3u64,
            bidder_addr.clone(),
            OrderDirection::Buy,
            Decimal::from_str("1.1").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            4u64,
            bidder_addr.clone(),
            OrderDirection::Buy,
            Decimal::from_str("1.0").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            5u64,
            bidder_addr.clone(),
            OrderDirection::Buy,
            Decimal::from_str("1.2").unwrap(),
            10000u128.into(),
        ),
    ];

    let ob = OrderBook::new(buy_info, sell_info, orders.clone());

    let buy_orders = ob.buy_orders_at(Decimal::from_str("1.1").unwrap());
    assert_eq!(buy_orders.len(), 2);
    assert_eq!(buy_orders, orders[1..=2]);

    assert!(ob
        .buy_orders_at(Decimal::from_str("0.9").unwrap())
        .is_empty());
}

#[test]
fn sell_orders_at() {
    let deps = mock_dependencies();

    let buy_info = AssetInfoRaw::NativeToken {
        denom: ORAI_DENOM.to_string(),
    };
    let sell_info = AssetInfoRaw::NativeToken {
        denom: ATOM_DENOM.to_string(),
    };
    let bidder_addr = deps.api.addr_canonicalize("addr0000").unwrap();
    let orders = vec![
        Order::new(
            1u64,
            bidder_addr.clone(),
            OrderDirection::Buy,
            Decimal::from_str("1.1").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            2u64,
            bidder_addr.clone(),
            OrderDirection::Sell,
            Decimal::from_str("1.1").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            3u64,
            bidder_addr.clone(),
            OrderDirection::Sell,
            Decimal::from_str("1.1").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            4u64,
            bidder_addr.clone(),
            OrderDirection::Sell,
            Decimal::from_str("1.0").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            5u64,
            bidder_addr.clone(),
            OrderDirection::Sell,
            Decimal::from_str("1.2").unwrap(),
            10000u128.into(),
        ),
    ];

    let ob = OrderBook::new(buy_info, sell_info, orders.clone());
    let sell_orders = ob.sell_orders_at(Decimal::from_str("1.1").unwrap());
    assert_eq!(sell_orders.len(), 2);
    assert_eq!(sell_orders, orders[1..=2]);
    assert!(ob
        .sell_orders_at(Decimal::from_str("0.9").unwrap())
        .is_empty());
}

#[test]
fn highest_price_lowest_price() {
    let deps = mock_dependencies();

    let buy_info = AssetInfoRaw::NativeToken {
        denom: ORAI_DENOM.to_string(),
    };
    let sell_info = AssetInfoRaw::NativeToken {
        denom: ATOM_DENOM.to_string(),
    };
    let bidder_addr = deps.api.addr_canonicalize("addr0000").unwrap();
    struct HighestLowestPrice {
        ob: OrderBook,
        found: bool,
        highest_price: Decimal,
        lowest_price: Decimal,
    }

    let test_cases: Vec<HighestLowestPrice> = vec![
        HighestLowestPrice {
            ob: OrderBook::new(buy_info.clone(), sell_info.clone(), vec![]),
            found: false,
            highest_price: Decimal::zero(),
            lowest_price: Decimal::zero(),
        },
        HighestLowestPrice {
            ob: OrderBook::new(
                buy_info.clone(),
                sell_info.clone(),
                vec![
                    Order::new(
                        1u64,
                        bidder_addr.clone(),
                        OrderDirection::Buy,
                        Decimal::from_str("1.1").unwrap(),
                        10000u128.into(),
                    ),
                    Order::new(
                        2u64,
                        bidder_addr.clone(),
                        OrderDirection::Buy,
                        Decimal::from_str("1.0").unwrap(),
                        10000u128.into(),
                    ),
                ],
            ),
            found: true,
            highest_price: Decimal::from_str("1.1").unwrap(),
            lowest_price: Decimal::from_str("1.0").unwrap(),
        },
        HighestLowestPrice {
            ob: OrderBook::new(
                buy_info.clone(),
                sell_info.clone(),
                vec![
                    Order::new(
                        3u64,
                        bidder_addr.clone(),
                        OrderDirection::Sell,
                        Decimal::from_str("1.1").unwrap(),
                        10000u128.into(),
                    ),
                    Order::new(
                        4u64,
                        bidder_addr.clone(),
                        OrderDirection::Sell,
                        Decimal::from_str("1.0").unwrap(),
                        10000u128.into(),
                    ),
                ],
            ),
            found: true,
            highest_price: Decimal::from_str("1.1").unwrap(),
            lowest_price: Decimal::from_str("1.0").unwrap(),
        },
        HighestLowestPrice {
            ob: OrderBook::new(
                buy_info.clone(),
                sell_info.clone(),
                vec![
                    Order::new(
                        5u64,
                        bidder_addr.clone(),
                        OrderDirection::Sell,
                        Decimal::from_str("1.1").unwrap(),
                        10000u128.into(),
                    ),
                    Order::new(
                        6u64,
                        bidder_addr.clone(),
                        OrderDirection::Sell,
                        Decimal::from_str("1.0").unwrap(),
                        10000u128.into(),
                    ),
                    Order::new(
                        7u64,
                        bidder_addr.clone(),
                        OrderDirection::Buy,
                        Decimal::from_str("1.0").unwrap(),
                        10000u128.into(),
                    ),
                    Order::new(
                        8u64,
                        bidder_addr.clone(),
                        OrderDirection::Buy,
                        Decimal::from_str("0.9").unwrap(),
                        10000u128.into(),
                    ),
                ],
            ),
            found: true,
            highest_price: Decimal::from_str("1.1").unwrap(),
            lowest_price: Decimal::from_str("0.9").unwrap(),
        },
    ];

    for tc in test_cases {
        let (highest, found) = tc.ob.highest_price();
        assert_eq!(tc.found, found);
        let (lowest, found) = tc.ob.lowest_price();
        assert_eq!(tc.found, found);
        if tc.found {
            assert_eq!(tc.highest_price, highest);
            assert_eq!(tc.lowest_price, lowest);
        }
    }
}
