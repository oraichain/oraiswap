use cosmwasm_std::{Deps, StdResult, Uint128};

use crate::state::{CONFIG, ROUTING_TABLE};
use oraiswap::{
    asset::AssetInfo,
    smartrouter::{
        GetConfigResponse, GetRouteResponse, GetRoutesResponse, GetSmartRouteResponse,
        SmartRouteMode,
    },
};

pub fn query_config(deps: Deps) -> StdResult<GetConfigResponse> {
    let state = CONFIG.load(deps.storage)?;

    Ok(GetConfigResponse {
        owner: state.owner.into_string(),
        router: state.router_contract.addr(),
    })
}

pub fn query_route(
    deps: Deps,
    input_token: &str,
    output_token: &str,
    route_index: usize,
) -> StdResult<GetRouteResponse> {
    let routes = ROUTING_TABLE.load(deps.storage, (input_token, output_token))?;
    match routes.get(route_index) {
        Some(route) => Ok(GetRouteResponse {
            pool_route: route.to_owned(),
        }),
        None => Err(cosmwasm_std::StdError::generic_err(
            "Could not find route given the route index",
        )),
    }
}

pub fn query_routes(
    deps: Deps,
    input_token: &str,
    output_token: &str,
) -> StdResult<GetRoutesResponse> {
    Ok(GetRoutesResponse {
        pool_routes: ROUTING_TABLE.load(deps.storage, (input_token, output_token))?,
    })
}

pub fn query_smart_route(
    deps: Deps,
    input_info: AssetInfo,
    output_info: AssetInfo,
    offer_amount: Uint128,
    expected_minimum_receive: Uint128,
    route_mode: Option<SmartRouteMode>,
) -> StdResult<GetSmartRouteResponse> {
    let config = CONFIG.load(deps.storage)?;
    let router = config.router_contract;
    let route_mode = route_mode.unwrap_or(SmartRouteMode::NearestMinimumReceive);
    let pool_routes = ROUTING_TABLE.load(
        deps.storage,
        (&input_info.to_string(), &output_info.to_string()),
    )?;
    let mut route_simulate_result: (usize, Uint128) = (0usize, Uint128::zero());
    for (index, route) in pool_routes.to_owned().into_iter().enumerate() {
        let simulate_result = router.simulate_swap(&deps.querier, offer_amount, route)?;
        let prev_route_simulate_result = route_simulate_result.1;
        match route_mode {
            SmartRouteMode::NearestMinimumReceive => {
                route_simulate_result.1 = route_simulate_result.1.min(expected_minimum_receive.abs_diff(simulate_result.amount));
                if prev_route_simulate_result.ne(&route_simulate_result.1) {
                    route_simulate_result.0 = index;
                }
            },
            SmartRouteMode::FurthestMinimumReceive => {
                route_simulate_result.1 = route_simulate_result.1.max(expected_minimum_receive.abs_diff(simulate_result.amount));
            },
        }
        if prev_route_simulate_result.ne(&route_simulate_result.1) {
            route_simulate_result.0 = index;
        }
    }
    Ok(GetSmartRouteResponse {
        swap_ops: pool_routes[route_simulate_result.0].to_owned(),
        actual_minimum_receive: route_simulate_result.1
    })
}
