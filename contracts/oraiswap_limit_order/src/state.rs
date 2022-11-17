use cosmwasm_std::{CanonicalAddr, Order as OrderBy, StdError, StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read, Bucket, ReadonlyBucket};
use oraiswap::math::Truncate;
use std::convert::TryInto;

use crate::orderbook::Order;

// only show tick with 3 floating number place
const FLOATING_ROUND: usize = 3;

static KEY_LAST_ORDER_ID: &[u8] = b"last_order_id";
static PREFIX_ORDER: &[u8] = b"order";
static PREFIX_ORDER_BY_BIDDER: &[u8] = b"order_by_bidder";
static PREFIX_ORDER_BY_PRICE: &[u8] = b"order_by_price";

pub fn init_last_order_id(storage: &mut dyn Storage) -> StdResult<()> {
    singleton(storage, KEY_LAST_ORDER_ID).save(&0u64)
}

pub fn increase_last_order_id(storage: &mut dyn Storage) -> StdResult<u64> {
    singleton(storage, KEY_LAST_ORDER_ID).update(|v| Ok(v + 1))
}

pub fn read_last_order_id(storage: &dyn Storage) -> StdResult<u64> {
    singleton_read(storage, KEY_LAST_ORDER_ID).load()
}

pub fn store_order(storage: &mut dyn Storage, pair_key: &[u8], order: &Order) -> StdResult<()> {
    let order_id_key = &order.order_id.to_le_bytes();

    Bucket::multilevel(storage, &[PREFIX_ORDER, pair_key]).save(order_id_key, order)?;
    // index order by price and pair key ?, store tick using price as key then sort by ID ?
    // => query tick price from pair key => each price query order belong to price => order list
    // insert tick => insert price entry for pair_key of prefix tick
    // insert order to tick => update index for [pair key, price]

    Bucket::multilevel(
        storage,
        &[
            PREFIX_ORDER_BY_PRICE,
            pair_key,
            order.get_price().to_string_round(FLOATING_ROUND).as_bytes(),
        ],
    )
    .save(order_id_key, &true)?;

    Bucket::multilevel(
        storage,
        &[
            PREFIX_ORDER_BY_BIDDER,
            pair_key,
            order.bidder_addr.as_slice(),
        ],
    )
    .save(order_id_key, &true)?;

    Ok(())
}

pub fn remove_order(storage: &mut dyn Storage, pair_key: &[u8], order: &Order) {
    let order_id_key = &order.order_id.to_le_bytes();
    Bucket::<Order>::multilevel(storage, &[PREFIX_ORDER, pair_key]).remove(order_id_key);

    // value is just bool to represent indexer
    Bucket::<bool>::multilevel(
        storage,
        &[
            PREFIX_ORDER_BY_PRICE,
            pair_key,
            order.get_price().to_string_round(FLOATING_ROUND).as_bytes(),
        ],
    )
    .remove(order_id_key);

    Bucket::<bool>::multilevel(
        storage,
        &[
            PREFIX_ORDER_BY_BIDDER,
            pair_key,
            order.bidder_addr.as_slice(),
        ],
    )
    .remove(order_id_key);
}

pub fn read_order(storage: &dyn Storage, pair_key: &[u8], order_id: u64) -> StdResult<Order> {
    ReadonlyBucket::multilevel(storage, &[PREFIX_ORDER, pair_key]).load(&order_id.to_le_bytes())
}

pub fn read_orders_with_bidder_indexer(
    storage: &dyn Storage,
    bidder_addr: &CanonicalAddr,
    pair_key: &[u8],
    start_after: Option<u64>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
) -> StdResult<Vec<Order>> {
    let position_indexer: ReadonlyBucket<bool> = ReadonlyBucket::multilevel(
        storage,
        &[PREFIX_ORDER_BY_BIDDER, pair_key, bidder_addr.as_slice()],
    );

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let (start, end, order_by) = match order_by {
        Some(OrderBy::Ascending) => (calc_range_start(start_after), None, OrderBy::Ascending),
        _ => (None, calc_range_end(start_after), OrderBy::Descending),
    };

    position_indexer
        .range(start.as_deref(), end.as_deref(), order_by.into())
        .take(limit)
        .map(|item| {
            let (k, _) = item?;
            read_order(storage, pair_key, bytes_to_u64(&k)?)
        })
        .collect()
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;
pub fn read_orders(
    storage: &dyn Storage,
    pair_key: &[u8],
    start_after: Option<u64>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
) -> StdResult<Vec<Order>> {
    let position_bucket: ReadonlyBucket<Order> =
        ReadonlyBucket::multilevel(storage, &[PREFIX_ORDER, pair_key]);

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let (start, end, order_by) = match order_by {
        Some(OrderBy::Ascending) => (calc_range_start(start_after), None, OrderBy::Ascending),
        _ => (None, calc_range_end(start_after), OrderBy::Descending),
    };

    position_bucket
        .range(start.as_deref(), end.as_deref(), order_by.into())
        .take(limit)
        .map(|item| {
            let (_, v) = item?;
            Ok(v)
        })
        .collect()
}

fn bytes_to_u64(data: &[u8]) -> StdResult<u64> {
    match data[0..8].try_into() {
        Ok(bytes) => Ok(u64::from_le_bytes(bytes)),
        Err(_) => Err(StdError::generic_err(
            "Corrupted data found. 8 byte expected.",
        )),
    }
}

// this will set the first key after the provided key, by appending a 1 byte
fn calc_range_start(start_after: Option<u64>) -> Option<Vec<u8>> {
    start_after.map(|id| {
        let mut v = id.to_le_bytes().to_vec();
        v.push(1);
        v
    })
}

// this will set the first key after the provided key, by appending a 1 byte
fn calc_range_end(start_after: Option<u64>) -> Option<Vec<u8>> {
    start_after.map(|id| id.to_le_bytes().to_vec())
}
