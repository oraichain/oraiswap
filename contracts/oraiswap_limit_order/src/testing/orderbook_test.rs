use std::str::FromStr;

use cosmwasm_std::{testing::mock_dependencies, Api, Decimal};
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

    let mut ob = OrderBook::new(&pair_key);

    for order in orders {
        let total_orders = ob.add_order(deps.as_mut().storage, &order).unwrap();
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

    let (highest, found) = ob.highest_price(deps.as_ref().storage);
    assert!(found);
    assert_eq!(highest, Decimal::from_str("10.01").unwrap());

    let (lowest, found) = ob.lowest_price(deps.as_ref().storage);
    assert!(found);
    assert_eq!(lowest, Decimal::from_str("9.996").unwrap());
}

// #[test]
// fn buy_orders_at() {
//     let deps = mock_dependencies();

//     let buy_info = AssetInfoRaw::NativeToken {
//         denom: ORAI_DENOM.to_string(),
//     };
//     let sell_info = AssetInfoRaw::NativeToken {
//         denom: ATOM_DENOM.to_string(),
//     };
//     let bidder_addr = deps.api.addr_canonicalize("addr0000").unwrap();
//     let orders = vec![
//         Order::new(
//             1u64,
//             bidder_addr.clone(),
//             OrderDirection::Sell,
//             Decimal::from_str("1.1").unwrap(),
//             10000u128.into(),
//         ),
//         Order::new(
//             2u64,
//             bidder_addr.clone(),
//             OrderDirection::Buy,
//             Decimal::from_str("1.1").unwrap(),
//             10000u128.into(),
//         ),
//         Order::new(
//             3u64,
//             bidder_addr.clone(),
//             OrderDirection::Buy,
//             Decimal::from_str("1.1").unwrap(),
//             10000u128.into(),
//         ),
//         Order::new(
//             4u64,
//             bidder_addr.clone(),
//             OrderDirection::Buy,
//             Decimal::from_str("1.0").unwrap(),
//             10000u128.into(),
//         ),
//         Order::new(
//             5u64,
//             bidder_addr.clone(),
//             OrderDirection::Buy,
//             Decimal::from_str("1.2").unwrap(),
//             10000u128.into(),
//         ),
//     ];

//     let ob = OrderBook::new(buy_info, sell_info, orders.clone());

//     let buy_orders = ob.buy_orders_at(Decimal::from_str("1.1").unwrap());
//     assert_eq!(buy_orders.len(), 2);
//     assert_eq!(buy_orders, orders[1..=2]);

//     assert!(ob
//         .buy_orders_at(Decimal::from_str("0.9").unwrap())
//         .is_empty());
// }

// #[test]
// fn sell_orders_at() {
//     let deps = mock_dependencies();

//     let buy_info = AssetInfoRaw::NativeToken {
//         denom: ORAI_DENOM.to_string(),
//     };
//     let sell_info = AssetInfoRaw::NativeToken {
//         denom: ATOM_DENOM.to_string(),
//     };
//     let bidder_addr = deps.api.addr_canonicalize("addr0000").unwrap();
//     let orders = vec![
//         Order::new(
//             1u64,
//             bidder_addr.clone(),
//             OrderDirection::Buy,
//             Decimal::from_str("1.1").unwrap(),
//             10000u128.into(),
//         ),
//         Order::new(
//             2u64,
//             bidder_addr.clone(),
//             OrderDirection::Sell,
//             Decimal::from_str("1.1").unwrap(),
//             10000u128.into(),
//         ),
//         Order::new(
//             3u64,
//             bidder_addr.clone(),
//             OrderDirection::Sell,
//             Decimal::from_str("1.1").unwrap(),
//             10000u128.into(),
//         ),
//         Order::new(
//             4u64,
//             bidder_addr.clone(),
//             OrderDirection::Sell,
//             Decimal::from_str("1.0").unwrap(),
//             10000u128.into(),
//         ),
//         Order::new(
//             5u64,
//             bidder_addr.clone(),
//             OrderDirection::Sell,
//             Decimal::from_str("1.2").unwrap(),
//             10000u128.into(),
//         ),
//     ];

//     let ob = OrderBook::new(buy_info, sell_info, orders.clone());
//     let sell_orders = ob.sell_orders_at(Decimal::from_str("1.1").unwrap());
//     assert_eq!(sell_orders.len(), 2);
//     assert_eq!(sell_orders, orders[1..=2]);
//     assert!(ob
//         .sell_orders_at(Decimal::from_str("0.9").unwrap())
//         .is_empty());
// }

// #[test]
// fn highest_price_lowest_price() {
//     let deps = mock_dependencies();

//     let buy_info = AssetInfoRaw::NativeToken {
//         denom: ORAI_DENOM.to_string(),
//     };
//     let sell_info = AssetInfoRaw::NativeToken {
//         denom: ATOM_DENOM.to_string(),
//     };
//     let bidder_addr = deps.api.addr_canonicalize("addr0000").unwrap();
//     struct HighestLowestPrice {
//         ob: OrderBook,
//         found: bool,
//         highest_price: Decimal,
//         lowest_price: Decimal,
//     }

//     let test_cases: Vec<HighestLowestPrice> = vec![
//         HighestLowestPrice {
//             ob: OrderBook::new(buy_info.clone(), sell_info.clone(), vec![]),
//             found: false,
//             highest_price: Decimal::zero(),
//             lowest_price: Decimal::zero(),
//         },
//         HighestLowestPrice {
//             ob: OrderBook::new(
//                 buy_info.clone(),
//                 sell_info.clone(),
//                 vec![
//                     Order::new(
//                         1u64,
//                         bidder_addr.clone(),
//                         OrderDirection::Buy,
//                         Decimal::from_str("1.1").unwrap(),
//                         10000u128.into(),
//                     ),
//                     Order::new(
//                         2u64,
//                         bidder_addr.clone(),
//                         OrderDirection::Buy,
//                         Decimal::from_str("1.0").unwrap(),
//                         10000u128.into(),
//                     ),
//                 ],
//             ),
//             found: true,
//             highest_price: Decimal::from_str("1.1").unwrap(),
//             lowest_price: Decimal::from_str("1.0").unwrap(),
//         },
//         HighestLowestPrice {
//             ob: OrderBook::new(
//                 buy_info.clone(),
//                 sell_info.clone(),
//                 vec![
//                     Order::new(
//                         3u64,
//                         bidder_addr.clone(),
//                         OrderDirection::Sell,
//                         Decimal::from_str("1.1").unwrap(),
//                         10000u128.into(),
//                     ),
//                     Order::new(
//                         4u64,
//                         bidder_addr.clone(),
//                         OrderDirection::Sell,
//                         Decimal::from_str("1.0").unwrap(),
//                         10000u128.into(),
//                     ),
//                 ],
//             ),
//             found: true,
//             highest_price: Decimal::from_str("1.1").unwrap(),
//             lowest_price: Decimal::from_str("1.0").unwrap(),
//         },
//         HighestLowestPrice {
//             ob: OrderBook::new(
//                 buy_info.clone(),
//                 sell_info.clone(),
//                 vec![
//                     Order::new(
//                         5u64,
//                         bidder_addr.clone(),
//                         OrderDirection::Sell,
//                         Decimal::from_str("1.1").unwrap(),
//                         10000u128.into(),
//                     ),
//                     Order::new(
//                         6u64,
//                         bidder_addr.clone(),
//                         OrderDirection::Sell,
//                         Decimal::from_str("1.0").unwrap(),
//                         10000u128.into(),
//                     ),
//                     Order::new(
//                         7u64,
//                         bidder_addr.clone(),
//                         OrderDirection::Buy,
//                         Decimal::from_str("1.0").unwrap(),
//                         10000u128.into(),
//                     ),
//                     Order::new(
//                         8u64,
//                         bidder_addr.clone(),
//                         OrderDirection::Buy,
//                         Decimal::from_str("0.9").unwrap(),
//                         10000u128.into(),
//                     ),
//                 ],
//             ),
//             found: true,
//             highest_price: Decimal::from_str("1.1").unwrap(),
//             lowest_price: Decimal::from_str("0.9").unwrap(),
//         },
//     ];

//     for tc in test_cases {
//         let (highest, found) = tc.ob.highest_price();
//         assert_eq!(tc.found, found);
//         let (lowest, found) = tc.ob.lowest_price();
//         assert_eq!(tc.found, found);
//         if tc.found {
//             assert_eq!(tc.highest_price, highest);
//             assert_eq!(tc.lowest_price, lowest);
//         }
//     }
// }
