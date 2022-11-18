use std::convert::TryFrom;

use cosmwasm_std::{Decimal, Deps, Order as OrderBy, StdResult};
use cosmwasm_storage::ReadonlyBucket;
use oraiswap::{
    asset::{pair_key, AssetInfo},
    limit_order::{OrderDirection, TickResponse, TicksResponse},
    math::Truncate,
};

use crate::state::{DEFAULT_LIMIT, FLOATING_ROUND, MAX_LIMIT, PREFIX_TICK};

// this will set the first key after the provided key, by appending a 1 byte
fn calc_range_start(start_after: Option<Decimal>) -> Option<Vec<u8>> {
    start_after.map(|id| {
        let mut v = id.to_string_round(FLOATING_ROUND).into_bytes();
        v.push(1);
        v
    })
}

// this will set the first key after the provided key, by appending a 1 byte
fn calc_range_end(start_after: Option<Decimal>) -> Option<Vec<u8>> {
    start_after.map(|id| id.to_string_round(FLOATING_ROUND).into_bytes())
}

pub fn query_ticks(
    deps: Deps,
    offer_info: AssetInfo,
    ask_info: AssetInfo,
    direction: OrderDirection,
    start_after: Option<Decimal>,
    limit: Option<u32>,
    order_by: Option<i32>,
) -> StdResult<TicksResponse> {
    let order_by = order_by.map_or(None, |val| OrderBy::try_from(val).ok());
    let pair_key = pair_key(&[offer_info.to_raw(deps.api)?, ask_info.to_raw(deps.api)?]);
    let position_bucket: ReadonlyBucket<u64> = ReadonlyBucket::multilevel(
        deps.storage,
        &[PREFIX_TICK, &pair_key, direction.as_bytes()],
    );

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let (start, end, order_by) = match order_by {
        Some(OrderBy::Ascending) => (calc_range_start(start_after), None, OrderBy::Ascending),
        _ => (None, calc_range_end(start_after), OrderBy::Descending),
    };

    let ticks = position_bucket
        .range(start.as_deref(), end.as_deref(), order_by.into())
        .take(limit)
        .map(|item| {
            let (k, total_orders) = item?;
            let price = unsafe { String::from_utf8_unchecked(k) };
            Ok(TickResponse {
                price,
                total_orders,
            })
        })
        .collect::<StdResult<Vec<TickResponse>>>()?;

    Ok(TicksResponse { ticks })
}

pub fn query_tick(
    deps: Deps,
    offer_info: AssetInfo,
    ask_info: AssetInfo,
    direction: OrderDirection,
    price: Decimal,
) -> StdResult<TickResponse> {
    let pair_key = pair_key(&[offer_info.to_raw(deps.api)?, ask_info.to_raw(deps.api)?]);
    let price = price.to_string_round(FLOATING_ROUND);
    let total_orders = ReadonlyBucket::<u64>::multilevel(
        deps.storage,
        &[PREFIX_TICK, &pair_key, direction.as_bytes()],
    )
    .load(price.as_bytes())?;

    Ok(TickResponse {
        price,
        total_orders,
    })
}
