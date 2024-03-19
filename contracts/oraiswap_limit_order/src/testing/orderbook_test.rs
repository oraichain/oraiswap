use std::str::FromStr;

use cosmwasm_std::{testing::mock_dependencies, Api, Decimal, Order as OrderBy, Uint128};
use oraiswap::{
    asset::{AssetInfoRaw, ORAI_DENOM},
    limit_order::{OrderDirection, OrderStatus},
    math::DecimalPlaces,
    testing::ATOM_DENOM,
};

use crate::{
    order::{matching_order, MIN_VOLUME},
    orderbook::{Order, OrderBook},
    query::query_ticks_prices,
    state::{increase_last_order_id, init_last_order_id},
};

#[test]
fn test_limit_decimal_places() {
    let value = Decimal::from_ratio(6655325443433u128, 1000000000000u128);
    assert_eq!(
        value.limit_decimal_places(Some(2)).unwrap(),
        Decimal::from_str("6.65").unwrap()
    );
    assert_eq!(
        value.limit_decimal_places(Some(10)).unwrap(),
        Decimal::from_str("6.655325").unwrap()
    );
    assert_eq!(
        value.limit_decimal_places(None).unwrap(),
        Decimal::from_str("6.655325").unwrap()
    );

    assert_eq!(
        value.limit_decimal_places(Some(0)).unwrap(),
        Decimal::from_str("6").unwrap()
    )
}

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

    let buy_ticks = query_ticks_prices(
        deps.as_ref().storage,
        pair_key,
        OrderDirection::Buy,
        None,
        None,
        Some(OrderBy::Ascending),
    );
    println!("buy ticks: {:?}", buy_ticks);

    let sell_ticks = query_ticks_prices(
        deps.as_ref().storage,
        pair_key,
        OrderDirection::Sell,
        None,
        None,
        None,
    );
    println!("sell ticks: {:?}", sell_ticks);

    if let (Some((highest, _)), Some((_, _))) = (
        ob.highest_price(deps.as_ref().storage, OrderDirection::Buy),
        ob.lowest_price(deps.as_ref().storage, OrderDirection::Sell),
    ) {
        assert_eq!(highest, Decimal::from_str("10.01").unwrap());
    }
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

    println!("sell_orders: {:?}", sell_orders);
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
        if let (Some((highest_buy, _)), Some((highest_sell, _))) = (
            tc.ob
                .highest_price(deps.as_ref().storage, OrderDirection::Buy),
            tc.ob
                .highest_price(deps.as_ref().storage, OrderDirection::Sell),
        ) {
            let highest_price = Decimal::max(highest_buy, highest_sell);
            println!(
                "tc.highest_price: {} - highest_price: {}",
                tc.highest_price, highest_price
            );
        }

        if let (Some((lowest_buy, _)), Some((lowest_sell, _))) = (
            tc.ob
                .lowest_price(deps.as_ref().storage, OrderDirection::Buy),
            tc.ob
                .lowest_price(deps.as_ref().storage, OrderDirection::Sell),
        ) {
            let lowest_price = Decimal::min(lowest_buy, lowest_sell);
            println!(
                "tc.lowest_price: {} - lowest_price: {}",
                tc.lowest_price, lowest_price
            );
        }
    }
}

#[test]
fn test_matching_order_process() {
    let mut deps = mock_dependencies();

    let offer_info = AssetInfoRaw::NativeToken {
        denom: ATOM_DENOM.to_string(),
    };
    let ask_info = AssetInfoRaw::NativeToken {
        denom: ORAI_DENOM.to_string(),
    };

    let bidder_addr = deps.api.addr_canonicalize("addr0000").unwrap();
    init_last_order_id(deps.as_mut().storage).unwrap();

    // scenario: 4 sell orders 10000 orai at price 1, 1.1. 1.2, 1.3
    let orders = vec![
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Sell,
            Decimal::from_str("1").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Sell,
            Decimal::from_str("1.1").unwrap(),
            11000u128.into(),
        ),
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Sell,
            Decimal::from_str("1.2").unwrap(),
            12000u128.into(),
        ),
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Sell,
            Decimal::from_str("1.3").unwrap(),
            13000u128.into(),
        ),
    ];

    let mut ob = OrderBook::new(ask_info.clone(), offer_info.clone(), None);

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

    // case 1: order_price < all sell price, dont match
    let buy_price = Decimal::from_str("0.9").unwrap();
    let buy_order = Order::new(
        increase_last_order_id(deps.as_mut().storage).unwrap(),
        bidder_addr.clone(),
        OrderDirection::Buy,
        buy_price,
        25000u128.into(),
    );

    let (buy_order_with_fee, matched_order) =
        matching_order(deps.as_ref(), ob.clone(), &buy_order, buy_price).unwrap();

    assert_eq!(buy_order_with_fee.filled_ask_amount, Uint128::zero());
    assert_eq!(buy_order_with_fee.filled_offer_this_round, Uint128::zero());
    assert_eq!(matched_order.len(), 0);

    // case 2: submit buy order at price 1.15, ask 25000 Orai
    // - matched 2 sell order at price 1 and 1.1
    // => buy_order: matched 20000 orai, and 21000 usdt
    // order at price 1 and 1.1 if fulfilled

    let buy_price = Decimal::from_str("1.15").unwrap();
    let buy_order = Order::new(
        increase_last_order_id(deps.as_mut().storage).unwrap(),
        bidder_addr.clone(),
        OrderDirection::Buy,
        buy_price,
        25000u128.into(),
    );

    let (buy_order_with_fee, matched_order) =
        matching_order(deps.as_ref(), ob.clone(), &buy_order, buy_price).unwrap();
    // Because the number is rounded, we will check it differently from the expected amount
    assert!(
        buy_order_with_fee
            .filled_ask_amount
            .abs_diff(Uint128::from(20000u128))
            < Uint128::from(MIN_VOLUME)
    );
    assert!(
        buy_order_with_fee
            .filled_offer_amount
            .abs_diff(Uint128::from(21000u128))
            < Uint128::from(MIN_VOLUME)
    );
    assert_eq!(matched_order.len(), 2);
    for order in matched_order {
        assert!(order.filled_ask_amount.abs_diff(order.ask_amount) < Uint128::from(MIN_VOLUME));
        assert!(order.filled_offer_amount.abs_diff(order.offer_amount) < Uint128::from(MIN_VOLUME));
    }

    // case 3: match all
    // - order_buy: 25000 orai
    // - match:
    //      + sell: order at 1,1.1 fulfilled,
    //      + sell: order at price 1.2 partial filled (matched 5000 orai, 5000 * 1.2  = 7000 usdt)
    //      + buy: oder_buy match 25000 orai, with offer 10000 * 1 + 10000 * 1.1 + 5000 * 1.2 = 27000 usdt

    let buy_price = Decimal::from_str("1.5").unwrap();
    let buy_order = Order::new(
        increase_last_order_id(deps.as_mut().storage).unwrap(),
        bidder_addr.clone(),
        OrderDirection::Buy,
        buy_price,
        25000u128.into(),
    );

    let (buy_order_with_fee, matched_order) =
        matching_order(deps.as_ref(), ob.clone(), &buy_order, buy_price).unwrap();
    // Because the number is rounded, we will check it differently from the expected amount
    assert!(
        buy_order_with_fee
            .filled_ask_amount
            .abs_diff(buy_order.ask_amount)
            < Uint128::from(MIN_VOLUME)
    );
    assert!(
        buy_order_with_fee
            .filled_offer_amount
            .abs_diff(Uint128::from(27000u128))
            < Uint128::from(MIN_VOLUME)
    );

    assert_eq!(matched_order.len(), 3);
    for i in 0..2 {
        assert!(
            matched_order[i]
                .filled_ask_amount
                .abs_diff(matched_order[i].ask_amount)
                < Uint128::from(MIN_VOLUME)
        );
        assert!(
            matched_order[i]
                .filled_offer_amount
                .abs_diff(matched_order[i].offer_amount)
                < Uint128::from(MIN_VOLUME)
        );
    }
    assert!(
        matched_order[2]
            .filled_ask_amount
            .abs_diff(Uint128::from(6000u128))
            < Uint128::from(MIN_VOLUME)
    );

    // case 4: test with match sell order
    //
    // scenario: 4 buy orders 10000 orai at price 1, 1.1. 1.2, 1.3, and create a market sell order 50000 orai with min price = 1
    // sell_order matched: offer 50000,filled offer 40000, filled ask (1 + 1.1 + 1.2 + 1.3) * 1000 = 46000
    // all buy order will fulfilled

    let mut deps = mock_dependencies();
    init_last_order_id(deps.as_mut().storage).unwrap();

    let orders = vec![
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Buy,
            Decimal::from_str("1").unwrap(),
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
            Decimal::from_str("1.2").unwrap(),
            10000u128.into(),
        ),
        Order::new(
            increase_last_order_id(deps.as_mut().storage).unwrap(),
            bidder_addr.clone(),
            OrderDirection::Buy,
            Decimal::from_str("1.3").unwrap(),
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

    // create market sell order 50000 orai wih min price is 1
    let sell_min_price = Decimal::from_str("1").unwrap();
    let sell_price = Decimal::from_str("1.5").unwrap();
    let mut sell_order = Order::new(
        increase_last_order_id(deps.as_mut().storage).unwrap(),
        bidder_addr.clone(),
        OrderDirection::Sell,
        sell_price,
        50000u128.into(),
    );
    sell_order.offer_amount = Uint128::from(50000u128);

    let (sell_order_with_fee, matched_order) =
        matching_order(deps.as_ref(), ob.clone(), &sell_order, sell_min_price).unwrap();
    assert!(
        sell_order_with_fee
            .filled_offer_amount
            .abs_diff(Uint128::from(40000u128))
            < Uint128::from(MIN_VOLUME)
    );
    assert!(
        sell_order_with_fee
            .filled_ask_amount
            .abs_diff(Uint128::from(46000u128))
            < Uint128::from(MIN_VOLUME)
    );

    assert_eq!(matched_order.len(), 4);
    for order in matched_order {
        assert!(order.filled_ask_amount.abs_diff(order.ask_amount) < Uint128::from(MIN_VOLUME));
        assert!(order.filled_offer_amount.abs_diff(order.offer_amount) < Uint128::from(MIN_VOLUME));
    }
}

#[test]
fn test_matching_order_process_offer_amount_smaller_than_lef_match_ask() {
    let offer_info = AssetInfoRaw::NativeToken {
        denom: ATOM_DENOM.to_string(),
    };
    let ask_info = AssetInfoRaw::NativeToken {
        denom: ORAI_DENOM.to_string(),
    };

    let mut deps = mock_dependencies();
    let bidder_addr = deps.api.addr_canonicalize("addr0000").unwrap();

    init_last_order_id(deps.as_mut().storage).unwrap();

    // base on order id 3820823: https://lcd.orai.io/cosmos/tx/v1beta1/txs/6D90EA566FC6DE0336D5665111242C4EDABE8BD460A11844618B34335B1E994F
    let order = Order {
        direction: OrderDirection::Buy,
        order_id: increase_last_order_id(deps.as_mut().storage).unwrap(),
        bidder_addr: bidder_addr.clone(),
        offer_amount: 3502324000u128.into(),
        ask_amount: 196000000u128.into(),
        filled_offer_amount: 3499999968u128.into(),
        filled_ask_amount: 195985264u128.into(),
        status: OrderStatus::PartialFilled,
    };

    let mut ob = OrderBook::new(ask_info, offer_info, None);

    ob.add_order(deps.as_mut().storage, &order).unwrap();

    // base on order id 3820824: https://lcd.orai.io/cosmos/tx/v1beta1/txs/6D90EA566FC6DE0336D5665111242C4EDABE8BD460A11844618B34335B1E994F
    let ask_amount = 7526882u128;
    let offer_amount = 423021u128;
    let sell_price = Decimal::from_ratio(ask_amount, offer_amount);

    let sell_order = Order {
        direction: OrderDirection::Sell,
        order_id: increase_last_order_id(deps.as_mut().storage).unwrap(),
        bidder_addr: bidder_addr.clone(),
        offer_amount: offer_amount.into(),
        ask_amount: ask_amount.into(),
        filled_offer_amount: 0u128.into(),
        filled_ask_amount: 0u128.into(),
        status: OrderStatus::PartialFilled,
    };

    let (_, matched_orders) =
        matching_order(deps.as_ref(), ob.clone(), &sell_order, sell_price).unwrap();
    assert_eq!(matched_orders.len(), 1);

    let matched_order_price = Decimal::from_ratio(
        matched_orders[0].filled_offer_this_round,
        matched_orders[0].filled_ask_this_round,
    );
    println!("matched order price: {:?}", matched_order_price);
    assert_eq!(
        matched_order_price.lt(&Decimal::from_ratio(
            Uint128::from(18u128),
            Uint128::from(1u128)
        )),
        true
    )
}

#[test]
fn test_match_orde_process_minimum_remaining_to_fulfilled() {
    let offer_info = AssetInfoRaw::NativeToken {
        denom: ATOM_DENOM.to_string(),
    };
    let ask_info = AssetInfoRaw::NativeToken {
        denom: ORAI_DENOM.to_string(),
    };

    let mut deps = mock_dependencies();
    let bidder_addr = deps.api.addr_canonicalize("addr0000").unwrap();

    init_last_order_id(deps.as_mut().storage).unwrap();

    let mut ob = OrderBook::new(ask_info.clone(), offer_info.clone(), None);
    ob.min_ask_to_fulfilled = Some(100u128.into());
    ob.min_offer_to_fulfilled = Some(100u128.into());

    // case 1: matched_order if fulfilled because the remaining is less then threshold
    // create a buy order at price 1
    let order = Order {
        direction: OrderDirection::Buy,
        order_id: increase_last_order_id(deps.as_mut().storage).unwrap(),
        bidder_addr: bidder_addr.clone(),
        offer_amount: 1000000u128.into(),
        ask_amount: 1000000u128.into(),
        filled_offer_amount: 0u128.into(),
        filled_ask_amount: 0u128.into(),
        status: OrderStatus::Open,
    };
    ob.add_order(deps.as_mut().storage, &order).unwrap();

    // create a sell order at price 1
    let sell_order = Order {
        direction: OrderDirection::Sell,
        order_id: increase_last_order_id(deps.as_mut().storage).unwrap(),
        bidder_addr: bidder_addr.clone(),
        offer_amount: 999901u128.into(),
        ask_amount: 999901u128.into(),
        filled_offer_amount: 0u128.into(),
        filled_ask_amount: 0u128.into(),
        status: OrderStatus::Open,
    };

    // after matching process, all buy & sell order if fulfilled
    let (sell_order, matched_orders) =
        matching_order(deps.as_ref(), ob.clone(), &sell_order, Decimal::one()).unwrap();
    assert_eq!(sell_order.status, OrderStatus::Fulfilled);
    assert_eq!(matched_orders[0].status, OrderStatus::Fulfilled);

    // case 2: user if fulfilled because the remaining is less then threshold
    let mut deps = mock_dependencies();
    let bidder_addr = deps.api.addr_canonicalize("addr0000").unwrap();

    init_last_order_id(deps.as_mut().storage).unwrap();

    let mut ob = OrderBook::new(ask_info.clone(), offer_info.clone(), None);
    ob.min_ask_to_fulfilled = Some(100u128.into());
    ob.min_offer_to_fulfilled = Some(100u128.into());
    // create a buy order at price 1
    let order = Order {
        direction: OrderDirection::Sell,
        order_id: increase_last_order_id(deps.as_mut().storage).unwrap(),
        bidder_addr: bidder_addr.clone(),
        offer_amount: 999901u128.into(),
        ask_amount: 999901u128.into(),
        filled_offer_amount: 0u128.into(),
        filled_ask_amount: 0u128.into(),
        status: OrderStatus::Open,
    };
    ob.add_order(deps.as_mut().storage, &order).unwrap();

    // create a sell order at price 1
    let buy_order = Order {
        direction: OrderDirection::Buy,
        order_id: increase_last_order_id(deps.as_mut().storage).unwrap(),
        bidder_addr: bidder_addr.clone(),
        offer_amount: 1000000u128.into(),
        ask_amount: 1000000u128.into(),
        filled_offer_amount: 0u128.into(),
        filled_ask_amount: 0u128.into(),
        status: OrderStatus::Open,
    };

    // after matching process, all buy & sell order if fulfilled
    let (buy_order, matched_orders) =
        matching_order(deps.as_ref(), ob.clone(), &buy_order, Decimal::one()).unwrap();
    assert_eq!(buy_order.status, OrderStatus::Fulfilled);
    assert_eq!(matched_orders[0].status, OrderStatus::Fulfilled);

    // case 3, remaining greater threshold => PartialFilled

    let mut deps = mock_dependencies();
    let bidder_addr = deps.api.addr_canonicalize("addr0000").unwrap();

    init_last_order_id(deps.as_mut().storage).unwrap();

    let mut ob = OrderBook::new(ask_info.clone(), offer_info.clone(), None);
    ob.min_ask_to_fulfilled = Some(100u128.into());
    ob.min_offer_to_fulfilled = Some(100u128.into());
    // create a buy order at price 1
    let order = Order {
        direction: OrderDirection::Sell,
        order_id: increase_last_order_id(deps.as_mut().storage).unwrap(),
        bidder_addr: bidder_addr.clone(),
        offer_amount: 999899u128.into(),
        ask_amount: 999899u128.into(),
        filled_offer_amount: 0u128.into(),
        filled_ask_amount: 0u128.into(),
        status: OrderStatus::Open,
    };
    ob.add_order(deps.as_mut().storage, &order).unwrap();

    // create a sell order at price 1
    let buy_order = Order {
        direction: OrderDirection::Buy,
        order_id: increase_last_order_id(deps.as_mut().storage).unwrap(),
        bidder_addr: bidder_addr.clone(),
        offer_amount: 1000000u128.into(),
        ask_amount: 1000000u128.into(),
        filled_offer_amount: 0u128.into(),
        filled_ask_amount: 0u128.into(),
        status: OrderStatus::Open,
    };

    // after matching process, all buy & sell order if fulfilled
    let (buy_order, matched_orders) =
        matching_order(deps.as_ref(), ob.clone(), &buy_order, Decimal::one()).unwrap();
    assert_eq!(buy_order.status, OrderStatus::PartialFilled);
    assert_eq!(matched_orders[0].status, OrderStatus::Fulfilled);
}
