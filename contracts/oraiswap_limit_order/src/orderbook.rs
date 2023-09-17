use std::convert::TryInto;

use cosmwasm_schema::cw_serde;
use cosmwasm_storage::ReadonlyBucket;
use oraiswap::{
    asset::{pair_key_from_asset_keys, Asset, AssetInfo, AssetInfoRaw},
    limit_order::{OrderBookResponse, OrderDirection, OrderResponse, OrderStatus},
};

use cosmwasm_std::{Api, CanonicalAddr, Decimal, Order as OrderBy, StdResult, Storage, Uint128};

use crate::{
    state::{
        read_orders, read_orders_with_indexer, remove_order, store_order, PREFIX_ORDER_BY_PRICE,
        PREFIX_TICK,
    },
    tick::{query_ticks_prices, query_ticks_prices_with_end},
};

#[cw_serde]
pub struct Order {
    pub order_id: u64,
    pub status: OrderStatus,
    pub direction: OrderDirection, // if direction is sell then offer => sell asset, ask => buy asset
    pub bidder_addr: CanonicalAddr,
    pub offer_amount: Uint128,
    pub ask_amount: Uint128,
    pub filled_offer_amount: Uint128,
    pub filled_ask_amount: Uint128,
}

#[cw_serde]
pub struct Executor {
    pub address: CanonicalAddr,
    pub reward_assets: [Asset; 2],
}

impl Order {
    // create new order given a price and an offer amount
    pub fn new(
        order_id: u64,
        bidder_addr: CanonicalAddr,
        direction: OrderDirection,
        price: Decimal,
        ask_amount: Uint128,
    ) -> Self {
        let offer_amount = match direction {
            OrderDirection::Buy => ask_amount * price,
            OrderDirection::Sell => Uint128::from(ask_amount * Uint128::from(1000000u128))
                .checked_div(price * Uint128::from(1000000u128))
                .unwrap(),
        };

        Order {
            direction,
            order_id,
            bidder_addr,
            offer_amount,
            ask_amount,
            filled_offer_amount: Uint128::zero(),
            filled_ask_amount: Uint128::zero(),
            status: OrderStatus::Open,
        }
    }

    pub fn fill_order(&mut self, ask_amount: Uint128, offer_amount: Uint128) {
        self.filled_ask_amount += ask_amount;
        self.filled_offer_amount += offer_amount;

        if  self.filled_offer_amount == self.offer_amount ||
            self.filled_ask_amount == self.ask_amount {
            self.status = OrderStatus::Fulfilled;
        } else {
            self.status = OrderStatus::PartialFilled;
        }
    }

    pub fn match_order(&mut self, storage: &mut dyn Storage, pair_key: &[u8]) -> StdResult<u64> {
        if self.status == OrderStatus::Fulfilled {
            // When status is Fulfilled, remove order
            remove_order(storage, pair_key, self)
        } else {
            // update order
            store_order(storage, pair_key, self, false)
        }
    }

    // The price will be calculated by the number of base coins divided by the number of quote coins
    pub fn get_price(&self) -> Decimal {
        match self.direction {
            OrderDirection::Buy => Decimal::from_ratio(self.offer_amount, self.ask_amount),
            OrderDirection::Sell => Decimal::from_ratio(self.ask_amount, self.offer_amount),
        }
    }

    pub fn to_response(
        &self,
        api: &dyn Api,
        base_info: AssetInfo,
        quote_info: AssetInfo,
    ) -> StdResult<OrderResponse> {
        Ok(OrderResponse {
            order_id: self.order_id,
            status: self.status,
            direction: self.direction.clone(),
            bidder_addr: api.addr_humanize(&self.bidder_addr)?.to_string(),
            offer_asset: Asset {
                amount: self.offer_amount,
                info: match self.direction {
                    OrderDirection::Buy => quote_info.clone(),
                    OrderDirection::Sell => base_info.clone(),
                },
            },
            ask_asset: Asset {
                amount: self.ask_amount,
                info: match self.direction {
                    OrderDirection::Buy => base_info.clone(),
                    OrderDirection::Sell => quote_info.clone(),
                },
            },
            filled_offer_amount: self.filled_offer_amount,
            filled_ask_amount: self.filled_ask_amount,
        })
    }
}

/// Ticks are stored in Ordered database, so we just need to process at 50 recent ticks is ok
#[cw_serde]
pub struct OrderBook {
    pub base_coin_info: AssetInfoRaw,
    pub quote_coin_info: AssetInfoRaw,
    pub spread: Option<Decimal>,
    pub min_quote_coin_amount: Uint128,
}

impl OrderBook {
    pub fn new(
        base_coin_info: AssetInfoRaw,
        quote_coin_info: AssetInfoRaw,
        spread: Option<Decimal>,
    ) -> Self {
        OrderBook {
            base_coin_info,
            quote_coin_info,
            spread,
            min_quote_coin_amount: Uint128::zero(),
        }
    }

    pub fn to_response(&self, api: &dyn Api) -> StdResult<OrderBookResponse> {
        Ok(OrderBookResponse {
            base_coin_info: self.base_coin_info.to_normal(api)?,
            quote_coin_info: self.quote_coin_info.to_normal(api)?,
            spread: self.spread,
            min_quote_coin_amount: self.min_quote_coin_amount,
        })
    }

    pub fn get_pair_key(&self) -> Vec<u8> {
        pair_key_from_asset_keys(
            self.base_coin_info.as_bytes(),
            self.quote_coin_info.as_bytes(),
        )
    }

    pub fn add_order(&mut self, storage: &mut dyn Storage, order: &Order) -> StdResult<u64> {
        let pair_key = &self.get_pair_key();
        store_order(storage, pair_key, order, true)
    }

    fn best_price(
        &self,
        storage: &dyn Storage,
        direction: OrderDirection,
        price_increasing: OrderBy,
    ) -> (Decimal, bool, u64) {
        let pair_key = &self.get_pair_key();
        // get last tick if price_increasing is true, otherwise get first tick
        let tick_namespaces = &[PREFIX_TICK, pair_key, direction.as_bytes()];
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

    pub fn highest_price(
        &self,
        storage: &dyn Storage,
        direction: OrderDirection,
    ) -> (Decimal, bool, u64) {
        self.best_price(storage, direction, OrderBy::Descending)
    }

    pub fn lowest_price(
        &self,
        storage: &dyn Storage,
        direction: OrderDirection,
    ) -> (Decimal, bool, u64) {
        self.best_price(storage, direction, OrderBy::Ascending)
    }

    pub fn orders_at(
        &self,
        storage: &dyn Storage,
        price: Decimal,
        direction: OrderDirection,
        start_after: Option<u64>,
        limit: Option<u32>,
    ) -> Option<Vec<Order>> {
        let pair_key = &self.get_pair_key();
        read_orders_with_indexer::<OrderDirection>(
            storage,
            &[
                PREFIX_ORDER_BY_PRICE,
                pair_key,
                &price.atomics().to_be_bytes(),
            ],
            Box::new(move |item| direction.eq(item)),
            start_after,
            limit,
            Some(OrderBy::Ascending), // first in first out
        )
        .unwrap()
    }

    // get_orders returns all orders in the order book, with pagination
    pub fn get_orders(
        &self,
        storage: &dyn Storage,
        start_after: Option<u64>,
        limit: Option<u32>,
        order_by: Option<OrderBy>,
    ) -> StdResult<Vec<Order>> {
        let pair_key = &self.get_pair_key();
        read_orders(storage, pair_key, start_after, limit, order_by)
    }

    /// find best buy price and best sell price that matched a spread, currently no spread is set
    pub fn find_match_price(&self, storage: &dyn Storage) -> Option<(Decimal, Decimal)> {
        let pair_key = &self.get_pair_key();
        let (mut best_buy_price, found, _) = self.highest_price(storage, OrderDirection::Buy);
        if !found {
            return None;
        }

        // if there is spread, find the best sell price closest to best buy price
        if let Some(spread) = self.spread {
            let spread_factor = Decimal::one() + spread;
            let buy_price_list = ReadonlyBucket::<u64>::multilevel(
                storage,
                &[PREFIX_TICK, pair_key, OrderDirection::Buy.as_bytes()],
            )
            .range(None, None, OrderBy::Descending)
            .filter_map(|item| {
                if let Ok((price_key, _)) = item {
                    let buy_price =
                        Decimal::raw(u128::from_be_bytes(price_key.try_into().unwrap()));
                    return Some(buy_price);
                }
                None
            })
            .collect::<Vec<Decimal>>();

            let tick_namespaces = &[PREFIX_TICK, pair_key, OrderDirection::Sell.as_bytes()];

            // loop through sell ticks in Order ascending (low to high), if there is sell tick that satisfies formulation: sell <= highest buy <= sell * (1 + spread)
            if let Some(sell_price) = ReadonlyBucket::<u64>::multilevel(storage, tick_namespaces)
                .range(None, None, OrderBy::Ascending)
                .find_map(|item| {
                    if let Ok((price_key, _)) = item {
                        let sell_price =
                            Decimal::raw(u128::from_be_bytes(price_key.try_into().unwrap()));

                        for buy_price in &buy_price_list {
                            if buy_price.ge(&sell_price)
                                && buy_price.le(&(sell_price * spread_factor))
                            {
                                best_buy_price = *buy_price;
                                return Some(sell_price);
                            }
                        }
                    }
                    None
                })
            {
                return Some((best_buy_price, sell_price));
            }
        } else {
            let (lowest_sell_price, found, _) = self.lowest_price(storage, OrderDirection::Sell);
            // there is a match, we will find the best price with spread to prevent market fluctuation
            // we can use spread to convert price to index as well
            if found && best_buy_price.ge(&lowest_sell_price) {
                return Some((best_buy_price, lowest_sell_price));
            }
        }
        None
    }

    /// find list best buy / sell prices
    pub fn find_list_match_price(
        &self,
        storage: &dyn Storage,
        limit: Option<u32>,
    ) -> Option<(Vec<Decimal>, Vec<Decimal>)> {
        let pair_key = &self.get_pair_key();
        // asc
        let sell_price_list = query_ticks_prices(
            storage,
            pair_key,
            OrderDirection::Sell,
            None,
            limit,
            Some(1i32),
        );
        // guard code
        if sell_price_list.len() == 0 {
            return None;
        }

        let mut best_buy_price_list: Vec<Decimal> = Vec::new();
        let mut best_sell_price_list: Vec<Decimal> = Vec::new();

        // if there is spread, find the best list sell price
        if let Some(spread) = self.spread {
            let spread_factor = Decimal::one() + spread;
            for sell_price in sell_price_list {
                let sell_price_with_spread =
                    sell_price.checked_mul(spread_factor).unwrap_or_default();
                if sell_price_with_spread.is_zero() {
                    continue;
                }
                let start_after = if let Some(start_after) = Decimal::from_atomics(
                    sell_price
                        .atomics()
                        .checked_sub(Uint128::from(1u64))
                        .unwrap_or_default(), // sub 1 because we want to get buy price at the smallest sell price as well, not skip it
                    Decimal::DECIMAL_PLACES,
                )
                .ok()
                {
                    Some(start_after)
                } else {
                    None
                };
                let suitable_buy_price_list = query_ticks_prices_with_end(
                    storage,
                    pair_key,
                    OrderDirection::Buy,
                    start_after,
                    Some(sell_price_with_spread),
                    Some(1), // limit 1 because we only need to get the closest buy price possible to the sell price
                    Some(1),
                );

                // cannot find suitable buy price list for the given sell price
                if suitable_buy_price_list.len() == 0 {
                    continue;
                }
                // we loop sell price from smallest to highest, matching buy price must go from highest to lowest => always insert highest into the first element
                best_buy_price_list.insert(0, suitable_buy_price_list[0]);
                best_sell_price_list.push(sell_price);
            }
        } else {
            let start_after = if let Some(start_after) = Decimal::from_atomics(
                sell_price_list[0]
                    .atomics()
                    .checked_sub(Uint128::from(1u64))
                    .unwrap_or_default(), // sub 1 because we want to get buy price at the smallest sell price as well, not skip it
                Decimal::DECIMAL_PLACES,
            )
            .ok()
            {
                Some(start_after)
            } else {
                None
            };
            // desc, all items in this list are ge than the first item in sell list
            best_buy_price_list = query_ticks_prices_with_end(
                storage,
                pair_key,
                OrderDirection::Buy,
                None,
                start_after,
                limit,
                Some(2i32),
            );
            // both price lists are applicable because buy list is always larger than the first item of sell list
            best_sell_price_list = sell_price_list;
        }

        if best_buy_price_list.len() == 0 || best_sell_price_list.len() == 0 {
            return None;
        }
        return Some((best_buy_price_list, best_sell_price_list));
    }

    /// return the largest matchable amount of orders when matching orders at single price, that is total buy volume to sell at that price
    /// based on best buy price and best sell price, do the filling
    pub fn find_match_amount_at_price(
        &self,
        storage: &dyn Storage,
        price: Decimal,
        direction: OrderDirection,
    ) -> Uint128 {
        if let Some(orders) =
            self.query_orders_by_price_and_direction(storage, price, direction, None)
        {
            // in Order, ask amount is alway paid amount
            // in Orderbook, buy order is opposite to sell order
            return orders
                .iter()
                .map(|order| order.ask_amount.u128())
                .sum::<u128>()
                .into();
        }

        Uint128::zero()
    }

    /// matches orders sequentially, starting from buy orders with the highest price, and sell orders with the lowest price
    /// The matching continues until there's no more matchable orders.
    pub fn query_orders_by_price_and_direction(
        &self,
        storage: &dyn Storage,
        price: Decimal,
        direction: OrderDirection,
        limit: Option<u32>,
    ) -> Option<Vec<Order>> {
        let pair_key = &self.get_pair_key();
        let price_key = price.atomics().to_be_bytes();

        // there is a limit, and we just match a batch with maximum orders reach the limit step by step
        read_orders_with_indexer::<OrderDirection>(
            storage,
            &[PREFIX_ORDER_BY_PRICE, pair_key, &price_key],
            Box::new(move |x| direction.eq(x)),
            None,
            limit,
            Some(OrderBy::Ascending), // if mean we process from first to last order in the orderlist
        )
        .unwrap_or_default()
    }
}

impl Executor {
    pub fn new(address: CanonicalAddr, reward_assets: [Asset; 2]) -> Self {
        Executor {
            address,
            reward_assets,
        }
    }
}

pub struct BulkOrders {
    pub orders: Vec<Order>,
    pub direction: OrderDirection,
    pub price: Decimal,
    pub volume: Uint128,
    pub filled_volume: Uint128,
    pub ask_volume: Uint128,
    pub filled_ask_volume: Uint128,
    pub spread_volume: Uint128,
}

impl BulkOrders {
    /// Calculate sum of orders base on direction
    pub fn from_orders(orders: &Vec<Order>, price: Decimal, direction: OrderDirection) -> Self {
        let mut volume = Uint128::zero();
        let mut ask_volume = Uint128::zero();
        let filled_volume = Uint128::zero();
        let filled_ask_volume = Uint128::zero();
        let spread_volume = Uint128::zero();

        for order in orders {
            volume += order.offer_amount.checked_sub(order.filled_offer_amount).unwrap();
            ask_volume += order.ask_amount.checked_sub(order.filled_ask_amount).unwrap();
        }

        return Self {
            direction,
            price,
            orders: orders.clone(),
            volume,
            filled_volume,
            ask_volume,
            filled_ask_volume,
            spread_volume,
        };
    }
}
