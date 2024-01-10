#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, Uint128,
};
use oraiswap::error::ContractError;

use crate::order::{
    cancel_order, execute_matching_orders, get_paid_and_quote_assets, query_last_order_id,
    query_order, query_orderbook, query_orderbook_is_matchable, query_orderbooks, query_orders,
    remove_pair, submit_market_order, submit_order,
};
use crate::orderbook::OrderBook;
use crate::state::{
    init_last_order_id, read_config, read_orderbook, store_config, store_orderbook, validate_admin,
};
use crate::tick::{query_tick, query_ticks_with_end};

use cw20::Cw20ReceiveMsg;
use oraiswap::asset::{pair_key, Asset, AssetInfo};
use oraiswap::limit_order::{
    ContractInfo, ContractInfoResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg,
    OrderDirection, QueryMsg,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:oraiswap_limit_order";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// default commission rate = 0.1 %
const DEFAULT_COMMISSION_RATE: &str = "0.001";
const REWARD_WALLET: &str = "orai16stq6f4pnrfpz75n9ujv6qg3czcfa4qyjux5en";
const DEFAULT_MARKET_ORDER_SLIPPAGE: &str = "0.01";
const MAX_MARKET_ORDER_SLIPPAGE: &str = "0.1";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let creator = deps.api.addr_canonicalize(info.sender.as_str())?;
    let default_reward_address = deps.api.addr_canonicalize(REWARD_WALLET)?;
    let config = ContractInfo {
        name: msg.name.unwrap_or(CONTRACT_NAME.to_string()),
        version: msg.version.unwrap_or(CONTRACT_VERSION.to_string()),

        // admin should be multisig
        admin: if let Some(admin) = msg.admin {
            deps.api.addr_canonicalize(admin.as_str())?
        } else {
            creator
        },
        commission_rate: msg
            .commission_rate
            .unwrap_or(DEFAULT_COMMISSION_RATE.to_string()),
        reward_address: if let Some(reward_address) = msg.reward_address {
            deps.api.addr_canonicalize(reward_address.as_str())?
        } else {
            default_reward_address
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
        ExecuteMsg::UpdateConfig {
            reward_address,
            commission_rate,
        } => execute_update_config(deps, info, reward_address, commission_rate),
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
        ExecuteMsg::UpdateOrderbookPair {
            asset_infos,
            spread,
        } => {
            validate_admin(
                deps.api,
                read_config(deps.storage)?.admin,
                info.sender.as_str(),
            )?;
            let pair_key = pair_key(&[
                asset_infos[0].to_raw(deps.api)?,
                asset_infos[1].to_raw(deps.api)?,
            ]);
            let mut orderbook_pair = read_orderbook(deps.storage, &pair_key)?;
            orderbook_pair.spread = spread;
            store_orderbook(deps.storage, &pair_key, &orderbook_pair)?;
            Ok(Response::new().add_attributes(vec![("action", "update_orderbook_data")]))
        }
        ExecuteMsg::SubmitOrder { direction, assets } => {
            let pair_key = pair_key(&[
                assets[0].to_raw(deps.api)?.info,
                assets[1].to_raw(deps.api)?.info,
            ]);
            let orderbook_pair = read_orderbook(deps.storage, &pair_key)?;

            // if sell then paid asset must be ask asset, this way we've just assumed that we offer usdt and ask for orai
            // for execute order, it is direct match(user has known it is buy or sell) so no order is needed
            // Buy: wanting ask asset(orai) => paid offer asset(usdt)
            // Sell: paid ask asset(orai) => wating offer asset(usdt)
            let (paid_assets, quote_asset) =
                get_paid_and_quote_assets(deps.api, &orderbook_pair, assets, direction)?;

            paid_assets[0].assert_if_asset_is_native_token()?;
            paid_assets[0].assert_sent_native_token_balance(&info)?;

            // require minimum amount for quote asset
            if quote_asset.amount.lt(&orderbook_pair.min_quote_coin_amount) {
                return Err(ContractError::TooSmallQuoteAsset {
                    quote_coin: quote_asset.info.to_string(),
                    min_quote_amount: orderbook_pair.min_quote_coin_amount,
                });
            }

            // then submit order
            submit_order(deps, &orderbook_pair, info.sender, direction, paid_assets)
        }
        ExecuteMsg::SubmitMarketOrder {
            direction,
            assets,
            offer_asset_index,
            slippage,
        } => {
            let pair_key = pair_key(&[
                assets[0].to_raw(deps.api)?.info,
                assets[1].to_raw(deps.api)?.info,
            ]);
            let orderbook_pair = read_orderbook(deps.storage, &pair_key)?;
            let offer_asset = if offer_asset_index == 0 {
                assets[0].clone()
            } else {
                assets[1].clone()
            };

            offer_asset.assert_if_asset_is_native_token()?;
            offer_asset.assert_sent_native_token_balance(&info)?;

            // require minimum amount for quote asset
            if orderbook_pair
                .quote_coin_info
                .to_normal(deps.api)?
                .eq(&offer_asset.info)
                && offer_asset.amount.lt(&orderbook_pair.min_quote_coin_amount)
            {
                return Err(ContractError::TooSmallQuoteAsset {
                    quote_coin: offer_asset.info.to_string(),
                    min_quote_amount: orderbook_pair.min_quote_coin_amount,
                });
            }
            // submit market order
            submit_market_order(
                deps,
                &orderbook_pair,
                info.sender,
                direction,
                offer_asset,
                slippage,
            )
        }
        ExecuteMsg::CancelOrder {
            order_id,
            asset_infos,
        } => cancel_order(deps, info, order_id, asset_infos),
        ExecuteMsg::ExecuteOrderBookPair { asset_infos, limit } => {
            execute_matching_orders(deps, info, asset_infos, limit)
        }
        ExecuteMsg::RemoveOrderBookPair { asset_infos } => remove_pair(deps, info, asset_infos),
    }
}

pub fn execute_update_admin(
    deps: DepsMut,
    info: MessageInfo,
    admin: Addr,
) -> Result<Response, ContractError> {
    let mut contract_info = read_config(deps.storage)?;
    validate_admin(deps.api, contract_info.admin, info.sender.as_str())?;

    // update new admin
    contract_info.admin = deps.api.addr_canonicalize(admin.as_str())?;
    store_config(deps.storage, &contract_info)?;

    Ok(Response::new().add_attributes(vec![("action", "execute_update_admin")]))
}

pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    reward_address: Option<Addr>,
    commission_rate: Option<String>,
) -> Result<Response, ContractError> {
    let mut contract_info = read_config(deps.storage)?;
    let sender_addr = deps.api.addr_canonicalize(info.sender.as_str())?;

    // check authorized
    if contract_info.admin.ne(&sender_addr) {
        return Err(ContractError::Unauthorized {});
    }

    // update new reward address
    if let Some(reward_address) = reward_address {
        contract_info.reward_address = deps.api.addr_canonicalize(reward_address.as_str())?;
    }

    // update new commission rate
    if let Some(commission_rate) = commission_rate {
        contract_info.commission_rate = commission_rate;
    }

    store_config(deps.storage, &contract_info)?;
    Ok(Response::new().add_attributes(vec![("action", "execute_update_config")]))
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

    let pair_key = pair_key(&[
        base_coin_info.to_raw(deps.api)?,
        quote_coin_info.to_raw(deps.api)?,
    ]);

    let ob = read_orderbook(deps.storage, &pair_key);

    // Orderbook already exists
    if ob.is_ok() {
        return Err(ContractError::OrderBookAlreadyExists {});
    }

    let order_book = OrderBook {
        base_coin_info: base_coin_info.to_raw(deps.api)?,
        quote_coin_info: quote_coin_info.to_raw(deps.api)?,
        spread,
        min_quote_coin_amount,
    };
    store_orderbook(deps.storage, &pair_key, &order_book)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "create_orderbook_pair"),
        ("pair", &format!("{} - {}", base_coin_info, quote_coin_info)),
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
        Ok(Cw20HookMsg::SubmitOrder { direction, assets }) => {
            let pair_key = pair_key(&[
                assets[0].to_raw(deps.api)?.info,
                assets[1].to_raw(deps.api)?.info,
            ]);
            let orderbook_pair = read_orderbook(deps.storage, &pair_key)?;
            let (paid_assets, quote_asset) =
                get_paid_and_quote_assets(deps.api, &orderbook_pair, assets, direction)?;

            if paid_assets[0].amount != provided_asset.amount {
                return Err(ContractError::AssetMismatch {});
            }

            // require minimum amount for quote asset
            if quote_asset.amount.lt(&orderbook_pair.min_quote_coin_amount) {
                return Err(ContractError::TooSmallQuoteAsset {
                    quote_coin: quote_asset.info.to_string(),
                    min_quote_amount: orderbook_pair.min_quote_coin_amount,
                });
            }

            // then submit order
            submit_order(deps, &orderbook_pair, sender, direction, paid_assets)
        }
        Ok(Cw20HookMsg::SubmitMarketOrder {
            direction,
            assets,
            offer_asset_index,
            slippage,
        }) => {
            let pair_key = pair_key(&[
                assets[0].to_raw(deps.api)?.info,
                assets[1].to_raw(deps.api)?.info,
            ]);
            let orderbook_pair = read_orderbook(deps.storage, &pair_key)?;
            let offer_asset = if offer_asset_index == 0 {
                assets[0].clone()
            } else {
                assets[1].clone()
            };

            if offer_asset.amount != provided_asset.amount {
                return Err(ContractError::AssetMismatch {});
            }

            // require minimum amount for quote asset
            if orderbook_pair
                .quote_coin_info
                .to_normal(deps.api)?
                .eq(&offer_asset.info)
                && offer_asset.amount.lt(&orderbook_pair.min_quote_coin_amount)
            {
                return Err(ContractError::TooSmallQuoteAsset {
                    quote_coin: offer_asset.info.to_string(),
                    min_quote_amount: orderbook_pair.min_quote_coin_amount,
                });
            }
            // submit market order
            submit_market_order(
                deps,
                &orderbook_pair,
                sender,
                direction,
                offer_asset,
                slippage,
            )
        }
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
        QueryMsg::OrderBook { asset_infos } => to_binary(&query_orderbook(deps, asset_infos)?),
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
            &pair_key(&[
                asset_infos[0].to_raw(deps.api)?,
                asset_infos[1].to_raw(deps.api)?,
            ]),
            direction,
            price,
        )?),
        QueryMsg::Ticks {
            asset_infos,
            direction,
            start_after,
            end,
            limit,
            order_by,
        } => to_binary(&query_ticks_with_end(
            deps.storage,
            &pair_key(&[
                asset_infos[0].to_raw(deps.api)?,
                asset_infos[1].to_raw(deps.api)?,
            ]),
            direction,
            start_after,
            end,
            limit,
            order_by,
        )?),
        QueryMsg::OrderBookMatchable { asset_infos } => {
            to_binary(&query_orderbook_is_matchable(deps, asset_infos)?)
        }
        QueryMsg::MidPrice { asset_infos } => {
            let pair_key = pair_key(&[
                asset_infos[0].to_raw(deps.api)?,
                asset_infos[1].to_raw(deps.api)?,
            ]);
            let orderbook_pair = read_orderbook(deps.storage, &pair_key)?;
            let (highest_buy_price, buy_found, _) =
                orderbook_pair.highest_price(deps.storage, OrderDirection::Buy);
            let (lowest_sell_price, sell_found, _) =
                orderbook_pair.lowest_price(deps.storage, OrderDirection::Sell);
            let best_buy_price = if buy_found {
                highest_buy_price
            } else {
                Decimal::zero()
            };
            let best_sell_price = if sell_found {
                lowest_sell_price
            } else {
                Decimal::zero()
            };
            let mid_price = best_buy_price
                .checked_add(best_sell_price)
                .unwrap_or_default()
                .checked_div(Decimal::from_ratio(2u128, 1u128))
                .unwrap_or_default();
            to_binary(&mid_price)
        }
    }
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfoResponse> {
    let info = read_config(deps.storage)?;
    Ok(ContractInfoResponse {
        version: info.version,
        name: info.name,
        admin: deps.api.addr_humanize(&info.admin)?,
        commission_rate: info.commission_rate,
        reward_address: deps.api.addr_humanize(&info.reward_address)?,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
