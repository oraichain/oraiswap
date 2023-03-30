#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, Uint128,
};
use oraiswap::error::ContractError;

use crate::order::{
    cancel_order, query_last_order_id, query_order, query_orderbook,
    query_orderbooks, query_orders, submit_order, remove_pair, excecute_pair,
};
use crate::orderbook::OrderBook;
use crate::state::{init_last_order_id, read_config, store_config, store_orderbook, read_orderbook};
use crate::tick::{query_tick, query_ticks};

use cw20::Cw20ReceiveMsg;
use oraiswap::asset::{pair_key, Asset, AssetInfo};
use oraiswap::limit_order::{
    ContractInfo, ContractInfoResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg,
    OrderDirection, QueryMsg,
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
        ExecuteMsg::CreateOrderBookPair {
            base_coin_info,
            quote_coin_info,
            spread,
            min_quote_coin_amount,
        } => execute_create_pair(
            deps,
            info,
            base_coin_info,
            quote_coin_info,
            spread,
            min_quote_coin_amount,
        ),
        ExecuteMsg::SubmitOrder {
            direction,
            assets,
        } => {
            // if sell then paid asset must be ask asset, this way we've just assumed that we offer usdt and ask for orai
            // for execute order, it is direct match(user has known it is buy or sell) so no order is needed
            // Buy: wanting ask asset(orai) => paid offer asset(usdt)
            // Sell: paid ask asset(orai) => wating offer asset(usdt)
            let paid_asset = match direction {
                OrderDirection::Buy => &assets[1],
                OrderDirection::Sell => &assets[0],
            };

            // if paid asset is cw20, we check it in Cw20HookMessage
            if !paid_asset.is_native_token() {
                return Err(ContractError::MustProvideNativeToken {});
            }

            paid_asset.assert_sent_native_token_balance(&info)?;
            // then submit order
            match direction {
                OrderDirection::Buy => submit_order(deps, info.sender, direction, [assets[1].clone(), assets[0].clone()]),
                OrderDirection::Sell => submit_order(deps, info.sender, direction, [assets[0].clone(), assets[1].clone()]),
            }
        }
        ExecuteMsg::CancelOrder {
            order_id,
            asset_infos,
        } => cancel_order(deps, info, order_id, asset_infos),
        ExecuteMsg::ExecuteOrderBookPair {
            asset_infos,
        } => {
            excecute_pair(deps, info, asset_infos)
        }
        ExecuteMsg::RemoveOrderBookPair {
            asset_infos,
        } => {
            remove_pair(deps, info, asset_infos)
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

pub fn execute_create_pair(
    deps: DepsMut,
    info: MessageInfo,
    base_coin_info: AssetInfo,
    quote_coin_info: AssetInfo,
    spread: Option<Decimal>,
    min_quote_coin_amount: Uint128,
) -> Result<Response, ContractError> {
    let contract_info = read_config(deps.storage)?;
    let sender_addr = deps.api.addr_canonicalize(info.sender.as_str())?;

    // check authorized
    if contract_info.admin.ne(&sender_addr) {
        return Err(ContractError::Unauthorized {});
    }

    let pair_key = pair_key(&[base_coin_info.to_raw(deps.api)?, quote_coin_info.to_raw(deps.api)?]);

    let ob = read_orderbook(deps.storage, &pair_key);
    
    // Orderbook already exists
    if ob.is_ok() {
        return Err(ContractError::OrderBookAlreadyExists {});
    }

    let order_book = OrderBook {
        base_coin_info: base_coin_info.to_raw(deps.api)?,
        quote_coin_info: quote_coin_info.to_raw(deps.api)?,
        spread,
        min_quote_coin_amount
    };
    store_orderbook(deps.storage, &pair_key, &order_book)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "create_orderbook_pair"),
        ("pair", &format!("{}/{}", base_coin_info, quote_coin_info)),
        ("spread", &format!("{:.5}", spread.unwrap_or_default())),
        ("min_quote_coin_amount", &min_quote_coin_amount.to_string()),
    ]))
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
            direction,
            assets,
        }) => {
            let paid_asset = match direction {
                OrderDirection::Buy => &assets[1],
                OrderDirection::Sell => &assets[0],
            };

            if paid_asset.amount != provided_asset.amount {
                return Err(ContractError::AssetMismatch {});
            }

            match direction {
                OrderDirection::Buy => submit_order(deps, sender, direction, [assets[1].clone(), assets[0].clone()]),
                OrderDirection::Sell => submit_order(deps, sender, direction, [assets[0].clone(), assets[1].clone()]),
            }
        },
        Err(_) => Err(ContractError::InvalidCw20HookMessage {}),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ContractInfo {} => to_binary(&query_contract_info(deps)?),
        QueryMsg::Order {
            order_id,
            asset_infos,
        } => to_binary(&query_order(deps, asset_infos, order_id)?),
        QueryMsg::OrderBook {
            asset_infos,
        } => to_binary(&query_orderbook(deps, asset_infos)?),
        QueryMsg::OrderBooks {
            start_after,
            limit,
            order_by,
        } => to_binary(&query_orderbooks(deps, start_after, limit, order_by)?),
        QueryMsg::Orders {
            asset_infos,
            direction,
            filter,
            start_after,
            limit,
            order_by,
        } => to_binary(&query_orders(
            deps,
            asset_infos,
            direction,
            filter,
            start_after,
            limit,
            order_by,
        )?),
        QueryMsg::LastOrderId {} => to_binary(&query_last_order_id(deps)?),
        QueryMsg::Tick {
            price,
            asset_infos,
            direction,
        } => to_binary(&query_tick(
            deps.storage,
            &pair_key(&[asset_infos[0].to_raw(deps.api)?, asset_infos[1].to_raw(deps.api)?]),
            direction,
            price,
        )?),
        QueryMsg::Ticks {
            asset_infos,
            direction,
            start_after,
            limit,
            order_by,
        } => to_binary(&query_ticks(
            deps.storage,
            &pair_key(&[asset_infos[0].to_raw(deps.api)?, asset_infos[1].to_raw(deps.api)?]),
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
