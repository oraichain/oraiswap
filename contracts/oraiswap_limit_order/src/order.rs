use std::convert::TryFrom;

use crate::orderbook::Order;
use crate::state::{
    increase_last_order_id, read_last_order_id, read_order, read_orderbook, read_orders,
    read_orders_with_indexer, remove_order, store_order, PREFIX_ORDER_BY_BIDDER,
    PREFIX_ORDER_BY_PRICE, PREFIX_TICK,
};
use cosmwasm_std::{
    Addr, CosmosMsg, Deps, DepsMut, MessageInfo, Order as OrderBy, Response, StdResult, Uint128,
};

use oraiswap::asset::{pair_key, Asset, AssetInfo};
use oraiswap::error::ContractError;
use oraiswap::limit_order::{
    LastOrderIdResponse, OrderDirection, OrderFilter, OrderResponse, OrdersResponse,
};

pub fn submit_order(
    deps: DepsMut,
    sender: Addr,
    direction: OrderDirection,
    offer_asset: Asset,
    ask_asset: Asset,
) -> Result<Response, ContractError> {
    // check min offer amount and min ask amount
    // need to setup min offer_amount and ask_amount for a specific pair so that no one can spam
    let offer_asset_raw = offer_asset.to_raw(deps.api)?;
    let ask_asset_raw = ask_asset.to_raw(deps.api)?;
    let pair_key = pair_key(&[offer_asset_raw.info, ask_asset_raw.info]);
    let (_, min_offer_amount) = read_orderbook(deps.storage, &pair_key);

    // require minimum amount for the orderbook
    if offer_asset.amount.lt(&min_offer_amount) {
        return Err(ContractError::TooSmallOfferAmount {});
    }

    let order_id = increase_last_order_id(deps.storage)?;

    let total_orders = store_order(
        deps.storage,
        &pair_key,
        &Order {
            order_id,
            direction,
            bidder_addr: deps.api.addr_canonicalize(sender.as_str())?,
            offer_amount: offer_asset_raw.amount,
            ask_amount: ask_asset_raw.amount,
            filled_offer_amount: Uint128::zero(),
            filled_ask_amount: Uint128::zero(),
        },
        true,
    )?;

    Ok(Response::new().add_attributes(vec![
        ("action", "submit_order"),
        ("order_id", &order_id.to_string()),
        ("bidder_addr", sender.as_str()),
        ("offer_asset", &offer_asset.to_string()),
        ("ask_asset", &ask_asset.to_string()),
        ("total_orders", &total_orders.to_string()),
    ]))
}

pub fn cancel_order(
    deps: DepsMut,
    info: MessageInfo,
    offer_info: AssetInfo,
    ask_info: AssetInfo,
    order_id: u64,
) -> Result<Response, ContractError> {
    let pair_key = pair_key(&[offer_info.to_raw(deps.api)?, ask_info.to_raw(deps.api)?]);
    let order = read_order(deps.storage, &pair_key, order_id)?;

    if order.bidder_addr != deps.api.addr_canonicalize(info.sender.as_str())? {
        return Err(ContractError::Unauthorized {});
    }

    // Compute refund asset
    let left_offer_amount = order.offer_amount.checked_sub(order.filled_offer_amount)?;
    let bidder_refund = Asset {
        info: offer_info,
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

    let total_orders = remove_order(deps.storage, &pair_key, &order)?;

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("action", "cancel_order"),
        ("order_id", &order_id.to_string()),
        ("bidder_refund", &bidder_refund.to_string()),
        ("total_orders", &total_orders.to_string()),
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

    // Compute offer amount & left ask amount
    let (offer_amount, left_ask_amount) = order.matchable_amount(ask_asset.amount)?;
    let executor_receive = Asset {
        info: offer_info,
        amount: offer_amount,
    };

    let bidder_addr = deps.api.addr_humanize(&order.bidder_addr)?;

    // When left amount is zero, close order
    let total_orders = if left_ask_amount == ask_asset.amount {
        remove_order(deps.storage, &pair_key, &order)?
    } else {
        order.filled_ask_amount += ask_asset.amount;
        order.filled_offer_amount += executor_receive.amount;
        // update order
        store_order(deps.storage, &pair_key, &order, false)?
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
        ("total_orders", &total_orders.to_string()),
    ]))
}

pub fn query_order(
    deps: Deps,
    offer_info: AssetInfo,
    ask_info: AssetInfo,
    order_id: u64,
) -> StdResult<OrderResponse> {
    let pair_key = pair_key(&[offer_info.to_raw(deps.api)?, ask_info.to_raw(deps.api)?]);
    let order = read_order(deps.storage, &pair_key, order_id)?;
    order.to_response(deps.api, offer_info, ask_info)
}

pub fn query_orders(
    deps: Deps,
    offer_info: AssetInfo,
    ask_info: AssetInfo,
    direction: Option<OrderDirection>,
    filter: OrderFilter,
    start_after: Option<u64>,
    limit: Option<u32>,
    order_by: Option<i32>,
) -> StdResult<OrdersResponse> {
    let order_by = order_by.map_or(None, |val| OrderBy::try_from(val).ok());
    let pair_key = pair_key(&[offer_info.to_raw(deps.api)?, ask_info.to_raw(deps.api)?]);

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
            .map(|order| order.to_response(deps.api, offer_info.clone(), ask_info.clone()))
            .collect::<StdResult<Vec<OrderResponse>>>()?,
    };

    Ok(resp)
}

pub fn query_last_order_id(deps: Deps) -> StdResult<LastOrderIdResponse> {
    let last_order_id = read_last_order_id(deps.storage)?;
    let resp = LastOrderIdResponse { last_order_id };

    Ok(resp)
}
