use std::convert::TryFrom;

use crate::orderbook::Order;
use crate::state::{
    increase_last_order_id, read_last_order_id, read_order, read_orderbook, read_orderbooks,
    read_orders, read_orders_with_indexer, remove_order, store_order, PREFIX_ORDER_BY_BIDDER,
    PREFIX_ORDER_BY_PRICE, PREFIX_TICK, read_config, remove_orderbook,
};
use cosmwasm_std::{
    Addr, CosmosMsg, Deps, DepsMut, MessageInfo, Order as OrderBy, Response, StdResult, Uint128,
};

use oraiswap::math::Converter128;
use oraiswap::asset::{pair_key, Asset, AssetInfo};
use oraiswap::error::ContractError;
use oraiswap::limit_order::{
    LastOrderIdResponse, OrderBookResponse, OrderBooksResponse, OrderDirection, OrderFilter,
    OrderResponse, OrdersResponse, OrderStatus,
};

pub fn submit_order(
    deps: DepsMut,
    sender: Addr,
    direction: OrderDirection,
    assets: [Asset; 2],
) -> Result<Response, ContractError> {
    if assets[0].amount.is_zero() || assets[1].amount.is_zero() {
        return Err(ContractError::AssetMustNotBeZero {})
    }
    
    // check min base coin amount
    // need to setup min base coin amount for a specific pair so that no one can spam
    let pair_key = pair_key(&[assets[0].to_raw(deps.api)?.info, assets[1].to_raw(deps.api)?.info]);
    let order_book = read_orderbook(deps.storage, &pair_key)?;
    let base_asset = match direction {
        OrderDirection::Buy => assets[0].clone(),
        OrderDirection::Sell => assets[1].clone(),
    };

    // require minimum amount for base asset
    if base_asset.amount.lt(&order_book.min_base_coin_amount) {
        return Err(ContractError::TooSmallBaseCoinAmount {
            base_coin: base_asset.info.to_string(),
            min_base_amount: order_book.min_base_coin_amount,
        });
    }

    let order_id = increase_last_order_id(deps.storage)?;

    store_order(
        deps.storage,
        &pair_key,
        &Order {
            order_id,
            direction,
            bidder_addr: deps.api.addr_canonicalize(sender.as_str())?,
            offer_amount: assets[0].to_raw(deps.api)?.amount,
            ask_amount: assets[1].to_raw(deps.api)?.amount,
            filled_offer_amount: Uint128::zero(),
            filled_ask_amount: Uint128::zero(),
            status: OrderStatus::Open,
        },
        true,
    )?;

    Ok(Response::new().add_attributes(vec![
        ("action", "submit_order"),
        ("order_id", &order_id.to_string()),
        ("direction", &format!("{:?}", direction)),
        ("bidder_addr", sender.as_str()),
        ("offer_asset", &format!("{} {}", &assets[0].amount, &assets[0].info)),
        ("ask_asset", &format!("{} {}", &assets[1].amount, &assets[1].info)),
    ]))
}

pub fn update_order(
    deps: DepsMut,
    info: MessageInfo,
    order_id: u64,
    assets: [Asset; 2],
) -> Result<Response, ContractError> {
    if assets[0].amount.is_zero() || assets[1].amount.is_zero() {
        return Err(ContractError::AssetMustNotBeZero {})
    }
    
    // check min base coin amount
    // need to setup min base coin amount for a specific pair so that no one can spam
    let pair_key = pair_key(&[assets[0].to_raw(deps.api)?.info, assets[1].to_raw(deps.api)?.info]);
    let order_book = read_orderbook(deps.storage, &pair_key)?;
    let order = read_order(deps.storage, &pair_key, order_id)?;

    if order.status == OrderStatus::Fulfilled {
        return Err(ContractError::OrderFulfilled {
            order_id: order.order_id,
        });
    }

    if order.status == OrderStatus::Filling {
        return Err(ContractError::OrderIsFilling {
            order_id: order.order_id,
        });
    }

    let offer_asset = match order.direction {
        OrderDirection::Buy => &assets[0],
        OrderDirection::Sell => &assets[1],
    };

    let ask_asset = match order.direction {
        OrderDirection::Buy => &assets[1],
        OrderDirection::Sell => &assets[0],
    };

    // if paid asset is cw20, we check it in Cw20HookMessage
    if !offer_asset.is_native_token() {
        return Err(ContractError::MustProvideNativeToken {});
    }

    offer_asset.assert_sent_native_token_balance(&info)?;

    // require minimum amount for base asset
    if assets[0].amount.lt(&order_book.min_base_coin_amount) {
        return Err(ContractError::TooSmallBaseCoinAmount {
            base_coin: assets[0].info.to_string(),
            min_base_amount: order_book.min_base_coin_amount,
        });
    }

    store_order(
        deps.storage,
        &pair_key,
        &Order {
            order_id,
            direction: order.direction,
            bidder_addr: deps.api.addr_canonicalize(info.sender.as_str())?,
            offer_amount: offer_asset.to_raw(deps.api)?.amount,
            ask_amount: ask_asset.to_raw(deps.api)?.amount,
            filled_offer_amount: Uint128::zero(),
            filled_ask_amount: Uint128::zero(),
            status: order.status,
        },
        false,
    )?;

    Ok(Response::new().add_attributes(vec![
        ("action", "update_order"),
        ("order_id", &order_id.to_string()),
        ("direction", &format!("{:?}", order.direction)),
        ("bidder_addr", info.sender.as_str()),
        ("offer_asset", &format!("{} {}", &offer_asset.amount, &offer_asset.info)),
        ("ask_asset", &format!("{} {}", &ask_asset.amount, &ask_asset.info)),
    ]))
}

pub fn cancel_order(
    deps: DepsMut,
    info: MessageInfo,
    order_id: u64,
    asset_infos: [AssetInfo; 2],
) -> Result<Response, ContractError> {
    let pair_key = pair_key(&[asset_infos[0].to_raw(deps.api)?, asset_infos[1].to_raw(deps.api)?]);
    let mut order = read_order(deps.storage, &pair_key, order_id)?;

    if order.status == OrderStatus::Fulfilled {
        return Err(ContractError::OrderFulfilled {
            order_id: order.order_id,
        });
    }

    if order.status == OrderStatus::Filling {
        return Err(ContractError::OrderIsFilling {
            order_id: order.order_id,
        });
    }

    if order.bidder_addr != deps.api.addr_canonicalize(info.sender.as_str())? {
        return Err(ContractError::Unauthorized {});
    }

    // Compute refund asset
    let left_offer_amount = order.offer_amount.checked_sub(order.filled_offer_amount)?;

    let bidder_refund = Asset {
        info: match order.direction {
            OrderDirection::Buy => asset_infos[0].clone(),
            OrderDirection::Sell => asset_infos[1].clone(),
        },
        amount: left_offer_amount,
    };

    // Build refund msg
    let messages = if left_offer_amount > Uint128::zero() {
        vec![bidder_refund
            .clone()
            .into_msg(None, &deps.querier, info.sender)?]
    } else {
        vec![]
    };
    order.status = OrderStatus::Cancel;
    remove_order(deps.storage, &pair_key, &order).unwrap();

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("action", "cancel_order"),
        ("order_id", &order_id.to_string()),
        ("bidder_refund", &bidder_refund.to_string()),
    ]))
}

pub fn execute_order(
    deps: DepsMut,
    offer_info: AssetInfo,
    sender: Addr,
    ask_asset: Asset,
    order_id: u64,
) -> Result<Response, ContractError> {
    let pair_key = pair_key(&[
        offer_info.to_raw(deps.api)?,
        ask_asset.info.to_raw(deps.api)?,
    ]);
    let mut order = read_order(deps.storage, &pair_key, order_id)?;

    // Compute offer amount & match ask amount
    let (offer_amount, match_ask_amount) = order.matchable_amount(ask_asset.amount)?;
    let executor_receive = Asset {
        info: offer_info,
        amount: offer_amount,
    };

    let bidder_addr = deps.api.addr_humanize(&order.bidder_addr)?;

    // When match amount equals ask amount, close order
    if match_ask_amount == ask_asset.amount {
        order.status = OrderStatus::Fulfilled;
        remove_order(deps.storage, &pair_key, &order).unwrap();
    } else {
        order.filled_ask_amount += ask_asset.amount;
        order.filled_offer_amount += executor_receive.amount;
        // update order
        store_order(deps.storage, &pair_key, &order, false)?;
    };

    let mut messages: Vec<CosmosMsg> = vec![];
    if !executor_receive.amount.is_zero() {
        // dont use oracle for limit order
        messages.push(executor_receive.clone().into_msg(
            None,
            &deps.querier,
            deps.api.addr_validate(sender.as_str())?,
        )?);
    }

    if !ask_asset.amount.is_zero() {
        messages.push(ask_asset.into_msg(None, &deps.querier, bidder_addr)?);
    }

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("action", "execute_order"),
        ("order_id", &order_id.to_string()),
        ("executor_receive", &executor_receive.to_string()),
        ("bidder_receive", &ask_asset.to_string()),
    ]))
}

pub fn excecute_pair(
    deps: DepsMut,
    info: MessageInfo,
    asset_infos: [AssetInfo; 2],
) -> Result<Response, ContractError> {
    let contract_info = read_config(deps.storage)?;
    let sender_addr = deps.api.addr_canonicalize(info.sender.as_str())?;

    if contract_info.admin.ne(&sender_addr) {
        return Err(ContractError::Unauthorized {});
    }

    let pair_key = pair_key(&[asset_infos[0].to_raw(deps.api)?, asset_infos[1].to_raw(deps.api)?]);
    let ob = read_orderbook(deps.storage, &pair_key)?;

    let (best_buy_price, best_sell_price) = ob.find_match_price(deps.as_ref().storage).unwrap();

    let mut match_one_price = false;
    if best_buy_price.eq(&best_sell_price) {
        match_one_price = true;
    }

    let mut match_buy_orders = ob.find_match_orders(deps.as_ref().storage, best_buy_price, OrderDirection::Buy);
    let mut match_sell_orders = ob.find_match_orders(deps.as_ref().storage, best_sell_price, OrderDirection::Sell);

    let mut messages: Vec<CosmosMsg> = vec![];
    let mut total_orders =  0;

    for buy_order in &mut match_buy_orders {
        // check status of buy_order
        if buy_order.status == OrderStatus::Fulfilled {
            continue;
        }

        buy_order.status = OrderStatus::Filling;
        let bidder_addr = deps.api.addr_humanize(&buy_order.bidder_addr)?;
        let mut match_price = buy_order.get_price();
        let mut price_direction: OrderDirection = OrderDirection::Buy;

        let mut executor_receive_amount = Uint128::zero();

        let mut lef_buy_ask_amount = buy_order.ask_amount - buy_order.filled_ask_amount;
        let mut lef_buy_offer_amount = buy_order.offer_amount - buy_order.filled_offer_amount;
 
        for sell_order in &mut match_sell_orders {
            // check status of sell_order
            if sell_order.status == OrderStatus::Fulfilled {
                continue;
            }

            sell_order.status = OrderStatus::Filling;
            let mut lef_sell_ask_amount = sell_order.ask_amount - sell_order.filled_ask_amount;
            let lef_sell_offer_amount = sell_order.offer_amount - sell_order.filled_offer_amount;

            if match_one_price == false {
                if sell_order.order_id < buy_order.order_id {
                    match_price = sell_order.get_price();
                    price_direction = OrderDirection::Sell;
                    lef_buy_ask_amount = Uint128::from(lef_buy_offer_amount * match_price);
                } else {
                    match_price = buy_order.get_price();
                    price_direction = OrderDirection::Buy;
                    lef_sell_ask_amount = Uint128::from(lef_sell_offer_amount).checked_div_decimal(match_price).unwrap();
                }
            }

            // offer amount is already paid, we need ask amount to be received
            // remember that ask of buy and ask of sell are opposite sides
            // sell_ask_amount is equal match ask amount, to make sure always matched
            let sell_ask_amount = Uint128::min(
                lef_buy_offer_amount,
                lef_sell_ask_amount,
            );

            if sell_ask_amount.is_zero() {
                sell_order.status = OrderStatus::Fulfilled;
                remove_order(deps.storage, &pair_key, sell_order).unwrap();
                continue;
            }

            let mut sell_ask_asset = Asset {
                info: asset_infos[0].clone(),
                amount: sell_ask_amount,
            };
            
            let (_, mut sell_amount) = sell_order.matchable_amount(sell_ask_asset.amount)?;

            sell_amount = match price_direction {
                OrderDirection::Buy => sell_ask_asset.amount,
                OrderDirection::Sell => sell_amount,
            };

            sell_amount = Uint128::min(
                lef_buy_offer_amount,
                sell_amount,
            );

            sell_ask_asset.amount = sell_amount;
            lef_buy_offer_amount -= sell_ask_asset.amount;

            let buy_offer_amount = match_price * sell_ask_asset.amount;

            executor_receive_amount += buy_offer_amount;

            let asker_addr = deps.api.addr_humanize(&sell_order.bidder_addr)?;

            // fill this order
            sell_order.fill_order(deps.storage, &pair_key, sell_amount, buy_offer_amount).unwrap();

            if buy_offer_amount.is_zero() && sell_amount.is_zero() {
                sell_order.status = OrderStatus::Fulfilled;
            }
            
            if !sell_ask_asset.amount.is_zero() {
                messages.push(sell_ask_asset.into_msg(None, &deps.querier, asker_addr)?);
            }

            if sell_order.status == OrderStatus::Fulfilled {
                total_orders += 1;
            }

            if lef_buy_ask_amount.is_zero() || lef_buy_offer_amount.is_zero() {
                break;
            }
        }

        // there is match
        if !executor_receive_amount.is_zero() {
            // ask is order ask asset, not depending on order direction
            // so we just make sure ask amount is equal on both sides
            buy_order.fill_order(
                deps.storage,
                &pair_key,
                executor_receive_amount,
                buy_order.offer_amount - buy_order.filled_offer_amount - lef_buy_offer_amount,
            ).unwrap();

            if buy_order.status == OrderStatus::Fulfilled {
                total_orders += 1;
            }

            let executor_receive = Asset {
                info: asset_infos[1].clone(),
                amount: executor_receive_amount,
            };

            // dont use oracle for limit order
            messages.push(executor_receive.into_msg(
                None,
                &deps.querier,
                deps.api.addr_validate(bidder_addr.as_str())?,
            )?);
        }
    }

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("action", "execute_all_orders"),
        ("pair", &format!("{}/{}", &asset_infos[0], &asset_infos[1])),
        ("total_matched_orders", &total_orders.to_string()),
    ]))
}

pub fn remove_pair(
    deps: DepsMut,
    info: MessageInfo,
    asset_infos: [AssetInfo; 2],
) -> Result<Response, ContractError> {
    let contract_info = read_config(deps.storage)?;
    let sender_addr = deps.api.addr_canonicalize(info.sender.as_str())?;

    if contract_info.admin.ne(&sender_addr) {
        return Err(ContractError::Unauthorized {});
    }

    let pair_key = pair_key(&[asset_infos[0].to_raw(deps.api)?, asset_infos[1].to_raw(deps.api)?]);
    let ob = read_orderbook(deps.storage, &pair_key)?;

    let mut all_orders = ob.get_orders(deps.storage, None, None, Some(OrderBy::Ascending))?;
    
    let mut messages: Vec<CosmosMsg> = vec![];
    let mut total_orders =  0;
    for order in &mut all_orders {
        // Compute refund asset
        let left_offer_amount = match order.direction {
            OrderDirection::Buy => order.offer_amount.checked_sub(order.filled_offer_amount)?,
            OrderDirection::Sell => order.ask_amount.checked_sub(order.filled_ask_amount)?,
        };

        let bidder_refund = Asset {
            info: match order.direction {
                OrderDirection::Buy => asset_infos[0].clone(),
                OrderDirection::Sell => asset_infos[1].clone(),
            },
            amount: left_offer_amount,
        };

        // Build refund msg
        if left_offer_amount > Uint128::zero() {
            messages.push(bidder_refund.into_msg(None, &deps.querier, deps.api.addr_humanize(&order.bidder_addr)?)?);
        }
        total_orders += 1;
        order.status = OrderStatus::Cancel;
        remove_order(deps.storage, &pair_key, &order).unwrap();
    }

    remove_orderbook(deps.storage, &pair_key);

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("action", "remove_orderbook"),
        ("pair", &format!("{}/{}", &asset_infos[0], &asset_infos[1])),
        ("total_removed_orders", &total_orders.to_string()),
    ]))
}

pub fn query_order(
    deps: Deps,
    asset_infos: [AssetInfo; 2],
    order_id: u64,
) -> StdResult<OrderResponse> {
    let pair_key = pair_key(&[asset_infos[0].to_raw(deps.api)?, asset_infos[1].to_raw(deps.api)?]);
    let order = read_order(deps.storage, &pair_key, order_id)?;
    order.to_response(deps.api, asset_infos[0].clone(), asset_infos[1].clone())
}

pub fn query_orders(
    deps: Deps,
    asset_infos: [AssetInfo; 2],
    direction: Option<OrderDirection>,
    filter: OrderFilter,
    start_after: Option<u64>,
    limit: Option<u32>,
    order_by: Option<i32>,
) -> StdResult<OrdersResponse> {
    let order_by = order_by.map_or(None, |val| OrderBy::try_from(val).ok());
    let pair_key = pair_key(&[asset_infos[0].to_raw(deps.api)?, asset_infos[1].to_raw(deps.api)?]);

    let (direction_filter, direction_key): (Box<dyn Fn(&OrderDirection) -> bool>, Vec<u8>) =
        match direction {
            // copy value to closure
            Some(d) => (Box::new(move |x| d.eq(x)), d.as_bytes().to_vec()),
            None => (Box::new(|_| true), OrderDirection::Buy.as_bytes().to_vec()),
        };

    let orders: Vec<Order> = match filter {
        OrderFilter::Bidder(bidder_addr) => {
            let bidder_addr_raw = deps.api.addr_canonicalize(&bidder_addr)?;
            read_orders_with_indexer::<OrderDirection>(
                deps.storage,
                &[
                    PREFIX_ORDER_BY_BIDDER,
                    &pair_key,
                    bidder_addr_raw.as_slice(),
                ],
                direction_filter,
                start_after,
                limit,
                order_by,
            )?
        }
        OrderFilter::Tick {} => read_orders_with_indexer::<u64>(
            deps.storage,
            &[PREFIX_TICK, &pair_key, &direction_key],
            Box::new(|_| true),
            start_after,
            limit,
            order_by,
        )?,
        OrderFilter::Price(price) => {
            let price_key = price.atomics().to_be_bytes();
            read_orders_with_indexer::<OrderDirection>(
                deps.storage,
                &[PREFIX_ORDER_BY_PRICE, &pair_key, &price_key],
                direction_filter,
                start_after,
                limit,
                order_by,
            )?
        }
        OrderFilter::None => read_orders(deps.storage, &pair_key, start_after, limit, order_by)?,
    };

    let resp = OrdersResponse {
        orders: orders
            .iter()
            .map(|order| order.to_response(deps.api, asset_infos[0].clone(), asset_infos[1].clone()))
            .collect::<StdResult<Vec<OrderResponse>>>()?,
    };

    Ok(resp)
}

pub fn query_last_order_id(deps: Deps) -> StdResult<LastOrderIdResponse> {
    let last_order_id = read_last_order_id(deps.storage)?;
    let resp = LastOrderIdResponse { last_order_id };

    Ok(resp)
}

pub fn query_orderbooks(
    deps: Deps,
    start_after: Option<Vec<u8>>,
    limit: Option<u32>,
    order_by: Option<i32>,
) -> StdResult<OrderBooksResponse> {
    let order_by = order_by.map_or(None, |val| OrderBy::try_from(val).ok());
    let order_books = read_orderbooks(deps.storage, start_after, limit, order_by)?;
    order_books
        .into_iter()
        .map(|ob| ob.to_response(deps.api))
        .collect::<StdResult<Vec<OrderBookResponse>>>()
        .map(|order_books| OrderBooksResponse { order_books })
}

pub fn query_orderbook(
    deps: Deps,
    asset_infos: [AssetInfo; 2],
) -> StdResult<OrderBookResponse> {
    let pair_key = pair_key(&[asset_infos[0].to_raw(deps.api)?, asset_infos[1].to_raw(deps.api)?]);
    let ob = read_orderbook(deps.storage, &pair_key)?;
    ob.to_response(deps.api)
}
