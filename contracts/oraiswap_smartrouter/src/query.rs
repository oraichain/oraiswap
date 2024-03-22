use cosmwasm_std::{Deps, StdResult};

use crate::state::{CONFIG, ROUTING_TABLE};
use oraiswap::smartrouter::{GetConfigResponse, GetRouteResponse, GetRoutesResponse};

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
