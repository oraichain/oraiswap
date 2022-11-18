#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Uint128,
};
use oraiswap::error::ContractError;

use crate::order::{
    cancel_order, execute_order, query_last_order_id, query_order, query_orders, submit_order,
};
use crate::state::{
    init_last_order_id, read_config, read_orderbook, store_config, store_orderbook,
};
use crate::tick::{query_tick, query_ticks};

use cw20::Cw20ReceiveMsg;
use oraiswap::asset::{pair_key, Asset, AssetInfo};
use oraiswap::limit_order::{
    ContractInfo, ContractInfoResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg,
    OrderBookResponse, QueryMsg,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:oraiswap_limit_order";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let creator = deps.api.addr_canonicalize(info.sender.as_str())?;
    let config = ContractInfo {
        name: msg.name.unwrap_or(CONTRACT_NAME.to_string()),
        version: msg.version.unwrap_or(CONTRACT_VERSION.to_string()),

        // admin should be multisig
        admin: if let Some(admin) = msg.admin {
            deps.api.addr_canonicalize(admin.as_str())?
        } else {
            creator
        },
    };

    store_config(deps.storage, &config)?;

    init_last_order_id(deps.storage)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, info, msg),
        ExecuteMsg::UpdateAdmin { admin } => execute_update_admin(deps, info, admin),
        ExecuteMsg::UpdateOrderBook {
            offer_info,
            ask_info,
            min_offer_amount,
            min_ask_amount,
        } => execute_update_orderbook(
            deps,
            info,
            offer_info,
            ask_info,
            min_offer_amount,
            min_ask_amount,
        ),
        ExecuteMsg::SubmitOrder {
            direction,
            offer_asset,
            ask_asset,
        } => {
            if !offer_asset.is_native_token() {
                return Err(ContractError::MustProvideNativeToken {});
            }

            offer_asset.assert_sent_native_token_balance(&info)?;
            submit_order(deps, info.sender, direction, offer_asset, ask_asset)
        }
        ExecuteMsg::CancelOrder {
            order_id,
            ask_info,
            offer_info,
        } => cancel_order(deps, info, offer_info, ask_info, order_id),
        ExecuteMsg::ExecuteOrder {
            ask_asset,
            order_id,
            offer_info,
        } => {
            if !ask_asset.is_native_token() {
                return Err(ContractError::MustProvideNativeToken {});
            }

            ask_asset.assert_sent_native_token_balance(&info)?;
            execute_order(deps, offer_info, info.sender, ask_asset, order_id)
        }
    }
}

pub fn execute_update_admin(
    deps: DepsMut,
    info: MessageInfo,
    admin: Addr,
) -> Result<Response, ContractError> {
    let mut contract_info = read_config(deps.storage)?;
    let sender_addr = deps.api.addr_canonicalize(info.sender.as_str())?;

    // check authorized
    if contract_info.admin.ne(&sender_addr) {
        return Err(ContractError::Unauthorized {});
    }

    // update new admin
    contract_info.admin = deps.api.addr_canonicalize(admin.as_str())?;
    store_config(deps.storage, &contract_info)?;

    Ok(Response::new().add_attributes(vec![("action", "execute_update_admin")]))
}

pub fn execute_update_orderbook(
    deps: DepsMut,
    info: MessageInfo,
    ask_info: AssetInfo,
    offer_info: AssetInfo,
    min_ask_amount: Uint128,
    min_offer_amount: Uint128,
) -> Result<Response, ContractError> {
    let contract_info = read_config(deps.storage)?;
    let sender_addr = deps.api.addr_canonicalize(info.sender.as_str())?;

    // check authorized
    if contract_info.admin.ne(&sender_addr) {
        return Err(ContractError::Unauthorized {});
    }

    let pair_key = pair_key(&[offer_info.to_raw(deps.api)?, ask_info.to_raw(deps.api)?]);
    store_orderbook(deps.storage, &pair_key, min_ask_amount, min_offer_amount)?;

    Ok(Response::new().add_attributes(vec![("action", "execute_update_orderbook")]))
}

pub fn receive_cw20(
    deps: DepsMut,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let sender = deps.api.addr_validate(cw20_msg.sender.as_str())?;

    let provided_asset = Asset {
        info: AssetInfo::Token {
            contract_addr: info.sender,
        },
        amount: cw20_msg.amount,
    };

    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::SubmitOrder {
            ask_asset,
            direction,
        }) => submit_order(deps, sender, direction, provided_asset, ask_asset),
        Ok(Cw20HookMsg::ExecuteOrder {
            order_id,
            offer_info,
        }) => execute_order(deps, offer_info, sender, provided_asset, order_id),
        Err(_) => Err(ContractError::InvalidCw20HookMessage {}),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ContractInfo {} => to_binary(&query_contract_info(deps)?),
        QueryMsg::Order {
            order_id,
            offer_info,
            ask_info,
        } => to_binary(&query_order(deps, offer_info, ask_info, order_id)?),
        QueryMsg::OrderBook {
            offer_info,
            ask_info,
        } => to_binary(&query_orderbook(deps, offer_info, ask_info)?),
        QueryMsg::Orders {
            offer_info,
            ask_info,
            direction,
            filter,
            start_after,
            limit,
            order_by,
        } => to_binary(&query_orders(
            deps,
            offer_info,
            ask_info,
            direction,
            filter,
            start_after,
            limit,
            order_by,
        )?),
        QueryMsg::LastOrderId {} => to_binary(&query_last_order_id(deps)?),
        QueryMsg::Tick {
            price,
            offer_info,
            ask_info,
            direction,
        } => to_binary(&query_tick(
            deps.storage,
            &pair_key(&[offer_info.to_raw(deps.api)?, ask_info.to_raw(deps.api)?]),
            direction,
            price,
        )?),
        QueryMsg::Ticks {
            offer_info,
            ask_info,
            direction,
            start_after,
            limit,
            order_by,
        } => to_binary(&query_ticks(
            deps.storage,
            &pair_key(&[offer_info.to_raw(deps.api)?, ask_info.to_raw(deps.api)?]),
            direction,
            start_after,
            limit,
            order_by,
        )?),
    }
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfoResponse> {
    let info = read_config(deps.storage)?;
    Ok(ContractInfoResponse {
        version: info.version,
        name: info.name,
        admin: deps.api.addr_humanize(&info.admin)?,
    })
}

pub fn query_orderbook(
    deps: Deps,
    offer_info: AssetInfo,
    ask_info: AssetInfo,
) -> StdResult<OrderBookResponse> {
    let pair_key = pair_key(&[offer_info.to_raw(deps.api)?, ask_info.to_raw(deps.api)?]);
    let (min_ask_amount, min_offer_amount) = read_orderbook(deps.storage, &pair_key)?;
    Ok(OrderBookResponse {
        offer_info,
        ask_info,
        min_offer_amount,
        min_ask_amount,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
