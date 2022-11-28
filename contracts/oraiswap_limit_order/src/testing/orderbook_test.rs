use std::str::FromStr;

use cosmwasm_std::{testing::mock_dependencies, Api, Decimal, Order as OrderBy};
use oraiswap::{
    asset::{pair_key, AssetInfoRaw, ORAI_DENOM},
    limit_order::OrderDirection,
    testing::ATOM_DENOM,
};

use crate::{
    orderbook::{Order, OrderBook},
    state::{increase_last_order_id, init_last_order_id},
    tick::query_ticks,
};

#[test]
fn initialize() {
    let mut deps = mock_dependencies();

    let offer_info = AssetInfoRaw::NativeToken {
        denom: ORAI_DENOM.to_string(),
    };
    let ask_info = AssetInfoRaw::NativeToken {
        denom: ATOM_DENOM.to_string(),
    };
    let pair_key = pair_key(&[offer_info, ask_info]);
    let bidder_addr = deps.api.addr_canonicalize("addr0000").unwrap();

    init_last_order_id(deps.as_mut().storage).unwrap();

    let orders = vec![
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Buy,
            Decimal::from_str("10.01").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Buy,
            Decimal::from_str("10.00").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Buy,
            Decimal::from_str("9.999").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Sell,
            Decimal::from_str("9.999").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Buy,
            Decimal::from_str("9.998").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Sell,
            Decimal::from_str("9.998").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Sell,
            Decimal::from_str("9.997").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Sell,
            Decimal::from_str("9.996").unwrap(),
            10000u128.into(),
        ),
    ];

    let mut ob = OrderBook::new(&pair_key, None);

    for order in orders.iter() {
        let total_orders = ob.add_order(deps.as_mut().storage, order).unwrap();
        println!(
            "insert order id: {}, direction: {:?}, price: {}, total orders: {}",
            order.order_id,
            order.direction,
            order.get_price(),
            total_orders
        );
    }

    let buy_ticks = query_ticks(
        deps.as_ref().storage,
        &pair_key,
        OrderDirection::Buy,
        None,
        None,
        Some(1),
    )
    .unwrap();
    println!("buy ticks: {:?}", buy_ticks);

    let sell_ticks = query_ticks(
        deps.as_ref().storage,
        &pair_key,
        OrderDirection::Sell,
        None,
        None,
        None,
    )
    .unwrap();
    println!("sell ticks: {:?}", sell_ticks);

    let (highest, found, _) = ob.highest_price(deps.as_ref().storage, OrderDirection::Buy);
    assert!(found);
    assert_eq!(highest, Decimal::from_str("10.01").unwrap());

    let (lowest, found, _) = ob.lowest_price(deps.as_ref().storage, OrderDirection::Sell);
    assert!(found);
    assert_eq!(lowest, Decimal::from_str("9.996").unwrap());
}

#[test]
fn buy_orders_at() {
    let mut deps = mock_dependencies();

    let offer_info = AssetInfoRaw::NativeToken {
        denom: ORAI_DENOM.to_string(),
    };
    let ask_info = AssetInfoRaw::NativeToken {
        denom: ATOM_DENOM.to_string(),
    };
    let pair_key = pair_key(&[offer_info, ask_info]);
    let bidder_addr = deps.api.addr_canonicalize("addr0000").unwrap();
    init_last_order_id(deps.as_mut().storage).unwrap();

    let orders = vec![
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Sell,
            Decimal::from_str("1.1").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Buy,
            Decimal::from_str("1.1").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Buy,
            Decimal::from_str("1.1").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Buy,
            Decimal::from_str("1.0").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Buy,
            Decimal::from_str("1.2").unwrap(),
            10000u128.into(),
        ),
    ];

    let mut ob = OrderBook::new(&pair_key, None);

    for order in orders.iter() {
        let total_orders = ob.add_order(deps.as_mut().storage, order).unwrap();
        println!(
            "insert order id: {}, direction: {:?}, price: {}, total orders: {}",
            order.order_id,
            order.direction,
            order.get_price(),
            total_orders
        );
    }

    let buy_orders = ob
        .orders_at(
            deps.as_ref().storage,
            Decimal::from_str("1.1").unwrap(),
            OrderDirection::Buy,
            None,
            None,
            Some(OrderBy::Ascending), // remain order to compare
        )
        .unwrap();
    assert_eq!(buy_orders.len(), 2);
    assert_eq!(buy_orders, orders[1..=2]);

    assert!(ob
        .orders_at(
            deps.as_ref().storage,
            Decimal::from_str("0.9").unwrap(),
            OrderDirection::Buy,
            None,
            None,
            None
        )
        .unwrap()
        .is_empty());
}

#[test]
fn sell_orders_at() {
    let mut deps = mock_dependencies();

    let offer_info = AssetInfoRaw::NativeToken {
        denom: ORAI_DENOM.to_string(),
    };
    let ask_info = AssetInfoRaw::NativeToken {
        denom: ATOM_DENOM.to_string(),
    };
    let pair_key = pair_key(&[offer_info, ask_info]);
    let bidder_addr = deps.api.addr_canonicalize("addr0000").unwrap();
    init_last_order_id(deps.as_mut().storage).unwrap();

    let orders = vec![
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Buy,
            Decimal::from_str("1.1").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Sell,
            Decimal::from_str("1.1").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Sell,
            Decimal::from_str("1.1").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Sell,
            Decimal::from_str("1.0").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Sell,
            Decimal::from_str("1.2").unwrap(),
            10000u128.into(),
        ),
    ];

    let mut ob = OrderBook::new(&pair_key, None);
    for order in orders.iter() {
        let total_orders = ob.add_order(deps.as_mut().storage, order).unwrap();
        println!(
            "insert order id: {}, direction: {:?}, price: {}, total orders: {}",
            order.order_id,
            order.direction,
            order.get_price(),
            total_orders
        );
    }
    let sell_orders = ob
        .orders_at(
            deps.as_ref().storage,
            Decimal::from_str("1.1").unwrap(),
            OrderDirection::Sell,
            None,
            None,
            Some(OrderBy::Ascending),
        )
        .unwrap();
    assert_eq!(sell_orders.len(), 2);
    assert_eq!(sell_orders, orders[1..=2]);
    assert!(ob
        .orders_at(
            deps.as_ref().storage,
            Decimal::from_str("0.9").unwrap(),
            OrderDirection::Sell,
            None,
            None,
            None
        )
        .unwrap()
        .is_empty());
}

#[test]
fn highest_lowest_price() {
    let mut deps = mock_dependencies();

    let offer_info = AssetInfoRaw::NativeToken {
        denom: ORAI_DENOM.to_string(),
    };
    let ask_info = AssetInfoRaw::NativeToken {
        denom: ATOM_DENOM.to_string(),
    };
    let pair_key = pair_key(&[offer_info, ask_info]);
    let bidder_addr = deps.api.addr_canonicalize("addr0000").unwrap();
    init_last_order_id(deps.as_mut().storage).unwrap();

    struct HighestLowestPrice {
        ob: OrderBook,
        orders: Vec<Order>,
        highest_price: Decimal,
        lowest_price: Decimal,
    }

    let mut test_cases: Vec<HighestLowestPrice> = vec![
        HighestLowestPrice {
            ob: OrderBook::new(&pair_key, None),
            orders: vec![],
            highest_price: Decimal::MAX,
            lowest_price: Decimal::MIN,
        },
        HighestLowestPrice {
            ob: OrderBook::new(&pair_key, None),
            orders: vec![
                Order::new(
                    increase_last_order_id(deps.as_mut().storage).unwrap(),
                    bidder_addr.clone(),
                    OrderDirection::Buy,
                    Decimal::from_str("1.1").unwrap(),
                    10000u128.into(),
                ),
                Order::new(
                    increase_last_order_id(deps.as_mut().storage).unwrap(),
                    bidder_addr.clone(),
                    OrderDirection::Buy,
                    Decimal::from_str("1.0").unwrap(),
                    10000u128.into(),
                ),
            ],
            highest_price: Decimal::from_str("1.1").unwrap(),
            lowest_price: Decimal::from_str("1.0").unwrap(),
        },
        HighestLowestPrice {
            ob: OrderBook::new(&pair_key, None),
            orders: vec![
                Order::new(
                    increase_last_order_id(deps.as_mut().storage).unwrap(),
                    bidder_addr.clone(),
                    OrderDirection::Sell,
                    Decimal::from_str("1.1").unwrap(),
                    10000u128.into(),
                ),
                Order::new(
                    increase_last_order_id(deps.as_mut().storage).unwrap(),
                    bidder_addr.clone(),
                    OrderDirection::Sell,
                    Decimal::from_str("1.0").unwrap(),
                    10000u128.into(),
                ),
            ],
            highest_price: Decimal::from_str("1.1").unwrap(),
            lowest_price: Decimal::from_str("1.0").unwrap(),
        },
        HighestLowestPrice {
            ob: OrderBook::new(&pair_key, None),
            orders: vec![
                Order::new(
                    increase_last_order_id(deps.as_mut().storage).unwrap(),
                    bidder_addr.clone(),
                    OrderDirection::Sell,
                    Decimal::from_str("1.1").unwrap(),
                    10000u128.into(),
                ),
                Order::new(
                    increase_last_order_id(deps.as_mut().storage).unwrap(),
                    bidder_addr.clone(),
                    OrderDirection::Sell,
                    Decimal::from_str("1.0").unwrap(),
                    10000u128.into(),
                ),
                Order::new(
                    increase_last_order_id(deps.as_mut().storage).unwrap(),
                    bidder_addr.clone(),
                    OrderDirection::Buy,
                    Decimal::from_str("1.0").unwrap(),
                    10000u128.into(),
                ),
                Order::new(
                    increase_last_order_id(deps.as_mut().storage).unwrap(),
                    bidder_addr.clone(),
                    OrderDirection::Buy,
                    Decimal::from_str("0.9").unwrap(),
                    10000u128.into(),
                ),
            ],
            highest_price: Decimal::from_str("1.1").unwrap(),
            lowest_price: Decimal::from_str("0.9").unwrap(),
        },
    ];

    for tc in test_cases.iter_mut() {
        for order in tc.orders.iter() {
            let total_orders = tc.ob.add_order(deps.as_mut().storage, order).unwrap();
            println!(
                "insert order id: {}, direction: {:?}, price: {}, total orders: {}",
                order.order_id,
                order.direction,
                order.get_price(),
                total_orders
            );
        }
        let (highest_buy, found_buy, _) = tc
            .ob
            .highest_price(deps.as_ref().storage, OrderDirection::Buy);
        let (highest_sell, found_sell, _) = tc
            .ob
            .highest_price(deps.as_ref().storage, OrderDirection::Sell);

        if found_buy || found_sell {
            let highest_price = Decimal::max(highest_buy, highest_sell);
            assert_eq!(tc.highest_price, highest_price);
        }

        let (lowest_buy, found_buy, _) = tc
            .ob
            .lowest_price(deps.as_ref().storage, OrderDirection::Buy);
        let (lowest_sell, found_sell, _) = tc
            .ob
            .lowest_price(deps.as_ref().storage, OrderDirection::Sell);

        if found_buy || found_sell {
            let lowest_price = Decimal::min(lowest_buy, lowest_sell);
            assert_eq!(tc.lowest_price, lowest_price);
        }
    }
}

#[test]
fn matchable_orders() {
    // buy orai with usdt and sell orai to usdt, orai and usdt(ibc) are both native tokens
    // we show highest buy and lowest sell first
    let mut deps = mock_dependencies();

    let offer_info = AssetInfoRaw::NativeToken {
        denom: "usdt".to_string(),
    };
    let ask_info = AssetInfoRaw::NativeToken {
        denom: ORAI_DENOM.to_string(),
    };
    let pair_key = pair_key(&[offer_info.clone(), ask_info.clone()]);
    let bidder_addr = deps.api.addr_canonicalize("addr0000").unwrap();
    init_last_order_id(deps.as_mut().storage).unwrap();

    // buy is offering orai, asking usdt * orai_price, sell is asking for usdt
    let orders = vec![
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Buy,
            Decimal::from_str("1.098").unwrap(), // buy (want 10000 orai, paid 10980 usdt)
            10000u128.into(),
        ),
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Buy,
            Decimal::from_str("1.097").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Buy,
            Decimal::from_str("1.099").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Sell,
            Decimal::from_str("1.099").unwrap(), // sell (paid 10000 orai, want 10990 usdt)
            10000u128.into(),
        ),
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Sell,
            Decimal::from_str("1.108").unwrap(),
            10000u128.into(),
        ),
    ];

    let mut ob = OrderBook::new(&pair_key, None);
    for order in orders.iter() {
        let _total_orders = ob.add_order(deps.as_mut().storage, order).unwrap();
        // if sell then paid asset must be ask asset
        let paid_denom = match order.direction {
            OrderDirection::Buy => "usdt",
            OrderDirection::Sell => ORAI_DENOM,
        };

        println!(
            "insert order id: {}, paid {}{} for {:?} {} at {}",
            order.order_id,
            order.ask_amount,
            paid_denom,
            order.direction,
            ORAI_DENOM,
            order.get_price(),
        );
    }

    let (best_buy_price, best_sell_price) = ob.find_match_price(deps.as_ref().storage).unwrap();
    // both are 1.099
    assert_eq!(best_buy_price, best_sell_price);
}
