use crate::orderbook::{Order, OrderBook};
use crate::state::{
    read_last_order_id, read_order, read_orderbook, read_orderbooks, read_orders,
    read_orders_with_indexer, PREFIX_ORDER_BY_BIDDER, PREFIX_ORDER_BY_DIRECTION,
    PREFIX_ORDER_BY_PRICE, PREFIX_TICK,
};
use cosmwasm_std::{Decimal, Deps, Order as OrderBy, StdResult, Storage, Uint128};
use oraiswap::limit_order::BaseAmountResponse;
use std::convert::{TryFrom, TryInto};

use oraiswap::asset::{pair_key, AssetInfo};
use oraiswap::{
    limit_order::{
        LastOrderIdResponse, OrderBookMatchableResponse, OrderBookResponse, OrderBooksResponse,
        OrderDirection, OrderFilter, OrderResponse, OrdersResponse, TickResponse, TicksResponse,
    },
    querier::calc_range_start,
};

use cosmwasm_storage::ReadonlyBucket;

use crate::state::{DEFAULT_LIMIT, MAX_LIMIT};

pub fn query_order(
    deps: Deps,
    asset_infos: [AssetInfo; 2],
    order_id: u64,
) -> StdResult<OrderResponse> {
    let pair_key = pair_key(&[
        asset_infos[0].to_raw(deps.api)?,
        asset_infos[1].to_raw(deps.api)?,
    ]);
    let orderbook_pair = read_orderbook(deps.storage, &pair_key)?;
    let order = read_order(deps.storage, &pair_key, order_id)?;

    order.to_response(
        deps.api,
        orderbook_pair.base_coin_info.to_normal(deps.api)?,
        orderbook_pair.quote_coin_info.to_normal(deps.api)?,
    )
}

pub fn query_orders(
    deps: Deps,
    asset_infos: [AssetInfo; 2],
    direction: Option<OrderDirection>,
    filter: OrderFilter,
    start_after: Option<u64>,
    limit: Option<u32>,
    order_by: Option<i32>,
) -> StdResult<OrdersResponse> {
    let order_by = order_by.map_or(None, |val| OrderBy::try_from(val).ok());
    let pair_key = pair_key(&[
        asset_infos[0].to_raw(deps.api)?,
        asset_infos[1].to_raw(deps.api)?,
    ]);
    let orderbook_pair = read_orderbook(deps.storage, &pair_key)?;

    let (direction_filter, direction_key): (Box<dyn Fn(&OrderDirection) -> bool>, Vec<u8>) =
        match direction {
            // copy value to closure
            Some(d) => (Box::new(move |x| d.eq(x)), d.as_bytes().to_vec()),
            None => (Box::new(|_| true), OrderDirection::Buy.as_bytes().to_vec()),
        };

    let orders: Option<Vec<Order>> = match filter {
        OrderFilter::Bidder(bidder_addr) => {
            let bidder_addr_raw = deps.api.addr_canonicalize(&bidder_addr)?;
            read_orders_with_indexer::<OrderDirection>(
                deps.storage,
                &[
                    PREFIX_ORDER_BY_BIDDER,
                    &pair_key,
                    bidder_addr_raw.as_slice(),
                ],
                direction_filter,
                start_after,
                limit,
                order_by,
            )?
        }
        OrderFilter::Tick {} => read_orders_with_indexer::<u64>(
            deps.storage,
            &[PREFIX_TICK, &pair_key, &direction_key],
            Box::new(|_| true),
            start_after,
            limit,
            order_by,
        )?,
        OrderFilter::Price(price) => {
            let price_key = price.atomics().to_be_bytes();
            read_orders_with_indexer::<OrderDirection>(
                deps.storage,
                &[PREFIX_ORDER_BY_PRICE, &pair_key, &price_key],
                direction_filter,
                start_after,
                limit,
                order_by,
            )?
        }
        OrderFilter::None => match direction {
            Some(_) => read_orders_with_indexer::<OrderDirection>(
                deps.storage,
                &[PREFIX_ORDER_BY_DIRECTION, &pair_key, &direction_key],
                direction_filter,
                start_after,
                limit,
                order_by,
            )?,
            None => Some(read_orders(
                deps.storage,
                &pair_key,
                start_after,
                limit,
                order_by,
            )?),
        },
    };

    let resp = OrdersResponse {
        orders: orders
            .unwrap_or_default()
            .iter()
            .map(|order| {
                order.to_response(
                    deps.api,
                    orderbook_pair.base_coin_info.to_normal(deps.api)?,
                    orderbook_pair.quote_coin_info.to_normal(deps.api)?,
                )
            })
            .collect::<StdResult<Vec<OrderResponse>>>()?,
    };

    Ok(resp)
}

pub fn query_price_by_base_amount(
    deps: Deps,
    orderbook_pair: &OrderBook,
    direction: OrderDirection,
    base_amount: Uint128,
) -> StdResult<BaseAmountResponse> {
    // We have to query opposite direction to fill order
    let price_direction = match direction {
        OrderDirection::Buy => OrderDirection::Sell,
        OrderDirection::Sell => OrderDirection::Buy,
    };

    let order_by = match price_direction {
        OrderDirection::Buy => Some(2i32),
        OrderDirection::Sell => Some(1i32),
    };
    // get best price list base on direction
    let best_price_list = query_ticks_prices_with_end(
        deps.storage,
        &orderbook_pair.get_pair_key(),
        price_direction,
        None,
        None,
        None,
        order_by,
    );

    let mut total_base_amount_by_price = Uint128::zero();
    let mut market_price = Decimal::zero();
    let mut expected_base_amount = base_amount;
    for price in &best_price_list {
        let base_amount_by_price =
            orderbook_pair.find_base_amount_at_price(deps.storage, *price, price_direction);
        total_base_amount_by_price =
            total_base_amount_by_price.checked_add(base_amount_by_price)?;
        market_price = *price;
        if total_base_amount_by_price >= base_amount {
            break;
        }
    }
    if total_base_amount_by_price < base_amount {
        expected_base_amount = total_base_amount_by_price;
    }
    Ok(BaseAmountResponse {
        market_price,
        expected_base_amount,
    })
}

pub fn query_last_order_id(deps: Deps) -> StdResult<LastOrderIdResponse> {
    let last_order_id = read_last_order_id(deps.storage)?;
    let resp = LastOrderIdResponse { last_order_id };
    Ok(resp)
}

pub fn query_orderbooks(
    deps: Deps,
    start_after: Option<Vec<u8>>,
    limit: Option<u32>,
    order_by: Option<i32>,
) -> StdResult<OrderBooksResponse> {
    let order_by = order_by.map_or(None, |val| OrderBy::try_from(val).ok());
    let order_books = read_orderbooks(deps.storage, start_after, limit, order_by)?;
    order_books
        .into_iter()
        .map(|ob| ob.to_response(deps.api))
        .collect::<StdResult<Vec<OrderBookResponse>>>()
        .map(|order_books| OrderBooksResponse { order_books })
}

pub fn query_orderbook(deps: Deps, asset_infos: [AssetInfo; 2]) -> StdResult<OrderBookResponse> {
    let pair_key = pair_key(&[
        asset_infos[0].to_raw(deps.api)?,
        asset_infos[1].to_raw(deps.api)?,
    ]);
    let ob = read_orderbook(deps.storage, &pair_key)?;
    ob.to_response(deps.api)
}

pub fn query_orderbook_is_matchable(
    deps: Deps,
    asset_infos: [AssetInfo; 2],
) -> StdResult<OrderBookMatchableResponse> {
    let pair_key = pair_key(&[
        asset_infos[0].to_raw(deps.api)?,
        asset_infos[1].to_raw(deps.api)?,
    ]);
    let ob = read_orderbook(deps.storage, &pair_key)?;
    let (best_buy_price_list, best_sell_price_list) = ob
        .find_list_match_price(deps.storage, Some(30))
        .unwrap_or_default();

    Ok(OrderBookMatchableResponse {
        is_matchable: best_buy_price_list.len() != 0 && best_sell_price_list.len() != 0,
    })
}

pub fn query_ticks_prices(
    storage: &dyn Storage,
    pair_key: &[u8],
    direction: OrderDirection,
    start_after: Option<Decimal>,
    limit: Option<u32>,
    order_by: Option<i32>,
) -> Vec<Decimal> {
    query_ticks_prices_with_end(
        storage,
        pair_key,
        direction,
        start_after,
        None,
        limit,
        order_by,
    )
}

pub fn query_ticks_prices_with_end(
    storage: &dyn Storage,
    pair_key: &[u8],
    direction: OrderDirection,
    start_after: Option<Decimal>,
    end: Option<Decimal>,
    limit: Option<u32>,
    order_by: Option<i32>,
) -> Vec<Decimal> {
    query_ticks_with_end(
        storage,
        pair_key,
        direction,
        start_after,
        end,
        limit,
        order_by,
    )
    .unwrap_or(TicksResponse { ticks: vec![] })
    .ticks
    .into_iter()
    .map(|tick| tick.price)
    .collect::<Vec<Decimal>>()
}

pub fn query_ticks_with_end(
    storage: &dyn Storage,
    pair_key: &[u8],
    direction: OrderDirection,
    start_after: Option<Decimal>,
    end: Option<Decimal>,
    limit: Option<u32>,
    order_by: Option<i32>,
) -> StdResult<TicksResponse> {
    let order_by = order_by.map_or(None, |val| OrderBy::try_from(val).ok());

    let position_bucket: ReadonlyBucket<u64> =
        ReadonlyBucket::multilevel(storage, &[PREFIX_TICK, pair_key, direction.as_bytes()]);

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start_after = start_after.map(|id| id.atomics().to_be_bytes().to_vec());
    let end = end.map(|id| id.atomics().to_be_bytes().to_vec());

    let (start, end, order_by) = match order_by {
        // start_after < x <= end
        Some(OrderBy::Ascending) => (
            calc_range_start(start_after),
            calc_range_start(end),
            OrderBy::Ascending,
        ),
        // start_after < x <= end
        _ => (end, start_after, OrderBy::Descending),
    };

    let ticks = position_bucket
        .range(start.as_deref(), end.as_deref(), order_by)
        .take(limit)
        .map(|item| {
            let (k, total_orders) = item?;
            let price = Decimal::raw(u128::from_be_bytes(k.try_into().unwrap()));
            Ok(TickResponse {
                price,
                total_orders,
            })
        })
        .collect::<StdResult<Vec<TickResponse>>>()?;

    Ok(TicksResponse { ticks })
}

pub fn query_tick(
    storage: &dyn Storage,
    pair_key: &[u8],
    direction: OrderDirection,
    price: Decimal,
) -> StdResult<TickResponse> {
    let price_key = price.atomics().to_be_bytes();
    let total_orders =
        ReadonlyBucket::<u64>::multilevel(storage, &[PREFIX_TICK, pair_key, direction.as_bytes()])
            .load(&price_key)?;

    Ok(TickResponse {
        price,
        total_orders,
    })
}
