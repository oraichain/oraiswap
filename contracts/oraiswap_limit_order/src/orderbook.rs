use cosmwasm_schema::cw_serde;
use oraiswap::{asset::AssetInfoRaw, limit_order::OrderDirection};

use cosmwasm_std::{CanonicalAddr, Decimal, Uint128};

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

    pub fn get_price(&self) -> Decimal {
        match self.direction {
            OrderDirection::Buy => Decimal::from_ratio(self.ask_amount, self.offer_amount),
            OrderDirection::Sell => Decimal::from_ratio(self.offer_amount, self.ask_amount),
        }
    }
}

#[cw_serde]
pub struct Tick {
    price: Decimal,
    orders: Vec<Order>,
}

impl Tick {
    pub fn new(order: Order) -> Self {
        Tick {
            price: order.get_price(),
            orders: vec![order],
        }
    }

    pub fn add_order(&mut self, order: Order) {
        self.orders.push(order);
    }
}

#[cw_serde]
pub struct Ticks {
    ticks: Vec<Tick>,
    info: AssetInfoRaw,
    price_increasing: bool,
}

impl Ticks {
    pub fn new(info: AssetInfoRaw, price_increasing: bool) -> Self {
        Ticks {
            info,
            ticks: vec![],
            price_increasing,
        }
    }

    pub fn find_price(&self, price: Decimal) -> (usize, bool) {
        let mut i = 0;
        let mut j = self.ticks.len();

        while i < j {
            let h = (i + j) >> 1; // div 2, i ≤ h < j
            let filter_price = if self.price_increasing {
                // sell
                self.ticks[h].price.ge(&price)
            } else {
                // buy
                self.ticks[h].price.le(&price)
            };
            // parition
            if filter_price {
                j = h // preserves left
            } else {
                i = h + 1 // preserves right
            }
        }

        let exact = i < self.ticks.len() && self.ticks[i].price.eq(&price);
        (i, exact)
    }

    pub fn add_order(&mut self, order: Order) {
        let (i, exact) = self.find_price(order.get_price());
        if exact {
            self.ticks[i].add_order(order)
        } else {
            let tick = Tick::new(order);
            if i < self.ticks.len() {
                // Insert a new order book tick at index i.
                self.ticks.insert(i, tick);
            } else {
                self.ticks.push(tick);
            }
        }
    }

    pub fn orders_at(&self, price: Decimal) -> Vec<Order> {
        let (i, exact) = self.find_price(price);

        if !exact {
            return vec![];
        }
        self.ticks[i].orders.clone()
    }

    fn best_price(&self, price_increasing: bool) -> (Decimal, usize, bool) {
        if self.ticks.is_empty() {
            return (Decimal::zero(), 0, false);
        }

        // get from last
        if price_increasing {
            let last_ind = self.ticks.len() - 1;
            return (self.ticks[last_ind].price, last_ind, true);
        }

        (self.ticks[0].price, 0, true)
    }

    pub fn highest_price(&self) -> (Decimal, usize, bool) {
        self.best_price(self.price_increasing)
    }

    pub fn lowest_price(&self) -> (Decimal, usize, bool) {
        self.best_price(!self.price_increasing)
    }
}

#[cw_serde]
pub struct OrderBook {
    buys: Ticks,
    sells: Ticks,
}

impl OrderBook {
    pub fn new(buy_info: AssetInfoRaw, sell_info: AssetInfoRaw, orders: Vec<Order>) -> Self {
        let mut ob = OrderBook {
            buys: Ticks::new(buy_info, false),
            sells: Ticks::new(sell_info, true),
        };

        ob.add_orders(orders);
        ob
    }

    pub fn add_orders(&mut self, orders: Vec<Order>) {
        for order in orders {
            match order.direction {
                OrderDirection::Buy => self.buys.add_order(order),
                OrderDirection::Sell => self.sells.add_order(order),
            }
        }
    }

    // get_orders returns all orders in the order book.
    pub fn get_orders(&self) -> Vec<Order> {
        let mut orders = vec![];
        for tick in self.buys.ticks.iter() {
            orders.extend_from_slice(&tick.orders)
        }

        for tick in self.sells.ticks.iter() {
            orders.extend_from_slice(&tick.orders)
        }

        orders
    }

    pub fn buy_orders_at(&self, price: Decimal) -> Vec<Order> {
        self.buys.orders_at(price)
    }

    pub fn sell_orders_at(&self, price: Decimal) -> Vec<Order> {
        self.sells.orders_at(price)
    }

    pub fn highest_price(&self) -> (Decimal, bool) {
        let (highest_buy_price, _, found_buy) = self.buys.highest_price();
        let (highest_sell_price, _, found_sell) = self.sells.highest_price();
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

    pub fn lowest_price(&self) -> (Decimal, bool) {
        let (lowest_buy_price, _, found_buy) = self.buys.lowest_price();
        let (lowest_sell_price, _, found_sell) = self.sells.lowest_price();
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
