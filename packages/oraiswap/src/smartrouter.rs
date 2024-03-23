use crate::{asset::AssetInfo, router::SwapOperation};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

#[cw_serde]
pub enum SmartRouteMode {
    NearestMinimumReceive,
    FurthestMinimumReceive,
}

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String,
    pub router_addr: String,
}

#[cw_serde]
pub struct MigrateMsg {}

// #[cw_serde]
// pub enum Slippage {
//     Twap {
//         window_seconds: Option<u64>,
//         slippage_percentage: Decimal,
//     },
//     MinOutputAmount(Uint128),
// }

#[cw_serde]
pub enum ExecuteMsg {
    UpdateConfig {
        new_owner: Option<String>,
        new_router: Option<String>,
    },
    SetRoute {
        input_info: AssetInfo,
        output_info: AssetInfo,
        pool_route: Vec<SwapOperation>,
    },
    DeleteRoute {
        input_info: AssetInfo,
        output_info: AssetInfo,
        route_index: usize,
    }, // Swap {
       //     input_coin: Coin,
       //     output_denom: String,
       //     slippage: Slippage,
       //     route: Option<Vec<SwapOperation>>,
       // },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetConfigResponse)]
    Config {},
    #[returns(GetRoutesResponse)]
    GetRoutes {
        input_info: AssetInfo,
        output_info: AssetInfo,
    },
    #[returns(GetRouteResponse)]
    GetRoute {
        input_info: AssetInfo,
        output_info: AssetInfo,
        route_index: usize,
    },
    #[returns(GetSmartRouteResponse)]
    GetSmartRoute {
        input_info: AssetInfo,
        output_info: AssetInfo,
        offer_amount: Uint128,
        expected_minimum_receive: Uint128,
        route_mode: Option<SmartRouteMode>,
    },
}

// Response for GetOwner query
#[cw_serde]
pub struct GetConfigResponse {
    pub owner: String,
    pub router: String,
}

// Response for GetRoutes query
#[cw_serde]
pub struct GetRoutesResponse {
    pub pool_routes: Vec<Vec<SwapOperation>>,
}

// Response for GetRoute query

#[cw_serde]
pub struct GetRouteResponse {
    pub pool_route: Vec<SwapOperation>,
}

#[cw_serde]
pub struct GetSmartRouteResponse {
    pub swap_ops: Vec<SwapOperation>,
    pub actual_minimum_receive: Uint128,
}

// Response for Swap
// #[cw_serde]
// pub struct SwapResponse {
//     pub original_sender: String,
//     pub token_out_denom: String,
//     pub amount: Uint128,
// }
