use std::str::FromStr;

use cosmwasm_schema::cw_serde;
use cosmwasm_storage::ReadonlyBucket;
use oraiswap::{
    asset::{Asset, AssetInfo},
    limit_order::{OrderDirection, OrderResponse},
    math::Truncate,
};

use cosmwasm_std::{
    Api, CanonicalAddr, Decimal, Order as OrderBy, StdError, StdResult, Storage, Uint128,
};

use crate::state::{
    read_orders, read_orders_with_indexer, store_order, FLOATING_ROUND, PREFIX_ORDER_BY_PRICE,
    PREFIX_TICK,
};

#[cw_serde]
pub struct Order {
    pub order_id: u64,
    pub direction: OrderDirection, // if direction is sell then offer => sell asset, ask => buy asset
    pub bidder_addr: CanonicalAddr,
    pub offer_amount: Uint128,
    pub ask_amount: Uint128,
    pub filled_offer_amount: Uint128,
    pub filled_ask_amount: Uint128,
}

impl Order {
    // create new order given a price and an offer amount
    pub fn new(
        order_id: u64,
        bidder_addr: CanonicalAddr,
        direction: OrderDirection,
        price: Decimal,
        offer_amount: Uint128,
    ) -> Self {
        let (offer_amount, ask_amount) = match direction {
            OrderDirection::Buy => (offer_amount, price * offer_amount),
            OrderDirection::Sell => (price * offer_amount, offer_amount),
        };
        Order {
            direction,
            order_id,
            bidder_addr,
            offer_amount,
            ask_amount,
            filled_offer_amount: Uint128::zero(),
            filled_ask_amount: Uint128::zero(),
        }
    }

    // return matchable offer amount from ask amount, can differ between Sell and Buy
    pub fn matchable_amount(&self, ask_amount: Uint128) -> StdResult<(Uint128, Uint128)> {
        // Compute left offer & ask amount
        let left_offer_amount = self.offer_amount.checked_sub(self.filled_offer_amount)?;
        let left_ask_amount = self.ask_amount.checked_sub(self.filled_ask_amount)?;
        if left_ask_amount < ask_amount || left_offer_amount.is_zero() {
            return Err(StdError::generic_err("insufficient order amount left"));
        }

        // Cap the send amount to left_offer_amount
        Ok((
            if left_ask_amount == ask_amount {
                left_offer_amount
            } else {
                std::cmp::min(left_offer_amount, ask_amount * self.get_price())
            },
            left_ask_amount,
        ))
    }

    pub fn get_price(&self) -> Decimal {
        match self.direction {
            OrderDirection::Buy => Decimal::from_ratio(self.ask_amount, self.offer_amount),
            OrderDirection::Sell => Decimal::from_ratio(self.offer_amount, self.ask_amount),
        }
    }

    pub fn to_response(
        &self,
        api: &dyn Api,
        offer_info: AssetInfo,
        ask_info: AssetInfo,
    ) -> StdResult<OrderResponse> {
        Ok(OrderResponse {
            order_id: self.order_id,
            direction: self.direction.clone(),
            bidder_addr: api.addr_humanize(&self.bidder_addr)?.to_string(),
            offer_asset: Asset {
                amount: self.offer_amount,
                info: offer_info,
            },
            ask_asset: Asset {
                amount: self.ask_amount,
                info: ask_info,
            },
            filled_offer_amount: self.filled_offer_amount,
            filled_ask_amount: self.filled_ask_amount,
        })
    }
}

#[cw_serde]
pub struct Ticks {
    direction: OrderDirection, // buy => price_increasing false,
}

impl Ticks {
    pub fn new(direction: OrderDirection) -> Self {
        Ticks { direction }
    }

    fn best_price(
        &self,
        storage: &dyn Storage,
        pair_key: &[u8],
        price_increasing: bool,
    ) -> (Decimal, u64, bool) {
        // get last tick if price_increasing is true, otherwise get first tick
        let tick_namespaces = &[PREFIX_TICK, pair_key, self.direction.as_bytes()];
        let position_bucket: ReadonlyBucket<u64> =
            ReadonlyBucket::multilevel(storage, tick_namespaces);
        let order_by = if price_increasing {
            OrderBy::Descending
        } else {
            OrderBy::Ascending
        };
        if let Some(item) = position_bucket.range(None, None, order_by).next() {
            if let Ok((price_key, total_orders)) = item {
                // price is rounded already
                let price =
                    Decimal::from_str(unsafe { &String::from_utf8_unchecked(price_key) }).unwrap();
                return (price, total_orders, true);
            }
        }
        (Decimal::zero(), 0, false)
    }

    pub fn highest_price(&self, storage: &dyn Storage, pair_key: &[u8]) -> (Decimal, u64, bool) {
        self.best_price(storage, pair_key, self.direction == OrderDirection::Sell)
    }

    pub fn lowest_price(&self, storage: &dyn Storage, pair_key: &[u8]) -> (Decimal, u64, bool) {
        self.best_price(storage, pair_key, self.direction == OrderDirection::Buy)
    }
}

/// Ticks are stored in Ordered database, so we just need to process at 50 recent ticks is ok
#[cw_serde]
pub struct OrderBook {
    pair_key: Vec<u8>, // an unique pair of assets
    buys: Ticks,
    sells: Ticks,
}

impl OrderBook {
    pub fn new(pair_key: &[u8]) -> Self {
        OrderBook {
            buys: Ticks::new(OrderDirection::Buy),
            sells: Ticks::new(OrderDirection::Sell),
            pair_key: pair_key.to_vec(),
        }
    }

    pub fn add_order(&mut self, storage: &mut dyn Storage, order: &Order) -> StdResult<u64> {
        store_order(storage, &self.pair_key, order, true)
    }

    pub fn orders_at(
        &self,
        storage: &dyn Storage,
        price: Decimal,
        direction: OrderDirection,
        start_after: Option<u64>,
        limit: Option<u32>,
        order_by: Option<OrderBy>,
    ) -> StdResult<Vec<Order>> {
        read_orders_with_indexer::<OrderDirection>(
            storage,
            &[
                PREFIX_ORDER_BY_PRICE,
                &self.pair_key,
                price.to_string_round(FLOATING_ROUND).as_bytes(),
            ],
            Box::new(move |item| direction.eq(item)),
            start_after,
            limit,
            order_by,
        )
    }

    // get_orders returns all orders in the order book, with pagination
    pub fn get_orders(
        &self,
        storage: &dyn Storage,
        start_after: Option<u64>,
        limit: Option<u32>,
        order_by: Option<OrderBy>,
    ) -> StdResult<Vec<Order>> {
        read_orders(storage, &self.pair_key, start_after, limit, order_by)
    }

    pub fn highest_price(&self, storage: &dyn Storage) -> (Decimal, bool) {
        let (highest_buy_price, _, found_buy) = self.buys.highest_price(storage, &self.pair_key);
        let (highest_sell_price, _, found_sell) = self.sells.highest_price(storage, &self.pair_key);
        if found_buy && found_sell {
            return (Decimal::max(highest_buy_price, highest_sell_price), true);
        }
        if found_buy {
            return (highest_buy_price, true);
        }
        if found_sell {
            return (highest_sell_price, true);
        }
        (Decimal::zero(), false)
    }

    pub fn lowest_price(&self, storage: &dyn Storage) -> (Decimal, bool) {
        let (lowest_buy_price, _, found_buy) = self.buys.lowest_price(storage, &self.pair_key);
        let (lowest_sell_price, _, found_sell) = self.sells.lowest_price(storage, &self.pair_key);
        if found_buy && found_sell {
            return (Decimal::min(lowest_buy_price, lowest_sell_price), true);
        }
        if found_buy {
            return (lowest_buy_price, true);
        }
        if found_sell {
            return (lowest_sell_price, true);
        }
        (Decimal::zero(), false)
    }
}
