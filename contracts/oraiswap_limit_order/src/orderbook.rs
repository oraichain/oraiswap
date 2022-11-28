use std::convert::TryInto;

use cosmwasm_schema::cw_serde;
use cosmwasm_storage::ReadonlyBucket;
use oraiswap::{
    asset::{Asset, AssetInfo},
    limit_order::{OrderDirection, OrderResponse},
};

use cosmwasm_std::{
    Api, CanonicalAddr, Decimal, Order as OrderBy, StdError, StdResult, Storage, Uint128,
};

use crate::state::{
    read_orders, read_orders_with_indexer, store_order, PREFIX_ORDER_BY_PRICE, PREFIX_TICK,
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
        amount: Uint128,
    ) -> Self {
        let (offer_amount, ask_amount) = match direction {
            OrderDirection::Buy => (amount, price * amount),
            OrderDirection::Sell => (price * amount, amount),
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
        price_increasing: OrderBy,
    ) -> (Decimal, bool, u64) {
        // get last tick if price_increasing is true, otherwise get first tick
        let tick_namespaces = &[PREFIX_TICK, pair_key, self.direction.as_bytes()];
        let position_bucket: ReadonlyBucket<u64> =
            ReadonlyBucket::multilevel(storage, tick_namespaces);

        if let Some(item) = position_bucket.range(None, None, price_increasing).next() {
            if let Ok((price_key, total_orders)) = item {
                // price is rounded already
                let price = Decimal::raw(u128::from_be_bytes(price_key.try_into().unwrap()));
                return (price, true, total_orders);
            }
        }

        // return default
        (
            match price_increasing {
                OrderBy::Descending => Decimal::MIN, // highest => MIN (so using max will not include)
                OrderBy::Ascending => Decimal::MAX, // lowest => MAX (so using min will not include)
            },
            false,
            0,
        )
    }

    pub fn highest_price(&self, storage: &dyn Storage, pair_key: &[u8]) -> (Decimal, bool, u64) {
        self.best_price(storage, pair_key, OrderBy::Descending)
    }

    pub fn lowest_price(&self, storage: &dyn Storage, pair_key: &[u8]) -> (Decimal, bool, u64) {
        self.best_price(storage, pair_key, OrderBy::Ascending)
    }
}

/// Ticks are stored in Ordered database, so we just need to process at 50 recent ticks is ok
#[cw_serde]
pub struct OrderBook {
    pair_key: Vec<u8>, // an unique pair of assets
    precision: Option<Decimal>,
    buys: Ticks,
    sells: Ticks,
}

impl OrderBook {
    pub fn new(pair_key: &[u8], precision: Option<Decimal>) -> Self {
        OrderBook {
            buys: Ticks::new(OrderDirection::Buy),
            sells: Ticks::new(OrderDirection::Sell),
            pair_key: pair_key.to_vec(),
            precision,
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
                &price.atomics().to_be_bytes(),
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

    pub fn highest_price(
        &self,
        storage: &dyn Storage,
        direction: OrderDirection,
    ) -> (Decimal, bool, u64) {
        match direction {
            OrderDirection::Buy => self.buys.highest_price(storage, &self.pair_key),
            OrderDirection::Sell => self.sells.highest_price(storage, &self.pair_key),
        }
    }

    pub fn lowest_price(
        &self,
        storage: &dyn Storage,
        direction: OrderDirection,
    ) -> (Decimal, bool, u64) {
        match direction {
            OrderDirection::Buy => self.buys.lowest_price(storage, &self.pair_key),
            OrderDirection::Sell => self.sells.lowest_price(storage, &self.pair_key),
        }
    }

    /// find best buy price and best sell price that matched a precision, currently no precision is set
    pub fn find_match_price(&self, storage: &dyn Storage) -> Option<(Decimal, Decimal)> {
        let (highest_buy_price, found, _) = self.highest_price(storage, OrderDirection::Buy);
        if !found {
            return None;
        }

        // if there is precision, find the best sell price closest to best buy price
        if let Some(precision) = self.precision {
            let precision_factor = Decimal::one() + precision;
            let tick_namespaces = &[PREFIX_TICK, &self.pair_key, OrderDirection::Sell.as_bytes()];

            // loop through sell ticks in Order descending, if there is sell tick that satisfies formulation: sell <= highest buy <= sell * (1 + precision)
            if let Some(sell_price) = ReadonlyBucket::<u64>::multilevel(storage, tick_namespaces)
                .range(None, None, OrderBy::Descending)
                .find_map(|item| {
                    if let Ok((price_key, _)) = item {
                        let sell_price =
                            Decimal::raw(u128::from_be_bytes(price_key.try_into().unwrap()));
                        if highest_buy_price.ge(&sell_price)
                            && highest_buy_price.le(&(sell_price * precision_factor))
                        {
                            return Some(sell_price);
                        }
                    }
                    None
                })
            {
                return Some((highest_buy_price, sell_price));
            }
        } else {
            let (lowest_sell_price, found, _) = self.lowest_price(storage, OrderDirection::Sell);
            // there is a match, we will find the best price with precision to prevent market fluctuation
            // we can use precision to convert price to index as well
            if found && highest_buy_price.ge(&lowest_sell_price) {
                return Some((highest_buy_price, lowest_sell_price));
            }
        }
        None
    }

    /// return the largest matchable amount of orders when matching orders at single price
    /// based on best buy price and best sell price, do the filling
    pub fn find_matchable_amount_at_price() {}

    /// matches orders sequentially, starting from buy orders with the highest price, and sell orders with the lowest price
    /// The matching continues until there's no more matchable orders.
    pub fn find_match_orders() {}

    /// distributes the given order amount to the orders
    pub fn distribute_order_amount_to_orders() {}

    /// distributes the given order amount to the orders at the tick price
    pub fn distribute_order_amount_to_tick() {}
}
