use std::str::FromStr;

use cosmwasm_std::{testing::mock_dependencies, Api, Decimal};
use oraiswap::{
    asset::{AssetInfoRaw, ORAI_DENOM},
    limit_order::OrderDirection,
    testing::ATOM_DENOM,
};

use crate::{
    jsonstr,
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

    let mut ob = OrderBook::new(ask_info, offer_info, None);

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
    let pair_key = &ob.get_pair_key();

    let buy_ticks = query_ticks(
        deps.as_ref().storage,
        pair_key,
        OrderDirection::Buy,
        None,
        None,
        Some(1),
    )
    .unwrap();
    println!("buy ticks: {:?}", buy_ticks);

    let sell_ticks = query_ticks(
        deps.as_ref().storage,
        pair_key,
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

    let mut ob = OrderBook::new(ask_info, offer_info, None);

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

    let mut ob = OrderBook::new(ask_info, offer_info, None);
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
            ob: OrderBook::new(ask_info.clone(), offer_info.clone(), None),
            orders: vec![],
            highest_price: Decimal::MAX,
            lowest_price: Decimal::MIN,
        },
        HighestLowestPrice {
            ob: OrderBook::new(ask_info.clone(), offer_info.clone(), None),
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
            ob: OrderBook::new(ask_info.clone(), offer_info.clone(), None),
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
            ob: OrderBook::new(ask_info.clone(), offer_info.clone(), None),
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

    let bidder_addr = deps.api.addr_canonicalize("addr0000").unwrap();
    init_last_order_id(deps.as_mut().storage).unwrap();

    // buy : wanting ask amount, sell: paying ask amount
    let orders = vec![
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Buy,
            Decimal::from_str("1.098").unwrap(), // buy (want 10000 orai, paid 10980 usdt)
            10000u128.into(),                    // buy then amount is offer
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
            15000u128.into(),
        ),
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Buy,
            Decimal::from_str("1.099").unwrap(),
            15000u128.into(),
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
            Decimal::from_str("1.099").unwrap(), // sell (paid 10000 orai, want 10990 usdt)
            5000u128.into(),
        ),
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Sell,
            Decimal::from_str("1.108").unwrap(),
            10000u128.into(),
        ),
    ];

    let mut ob = OrderBook::new(ask_info, offer_info, Some(Decimal::percent(1)));
    for order in orders.iter() {
        let _total_orders = ob.add_order(deps.as_mut().storage, order).unwrap();
        // if sell then paid asset must be ask asset
        let action = match order.direction {
            OrderDirection::Buy => "paid",
            OrderDirection::Sell => "want",
        };

        println!(
            "insert order id: {}, {} {}{} for {:?} {}{} at {}",
            order.order_id,
            action,
            order.offer_amount,
            "usdt", // this is offer denom
            order.direction,
            order.ask_amount,
            ORAI_DENOM,
            order.get_price(),
        );
    }

    let (best_buy_price, best_sell_price) = ob.find_match_price(deps.as_ref().storage).unwrap();
    // both are 1.099
    assert_eq!(best_buy_price, best_sell_price);

    // now add a lower sell price but too low for precision
    let order = Order::new(
        increase_last_order_id(deps.as_mut().storage).unwrap(),
        bidder_addr.clone(),
        OrderDirection::Sell,
        Decimal::from_str("1.008").unwrap(),
        10000u128.into(),
    );
    ob.add_order(deps.as_mut().storage, &order).unwrap();
    let (best_buy_price, best_sell_price) = ob.find_match_price(deps.as_ref().storage).unwrap();
    // both are still 1.099, so user keeps the profit maximum
    assert_eq!(best_buy_price, best_sell_price);

    let mut match_buy_orders =
        ob.find_match_orders(deps.as_ref().storage, best_buy_price, OrderDirection::Buy);
    println!("match buy orders : {}", jsonstr!(match_buy_orders));
    assert_eq!(match_buy_orders, orders[2..=3]);

    // find sell order as ask order, and buy orders as offer
    let mut offer_orders = ob
        .orders_at(
            deps.as_ref().storage,
            best_sell_price,
            OrderDirection::Sell,
            None,
            None,
        )
        .unwrap();

    println!("offer order {}", jsonstr!(offer_orders));

    let ask_order = &mut match_buy_orders[0];

    let messages = ob
        .distribute_order_to_orders(deps.as_mut(), ask_order, &mut offer_orders)
        .unwrap();

    // ask order is fullfilled, as well as offer orders
    assert_eq!(ask_order.ask_amount, ask_order.filled_ask_amount);
    for offer_order in offer_orders.iter() {
        assert_eq!(offer_order.ask_amount, offer_order.filled_ask_amount);
    }

    println!("messages {:?}", messages);
    println!(
        "ask order {}\noffer order {}",
        jsonstr!(ask_order),
        jsonstr!(offer_orders)
    );
}
