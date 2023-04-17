use std::convert::TryInto;

use cosmwasm_schema::cw_serde;
use cosmwasm_storage::ReadonlyBucket;
use oraiswap::{
    asset::{pair_key_from_asset_keys, Asset, AssetInfo, AssetInfoRaw},
    limit_order::{OrderBookResponse, OrderDirection, OrderStatus, OrderResponse},
};

use cosmwasm_std::{
    Api, CanonicalAddr, Decimal, Order as OrderBy, StdResult,
    Storage, Uint128,
};

use crate::state::{
    read_orders, read_orders_with_indexer, remove_order, store_order, PREFIX_ORDER_BY_PRICE,
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
    pub status: OrderStatus,
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
            OrderDirection::Sell => Uint128::from(ask_amount * Uint128::from(1000000u128)).checked_div(price * Uint128::from(1000000u128)).unwrap(),
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

    pub fn fill_order(
        &mut self,
        storage: &mut dyn Storage,
        pair_key: &[u8],
        ask_amount: Uint128,
        offer_amount: Uint128,
    ) -> StdResult<u64> {
        self.filled_ask_amount += ask_amount;
        self.filled_offer_amount += offer_amount;

        if self.filled_offer_amount == self.offer_amount {
            // When match amount equals ask amount, close order
            self.status = OrderStatus::Fulfilled;
            remove_order(storage, pair_key, self)
        } else {
            // update order
            self.status = OrderStatus::PartialFilled;
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
        offer_info: AssetInfo,
        ask_info: AssetInfo,
    ) -> StdResult<OrderResponse> {
        Ok(OrderResponse {
            order_id: self.order_id,
            direction: self.direction.clone(),
            bidder_addr: api.addr_humanize(&self.bidder_addr)?.to_string(),
            offer_asset: Asset {
                amount: self.offer_amount,
                info: match self.direction {
                    OrderDirection::Buy => offer_info.clone(),
                    OrderDirection::Sell => ask_info.clone(),
                },
            },
            ask_asset: Asset {
                amount: self.ask_amount,
                info: match self.direction {
                    OrderDirection::Buy => ask_info.clone(),
                    OrderDirection::Sell => offer_info.clone(),
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
        pair_key_from_asset_keys(self.base_coin_info.as_bytes(), self.quote_coin_info.as_bytes())
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
    ) -> StdResult<Vec<Order>> {
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
        let (highest_buy_price, found, _) = self.highest_price(storage, OrderDirection::Buy);
        if !found {
            return None;
        }

        // if there is spread, find the best sell price closest to best buy price
        if let Some(spread) = self.spread {
            let spread_factor = Decimal::one() + spread;
            let tick_namespaces = &[PREFIX_TICK, pair_key, OrderDirection::Sell.as_bytes()];

            // loop through sell ticks in Order ascending (low to high), if there is sell tick that satisfies formulation: sell <= highest buy <= sell * (1 + spread)
            if let Some(sell_price) = ReadonlyBucket::<u64>::multilevel(storage, tick_namespaces)
                .range(None, None, OrderBy::Ascending)
                .find_map(|item| {
                    if let Ok((price_key, _)) = item {
                        let sell_price =
                            Decimal::raw(u128::from_be_bytes(price_key.try_into().unwrap()));
                        if highest_buy_price.ge(&sell_price)
                            && highest_buy_price.le(&(sell_price * spread_factor))
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
            // there is a match, we will find the best price with spread to prevent market fluctuation
            // we can use spread to convert price to index as well
            if found && highest_buy_price.ge(&lowest_sell_price) {
                return Some((highest_buy_price, lowest_sell_price));
            }
        }
        None
    }

    /// return the largest matchable amount of orders when matching orders at single price, that is total buy volume to sell at that price
    /// based on best buy price and best sell price, do the filling
    pub fn find_match_amount_at_price(
        &self,
        storage: &dyn Storage,
        price: Decimal,
        direction: OrderDirection,
    ) -> Uint128 {
        let orders = self.find_match_orders(storage, price, direction);
        // in Order, ask amount is alway paid amount
        // in Orderbook, buy order is opposite to sell order
        orders
            .iter()
            .map(|order| order.ask_amount.u128())
            .sum::<u128>()
            .into()
    }

    /// matches orders sequentially, starting from buy orders with the highest price, and sell orders with the lowest price
    /// The matching continues until there's no more matchable orders.
    pub fn find_match_orders(
        &self,
        storage: &dyn Storage,
        price: Decimal,
        direction: OrderDirection,
    ) -> Vec<Order> {
        let pair_key = &self.get_pair_key();
        let price_key = price.atomics().to_be_bytes();

        // there is a limit, and we just match a batch with maximum orders reach the limit step by step
        read_orders_with_indexer::<OrderDirection>(
            storage,
            &[PREFIX_ORDER_BY_PRICE, pair_key, &price_key],
            Box::new(move |x| direction.eq(x)),
            None,
            None,
            Some(OrderBy::Ascending), // if mean we process from first to last order in the orderlist
        )
        .unwrap_or_default() // default is empty list
    }
}

impl Executor {
    pub fn new(
        address: CanonicalAddr,
        reward_assets: [Asset; 2],
    ) -> Self {
        Executor {
            address,
            reward_assets,
        }
    }
}