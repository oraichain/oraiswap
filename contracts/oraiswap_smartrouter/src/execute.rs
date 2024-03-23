use cosmwasm_std::{DepsMut, MessageInfo, Response};
use oraiswap::router::{RouterController, SwapOperation};

use crate::error::ContractError;
use crate::helpers::{check_is_contract_owner, validate_pool_route};
use crate::state::{CONFIG, ROUTING_TABLE};

pub fn set_route(
    deps: DepsMut,
    info: MessageInfo,
    input_denom: String,
    output_denom: String,
    pool_route: Vec<SwapOperation>,
) -> Result<Response, ContractError> {
    // only owner
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    validate_pool_route(
        deps.as_ref(),
        input_denom.clone(),
        output_denom.clone(),
        pool_route.clone(),
    )?;
    match ROUTING_TABLE.may_load(deps.storage, (&input_denom, &output_denom))? {
        Some(mut routes) => {
            routes.push(pool_route);
            ROUTING_TABLE.save(deps.storage, (&input_denom, &output_denom), &routes)?;
        }
        None => ROUTING_TABLE.save(
            deps.storage,
            (&input_denom, &output_denom),
            &vec![pool_route],
        )?,
    }

    Ok(Response::new().add_attribute("action", "set_route"))
}

pub fn delete_route(
    deps: DepsMut,
    info: MessageInfo,
    input_denom: String,
    output_denom: String,
    route_index: usize,
) -> Result<Response, ContractError> {
    // only owner
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    let mut routes = ROUTING_TABLE.load(deps.storage, (&input_denom, &output_denom))?;
    routes.remove(route_index);

    ROUTING_TABLE.save(deps.storage, (&input_denom, &output_denom), &routes)?;
    Ok(Response::new().add_attribute("action", "delete_route"))
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: Option<String>,
    new_router: Option<String>,
) -> Result<Response, ContractError> {
    // only owner can update config
    check_is_contract_owner(deps.as_ref(), info.sender)?;

    let mut state = CONFIG.load(deps.storage)?;
    if let Some(new_owner) = new_owner {
        let owner_addr = deps.api.addr_validate(&new_owner)?;
        state.owner = owner_addr;
    }
    if let Some(new_router) = new_router {
        state.router_contract = RouterController(new_router);
    }
    CONFIG.save(deps.storage, &state)?;
    Ok(Response::new().add_attribute("action", "update_config"))
}
