use std::convert::{TryFrom, TryInto};

use cosmwasm_std::{Decimal, Order as OrderBy, StdResult, Storage};
use cosmwasm_storage::ReadonlyBucket;
use oraiswap::{
    limit_order::{OrderDirection, TickResponse, TicksResponse},
    querier::calc_range_start,
};

use crate::state::{DEFAULT_LIMIT, MAX_LIMIT, PREFIX_TICK};

pub fn query_ticks(
    storage: &dyn Storage,
    pair_key: &[u8],
    direction: OrderDirection,
    start_after: Option<Decimal>,
    limit: Option<u32>,
    order_by: Option<i32>,
) -> StdResult<TicksResponse> {
    query_ticks_with_end(
        storage,
        pair_key,
        direction,
        start_after,
        None,
        limit,
        order_by,
    )
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
