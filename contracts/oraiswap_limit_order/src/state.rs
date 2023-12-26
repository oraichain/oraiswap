use cosmwasm_std::{CanonicalAddr, Order as OrderBy, StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read, Bucket, ReadonlyBucket};
use oraiswap::{
    limit_order::{ContractInfo, OrderDirection},
    querier::calc_range_start,
};
use serde::{de::DeserializeOwned, Serialize};

use crate::orderbook::{Executor, Order, OrderBook};

// settings for pagination
pub const MAX_LIMIT: u32 = 100;
pub const DEFAULT_LIMIT: u32 = 10;

pub fn init_last_order_id(storage: &mut dyn Storage) -> StdResult<()> {
    singleton(storage, KEY_LAST_ORDER_ID).save(&0u64)
}

pub fn increase_last_order_id(storage: &mut dyn Storage) -> StdResult<u64> {
    singleton(storage, KEY_LAST_ORDER_ID).update(|v| Ok(v + 1))
}

pub fn read_last_order_id(storage: &dyn Storage) -> StdResult<u64> {
    singleton_read(storage, KEY_LAST_ORDER_ID).load()
}

pub fn store_config(storage: &mut dyn Storage, config: &ContractInfo) -> StdResult<()> {
    singleton(storage, CONTRACT_INFO).save(config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<ContractInfo> {
    singleton_read(storage, CONTRACT_INFO).load()
}

pub fn store_reward(
    storage: &mut dyn Storage,
    pair_key: &[u8],
    reward_wallet: &Executor,
) -> StdResult<()> {
    let reward_address_key = &reward_wallet.address;
    Bucket::multilevel(storage, &[PREFIX_REWARD, pair_key]).save(reward_address_key, reward_wallet)
}

pub fn read_reward(
    storage: &dyn Storage,
    pair_key: &[u8],
    address: &CanonicalAddr,
) -> StdResult<Executor> {
    ReadonlyBucket::multilevel(storage, &[PREFIX_REWARD, pair_key]).load(address)
}

pub fn store_orderbook(
    storage: &mut dyn Storage,
    pair_key: &[u8],
    order_book: &OrderBook,
) -> StdResult<()> {
    Bucket::new(storage, PREFIX_ORDER_BOOK).save(pair_key, order_book)
}

// do not return error, by default it return no precision and zero min offer amount
pub fn read_orderbook(storage: &dyn Storage, pair_key: &[u8]) -> StdResult<OrderBook> {
    ReadonlyBucket::new(storage, PREFIX_ORDER_BOOK).load(pair_key)
}

pub fn read_orderbooks(
    storage: &dyn Storage,
    start_after: Option<Vec<u8>>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
) -> StdResult<Vec<OrderBook>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let (start, end, order_by) = match order_by {
        Some(OrderBy::Ascending) => (calc_range_start(start_after), None, OrderBy::Ascending),
        _ => (None, start_after, OrderBy::Descending),
    };
    ReadonlyBucket::new(storage, PREFIX_ORDER_BOOK)
        .range(start.as_deref(), end.as_deref(), order_by)
        .take(limit)
        .map(|item| item.map(|item| item.1))
        .collect()
}

pub fn remove_orderbook<'a>(storage: &'a mut dyn Storage, pair_key: &[u8]) {
    Bucket::<'a, OrderBook>::new(storage, PREFIX_ORDER_BOOK).remove(pair_key)
}

pub fn store_order(
    storage: &mut dyn Storage,
    pair_key: &[u8],
    order: &Order,
    inserted: bool,
) -> StdResult<u64> {
    let order_id_key = &order.order_id.to_be_bytes();
    let price_key = order.get_price().atomics().to_be_bytes();

    Bucket::multilevel(storage, &[PREFIX_ORDER, pair_key]).save(order_id_key, order)?;

    let tick_namespaces = &[PREFIX_TICK, pair_key, order.direction.as_bytes()];

    // first time then total is 0
    let mut total_tick_orders = ReadonlyBucket::<u64>::multilevel(storage, tick_namespaces)
        .load(&price_key)
        .unwrap_or_default();

    if inserted {
        total_tick_orders += 1;
    }

    // save total orders for a tick
    Bucket::multilevel(storage, tick_namespaces).save(&price_key, &total_tick_orders)?;

    // index order by price and pair key ?, store tick using price as key then sort by ID ?
    // => query tick price from pair key => each price query order belong to price => order list
    // insert tick => insert price entry for pair_key of prefix tick
    // insert order to tick => update index for [pair key, price]
    Bucket::multilevel(storage, &[PREFIX_ORDER_BY_PRICE, pair_key, &price_key])
        .save(order_id_key, &order.direction)?;

    Bucket::multilevel(
        storage,
        &[
            PREFIX_ORDER_BY_BIDDER,
            pair_key,
            order.bidder_addr.as_slice(),
        ],
    )
    .save(order_id_key, &order.direction)?;

    Bucket::multilevel(
        storage,
        &[
            PREFIX_ORDER_BY_DIRECTION,
            pair_key,
            &order.direction.as_bytes(),
        ],
    )
    .save(order_id_key, &order.direction)?;

    Ok(total_tick_orders)
}

pub fn remove_order(storage: &mut dyn Storage, pair_key: &[u8], order: &Order) -> StdResult<u64> {
    let order_id_key = &order.order_id.to_be_bytes();
    let price_key = order.get_price().atomics().to_be_bytes();

    Bucket::<Order>::multilevel(storage, &[PREFIX_ORDER, pair_key]).remove(order_id_key);

    // not found means total is 0
    let tick_namespaces = &[PREFIX_TICK, pair_key, order.direction.as_bytes()];
    let mut total_tick_orders = ReadonlyBucket::<u64>::multilevel(storage, tick_namespaces)
        .load(&price_key)
        .unwrap_or_default();

    // substract one order, if total is 0 mean not existed
    if total_tick_orders > 0 {
        total_tick_orders -= 1;
        if total_tick_orders > 0 {
            // save total orders for a tick
            Bucket::multilevel(storage, tick_namespaces)
                .save(&price_key, &total_tick_orders)
                .unwrap();
        } else {
            Bucket::<u64>::multilevel(storage, tick_namespaces).remove(&price_key);
        }
    }

    // value is just bool to represent indexer
    Bucket::<OrderDirection>::multilevel(storage, &[PREFIX_ORDER_BY_PRICE, pair_key, &price_key])
        .remove(order_id_key);

    Bucket::<OrderDirection>::multilevel(
        storage,
        &[
            PREFIX_ORDER_BY_BIDDER,
            pair_key,
            order.bidder_addr.as_slice(),
        ],
    )
    .remove(order_id_key);

    Bucket::<OrderDirection>::multilevel(
        storage,
        &[
            PREFIX_ORDER_BY_DIRECTION,
            pair_key,
            &order.direction.as_bytes(),
        ],
    )
    .remove(order_id_key);

    // return total orders belong to the tick
    Ok(total_tick_orders)
}

pub fn read_order(storage: &dyn Storage, pair_key: &[u8], order_id: u64) -> StdResult<Order> {
    ReadonlyBucket::multilevel(storage, &[PREFIX_ORDER, pair_key]).load(&order_id.to_be_bytes())
}

/// read_orders_with_indexer: namespace is PREFIX + PAIR_KEY + INDEXER
pub fn read_orders_with_indexer<T: Serialize + DeserializeOwned>(
    storage: &dyn Storage,
    namespaces: &[&[u8]],
    filter: Box<dyn Fn(&T) -> bool>,
    start_after: Option<u64>,
    limit: Option<u32>,
    order_by: Option<OrderBy>,
) -> StdResult<Option<Vec<Order>>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start_after = start_after.map(|id| id.to_be_bytes().to_vec());
    let (start, end, order_by) = match order_by {
        Some(OrderBy::Ascending) => (calc_range_start(start_after), None, OrderBy::Ascending),
        _ => (None, start_after, OrderBy::Descending),
    };

    // just get 1 byte of value is ok
    let position_indexer: ReadonlyBucket<T> = ReadonlyBucket::multilevel(storage, namespaces);
    let order_bucket = ReadonlyBucket::multilevel(storage, &[PREFIX_ORDER, namespaces[1]]);

    position_indexer
        .range(start.as_deref(), end.as_deref(), order_by)
        .filter(|item| item.as_ref().map_or(false, |item| filter(&item.1)))
        .take(limit)
        .map(|item| order_bucket.may_load(&item?.0))
        .collect()
}

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
    let start_after = start_after.map(|id| id.to_be_bytes().to_vec());
    let (start, end, order_by) = match order_by {
        Some(OrderBy::Ascending) => (calc_range_start(start_after), None, OrderBy::Ascending),
        _ => (None, start_after, OrderBy::Descending),
    };

    position_bucket
        .range(start.as_deref(), end.as_deref(), order_by)
        .take(limit)
        .map(|item| item.map(|item| item.1))
        .collect()
}

static KEY_LAST_ORDER_ID: &[u8] = b"last_order_id"; // should use big int? guess no need
static CONTRACT_INFO: &[u8] = b"contract_info"; // contract info
static PREFIX_ORDER_BOOK: &[u8] = b"order_book"; // store config for an order book like min ask amount and min sell amount
static PREFIX_ORDER: &[u8] = b"order"; // this is orderbook
static PREFIX_REWARD: &[u8] = b"reward_wallet"; // executor that running matching engine for orderbook pair

pub static PREFIX_ORDER_BY_BIDDER: &[u8] = b"order_by_bidder"; // order from a bidder
pub static PREFIX_ORDER_BY_PRICE: &[u8] = b"order_by_price"; // this where orders belong to tick
pub static PREFIX_ORDER_BY_DIRECTION: &[u8] = b"order_by_direction"; // order from the direction
pub static PREFIX_TICK: &[u8] = b"tick"; // this is tick with value is the total orders
