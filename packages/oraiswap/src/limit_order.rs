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
    pub commission_rate: String,
    pub reward_address: CanonicalAddr,
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

#[cw_serde]
#[derive(Copy)]
pub enum OrderStatus {
    Open,
    PartialFilled,
    Fulfilled,
    Cancel,
}

impl OrderStatus {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            OrderStatus::Open => &[0u8],
            OrderStatus::PartialFilled => &[1u8],
            OrderStatus::Fulfilled => &[2u8],
            OrderStatus::Cancel => &[3u8],
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
    pub commission_rate: Option<String>,
    pub reward_address: Option<Addr>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),

    UpdateAdmin {
        admin: Addr,
    },

    UpdateConfig {
        reward_address: Option<Addr>,
        commission_rate: Option<String>,
    },

    CreateOrderBookPair {
        base_coin_info: AssetInfo,
        quote_coin_info: AssetInfo,
        spread: Option<Decimal>,
        min_quote_coin_amount: Uint128,
    },

    UpdateOrderbookPair {
        asset_infos: [AssetInfo; 2],
        spread: Option<Decimal>,
    },

    ///////////////////////
    /// User Operations ///
    ///////////////////////
    SubmitOrder {
        direction: OrderDirection, // default is buy, with sell then it is reversed
        assets: [Asset; 2],
    },

    CancelOrder {
        order_id: u64,
        asset_infos: [AssetInfo; 2],
    },

    /// Arbitrager execute order book pair
    ExecuteOrderBookPair {
        asset_infos: [AssetInfo; 2],
        limit: Option<u32>,
    },

    /// Arbitrager remove order book
    RemoveOrderBookPair {
        asset_infos: [AssetInfo; 2],
    },
}

#[cw_serde]
pub enum Cw20HookMsg {
    SubmitOrder {
        direction: OrderDirection,
        assets: [Asset; 2],
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
    OrderBook { asset_infos: [AssetInfo; 2] },
    #[returns(OrderBooksResponse)]
    OrderBooks {
        start_after: Option<Vec<u8>>,
        limit: Option<u32>,
        order_by: Option<i32>, // convert OrderBy to i32
    },
    #[returns(OrderResponse)]
    Order {
        order_id: u64,
        asset_infos: [AssetInfo; 2],
    },
    #[returns(OrdersResponse)]
    Orders {
        asset_infos: [AssetInfo; 2],
        filter: OrderFilter,
        direction: Option<OrderDirection>,
        start_after: Option<u64>,
        limit: Option<u32>,
        order_by: Option<i32>, // convert OrderBy to i32
    },
    #[returns(TickResponse)]
    Tick {
        price: Decimal,
        asset_infos: [AssetInfo; 2],
        direction: OrderDirection,
    },
    #[returns(TicksResponse)]
    Ticks {
        asset_infos: [AssetInfo; 2],
        direction: OrderDirection,
        start_after: Option<Decimal>,
        end: Option<Decimal>,
        limit: Option<u32>,
        order_by: Option<i32>, // convert OrderBy to i32
    },
    #[returns(LastOrderIdResponse)]
    LastOrderId {},
    #[returns(OrderBookMatchableResponse)]
    OrderBookMatchable { asset_infos: [AssetInfo; 2] },
    #[returns(Decimal)]
    MidPrice { asset_infos: [AssetInfo; 2] },
}

#[cw_serde]
pub struct ContractInfoResponse {
    pub name: String,
    pub version: String,

    // admin can update the parameter, may be multisig
    pub admin: Addr,
    pub commission_rate: String,
    pub reward_address: Addr,
}

#[cw_serde]
pub struct OrderResponse {
    pub order_id: u64,
    pub status: OrderStatus,
    pub direction: OrderDirection,
    pub bidder_addr: String,
    pub offer_asset: Asset,
    pub ask_asset: Asset,
    pub filled_offer_amount: Uint128,
    pub filled_ask_amount: Uint128,
}

#[cw_serde]
pub struct OrderBookResponse {
    pub base_coin_info: AssetInfo,
    pub quote_coin_info: AssetInfo,
    pub spread: Option<Decimal>,
    pub min_quote_coin_amount: Uint128,
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

#[cw_serde]
pub struct OrderBookMatchableResponse {
    pub is_matchable: bool,
}

/// We currently take no arguments for migrations
#[cw_serde]
pub struct MigrateMsg {}
