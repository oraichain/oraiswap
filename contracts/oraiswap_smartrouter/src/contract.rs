#[cfg(not(feature = "imported"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use oraiswap::router::RouterController;

use crate::error::ContractError;
use crate::execute::{delete_route, set_route, update_config};
use crate::query::{query_config, query_route, query_routes};
use crate::state::{Config, CONFIG};
use oraiswap::smartrouter::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:swaprouter";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Msg Reply IDs
pub const SWAP_REPLY_ID: u64 = 1u64;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // set contract version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // validate owner address and save to state
    let owner = deps.api.addr_validate(&msg.owner)?;
    let router_contract = RouterController(msg.router_addr);
    let state = Config {
        owner,
        router_contract,
    };
    CONFIG.save(deps.storage, &state)?;

    // return OK
    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SetRoute {
            input_info,
            output_info,
            pool_route,
        } => set_route(
            deps,
            info,
            output_info.to_string(),
            input_info.to_string(),
            pool_route,
        ),
        ExecuteMsg::DeleteRoute {
            input_info,
            output_info,
            route_index,
        } => delete_route(
            deps,
            info,
            output_info.to_string(),
            input_info.to_string(),
            route_index,
        ),
        // ExecuteMsg::Swap {
        //     input_coin,
        //     output_denom,
        //     slippage,
        //     route,
        // } => trade_with_slippage_limit(deps, env, info, input_coin, output_denom, slippage, route),
        ExecuteMsg::UpdateState {
            new_owner,
            new_router,
        } => update_config(deps, info, new_owner, new_router),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::GetRoute {
            input_info,
            output_info,
            route_index,
        } => to_binary(&query_route(
            deps,
            &input_info.to_string(),
            &output_info.to_string(),
            route_index,
        )?),
        QueryMsg::GetRoutes {
            input_info,
            output_info,
        } => to_binary(&query_routes(
            deps,
            &input_info.to_string(),
            &output_info.to_string(),
        )?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::new())
}
