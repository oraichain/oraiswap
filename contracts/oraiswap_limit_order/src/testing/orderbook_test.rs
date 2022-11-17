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
}
