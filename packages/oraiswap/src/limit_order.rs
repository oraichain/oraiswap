use crate::asset::Asset;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),

    ///////////////////////
    /// User Operations ///
    ///////////////////////
    SubmitOrder {
        offer_asset: Asset,
        ask_asset: Asset,
    },
    CancelOrder {
        order_id: u64,
    },

    /// Arbitrager execute order to get profit
    ExecuteOrder {
        execute_asset: Asset,
        order_id: u64,
    },
}

#[cw_serde]
pub enum Cw20HookMsg {
    SubmitOrder {
        ask_asset: Asset,
    },

    /// Arbitrager execute order to get profit
    ExecuteOrder {
        order_id: u64,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(OrderResponse)]
    Order { order_id: u64 },
    #[returns(OrdersResponse)]
    Orders {
        bidder_addr: Option<String>,
        start_after: Option<u64>,
        limit: Option<u32>,
        order_by: Option<i32>, // convert OrderBy to i32
    },
    #[returns(LastOrderIdResponse)]
    LastOrderId {},
}

#[cw_serde]
pub struct OrderResponse {
    pub order_id: u64,
    pub bidder_addr: String,
    pub offer_asset: Asset,
    pub ask_asset: Asset,
    pub filled_offer_amount: Uint128,
    pub filled_ask_amount: Uint128,
}

#[cw_serde]
pub struct OrdersResponse {
    pub orders: Vec<OrderResponse>,
}

#[cw_serde]
pub struct LastOrderIdResponse {
    pub last_order_id: u64,
}

/// We currently take no arguments for migrations
#[cw_serde]
pub struct MigrateMsg {}
