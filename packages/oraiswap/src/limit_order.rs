use crate::asset::{Asset, AssetInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, CanonicalAddr, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;

#[cw_serde]
pub struct ContractInfo {
    pub name: String,
    pub version: String,
    // admin can update the parameter, may be multisig
    pub admin: CanonicalAddr,
}

#[cw_serde]
#[derive(Copy)]
pub enum OrderDirection {
    Buy,
    Sell,
}

impl OrderDirection {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            OrderDirection::Buy => &[0u8],
            OrderDirection::Sell => &[1u8],
        }
    }
}

impl Default for OrderDirection {
    fn default() -> Self {
        OrderDirection::Buy
    }
}
#[cw_serde]
pub struct InstantiateMsg {
    pub name: Option<String>,
    pub version: Option<String>,
    pub admin: Option<Addr>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),

    UpdateAdmin {
        admin: Addr,
    },

    UpdateOrderBook {
        offer_info: AssetInfo,
        ask_info: AssetInfo,
        precision: Option<Decimal>,
        min_offer_amount: Uint128,
    },

    ///////////////////////
    /// User Operations ///
    ///////////////////////
    SubmitOrder {
        direction: OrderDirection, // default is buy, with sell then it is reversed
        offer_asset: Asset,
        ask_asset: Asset,
    },
    CancelOrder {
        order_id: u64,
        offer_info: AssetInfo,
        ask_info: AssetInfo,
    },

    /// Arbitrager execute order to get profit
    ExecuteOrder {
        ask_asset: Asset,
        order_id: u64,
        offer_info: AssetInfo,
    },

    /// Arbitrager execute all orders with pair
    ExecuteAllOrder {
        offer_info: AssetInfo,
        ask_info: AssetInfo,
    },
}

#[cw_serde]
pub enum Cw20HookMsg {
    SubmitOrder {
        ask_asset: Asset,
        direction: OrderDirection,
    },

    /// Arbitrager execute order to get profit
    ExecuteOrder {
        order_id: u64,
        offer_info: AssetInfo,
    },
}

#[cw_serde]
pub enum OrderFilter {
    Bidder(String), // filter by bidder
    Price(Decimal), // filter by price
    Tick,           // filter by direction
    None,           // no filter
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ContractInfoResponse)]
    ContractInfo {},
    #[returns(OrderBookResponse)]
    OrderBook {
        offer_info: AssetInfo,
        ask_info: AssetInfo,
    },
    #[returns(OrderBooksResponse)]
    OrderBooks {
        start_after: Option<Vec<u8>>,
        limit: Option<u32>,
        order_by: Option<i32>, // convert OrderBy to i32
    },
    #[returns(OrderResponse)]
    Order {
        order_id: u64,
        offer_info: AssetInfo,
        ask_info: AssetInfo,
    },
    #[returns(OrdersResponse)]
    Orders {
        offer_info: AssetInfo,
        ask_info: AssetInfo,
        filter: OrderFilter,
        direction: Option<OrderDirection>,
        start_after: Option<u64>,
        limit: Option<u32>,
        order_by: Option<i32>, // convert OrderBy to i32
    },
    #[returns(TickResponse)]
    Tick {
        price: Decimal,
        offer_info: AssetInfo,
        ask_info: AssetInfo,
        direction: OrderDirection,
    },
    #[returns(TicksResponse)]
    Ticks {
        offer_info: AssetInfo,
        ask_info: AssetInfo,
        direction: OrderDirection,
        start_after: Option<Decimal>,
        limit: Option<u32>,
        order_by: Option<i32>, // convert OrderBy to i32
    },
    #[returns(LastOrderIdResponse)]
    LastOrderId {},
}

#[cw_serde]
pub struct ContractInfoResponse {
    pub name: String,
    pub version: String,

    // admin can update the parameter, may be multisig
    pub admin: Addr,
}

#[cw_serde]
pub struct OrderResponse {
    pub order_id: u64,
    pub direction: OrderDirection,
    pub bidder_addr: String,
    pub offer_asset: Asset,
    pub ask_asset: Asset,
    pub filled_offer_amount: Uint128,
    pub filled_ask_amount: Uint128,
}

#[cw_serde]
pub struct OrderBookResponse {
    pub offer_info: AssetInfo,
    pub ask_info: AssetInfo,
    pub min_offer_amount: Uint128,
    pub precision: Option<Decimal>,
}

#[cw_serde]
pub struct OrderBooksResponse {
    pub order_books: Vec<OrderBookResponse>,
}

#[cw_serde]
pub struct OrdersResponse {
    pub orders: Vec<OrderResponse>,
}

#[cw_serde]
pub struct TickResponse {
    pub price: Decimal,
    pub total_orders: u64,
}

#[cw_serde]
pub struct TicksResponse {
    pub ticks: Vec<TickResponse>,
}

#[cw_serde]
pub struct LastOrderIdResponse {
    pub last_order_id: u64,
}

/// We currently take no arguments for migrations
#[cw_serde]
pub struct MigrateMsg {}
