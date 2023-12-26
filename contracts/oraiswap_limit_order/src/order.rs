use std::convert::TryFrom;
use std::str::FromStr;

use crate::orderbook::{BulkOrders, Executor, Order, OrderBook, OrderWithFee};
use crate::state::{
    increase_last_order_id, read_config, read_last_order_id, read_order, read_orderbook,
    read_orderbooks, read_orders, read_orders_with_indexer, read_reward, remove_order,
    remove_orderbook, store_order, store_reward, DEFAULT_LIMIT, MAX_LIMIT, PREFIX_ORDER_BY_BIDDER,
    PREFIX_ORDER_BY_DIRECTION, PREFIX_ORDER_BY_PRICE, PREFIX_TICK,
};
use cosmwasm_std::{
    attr, Addr, Attribute, CanonicalAddr, CosmosMsg, Decimal, Deps, DepsMut, Event, MessageInfo,
    Order as OrderBy, Response, StdError, StdResult, Storage, Uint128,
};

use cosmwasm_storage::ReadonlyBucket;
use oraiswap::asset::{pair_key, Asset, AssetInfo};
use oraiswap::error::ContractError;
use oraiswap::limit_order::{
    LastOrderIdResponse, OrderBookMatchableResponse, OrderBookResponse, OrderBooksResponse,
    OrderDirection, OrderFilter, OrderResponse, OrderStatus, OrdersResponse,
};

const RELAY_FEE: u128 = 300u128;

struct Payment {
    address: Addr,
    asset: Asset,
}

pub fn submit_order(
    deps: DepsMut,
    sender: Addr,
    pair_key: &[u8],
    direction: OrderDirection,
    assets: [Asset; 2],
) -> Result<Response, ContractError> {
    if assets[0].amount.is_zero() || assets[1].amount.is_zero() {
        return Err(ContractError::AssetMustNotBeZero {});
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
        (
            "pair",
            &format!("{} - {}", &assets[0].info, &assets[1].info),
        ),
        ("order_id", &order_id.to_string()),
        ("status", &format!("{:?}", OrderStatus::Open)),
        ("direction", &format!("{:?}", direction)),
        ("bidder_addr", sender.as_str()),
        (
            "offer_asset",
            &format!("{} {}", &assets[0].amount, &assets[0].info),
        ),
        (
            "ask_asset",
            &format!("{} {}", &assets[1].amount, &assets[1].info),
        ),
    ]))
}

pub fn cancel_order(
    deps: DepsMut,
    info: MessageInfo,
    order_id: u64,
    asset_infos: [AssetInfo; 2],
) -> Result<Response, ContractError> {
    let pair_key = pair_key(&[
        asset_infos[0].to_raw(deps.api)?,
        asset_infos[1].to_raw(deps.api)?,
    ]);
    let orderbook_pair = read_orderbook(deps.storage, &pair_key)?;
    let order = read_order(deps.storage, &pair_key, order_id)?;

    if order.bidder_addr != deps.api.addr_canonicalize(info.sender.as_str())? {
        return Err(ContractError::Unauthorized {});
    }

    // Compute refund asset
    let left_offer_amount = order.offer_amount.checked_sub(order.filled_offer_amount)?;

    let bidder_refund = Asset {
        info: match order.direction {
            OrderDirection::Buy => orderbook_pair.quote_coin_info.to_normal(deps.api)?,
            OrderDirection::Sell => orderbook_pair.base_coin_info.to_normal(deps.api)?,
        },
        amount: left_offer_amount,
    };

    // Build refund msg
    let messages = if left_offer_amount > Uint128::zero() {
        vec![bidder_refund.clone().into_msg(
            None,
            &deps.querier,
            deps.api.addr_humanize(&order.bidder_addr)?,
        )?]
    } else {
        vec![]
    };

    remove_order(deps.storage, &pair_key, &order)?;

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("action", "cancel_order"),
        (
            "pair",
            &format!(
                "{} - {}",
                &orderbook_pair.base_coin_info.to_normal(deps.api)?,
                &orderbook_pair.quote_coin_info.to_normal(deps.api)?
            ),
        ),
        ("order_id", &order_id.to_string()),
        ("direction", &format!("{:?}", order.direction)),
        ("status", "Cancel"),
        (
            "bidder_addr",
            &deps.api.addr_humanize(&order.bidder_addr)?.to_string(),
        ),
        ("offer_amount", &order.offer_amount.to_string()),
        ("ask_amount", &order.ask_amount.to_string()),
        ("bidder_refund", &bidder_refund.to_string()),
    ]))
}

fn to_events(order: &OrderWithFee, human_bidder: String) -> Event {
    let attrs: Vec<Attribute> = [
        attr("status", format!("{:?}", order.status)),
        attr("bidder_addr", human_bidder),
        attr("order_id", order.order_id.to_string()),
        attr("direction", format!("{:?}", order.direction)),
        attr("offer_amount", order.offer_amount.to_string()),
        attr("filled_offer_amount", order.filled_offer_amount.to_string()),
        attr("ask_amount", order.ask_amount.to_string()),
        attr("filled_ask_amount", order.filled_ask_amount.to_string()),
        attr("reward_fee", order.reward_fee),
        attr("relayer_fee", order.relayer_fee),
    ]
    .to_vec();
    Event::new("matched_order").add_attributes(attrs)
}

fn process_reward(
    storage: &dyn Storage,
    pair_key: &[u8],
    address: CanonicalAddr,
    reward_assets: [Asset; 2],
) -> Executor {
    let executor = read_reward(storage, &pair_key, &address);
    return match executor {
        Ok(r_reward) => r_reward,
        Err(_err) => Executor::new(address, reward_assets),
    };
}

fn transfer_reward(
    deps: &DepsMut,
    executor: &mut Executor,
    total_reward: &mut Vec<String>,
    messages: &mut Vec<CosmosMsg>,
) -> StdResult<()> {
    for reward_asset in executor.reward_assets.iter_mut() {
        if Uint128::from(reward_asset.amount) >= Uint128::from(1000000u128) {
            messages.push(reward_asset.into_msg(
                None,
                &deps.querier,
                deps.api.addr_humanize(&executor.address)?,
            )?);
            total_reward.push(reward_asset.to_string());
            reward_asset.amount = Uint128::zero();
        }
    }
    Ok(())
}

fn process_list_trader(
    deps: &DepsMut,
    traders: Vec<Payment>,
    messages: &mut Vec<CosmosMsg>,
) -> StdResult<()> {
    let mut minimalist_trader: Vec<Payment> = vec![];
    for trader in traders {
        if let Some(existing_payment) = minimalist_trader
            .iter_mut()
            .find(|p| p.address == trader.address)
        {
            existing_payment.asset.amount += trader.asset.amount;
        } else {
            minimalist_trader.push(trader);
        }
    }

    for trader in minimalist_trader {
        if !trader.asset.amount.is_zero() {
            messages.push(trader.asset.into_msg(
                None,
                &deps.querier,
                deps.api.addr_validate(trader.address.as_str())?,
            )?);
        }
    }

    Ok(())
}

fn execute_bulk_orders(
    deps: &DepsMut,
    orderbook_pair: OrderBook,
    limit: Option<u32>,
) -> StdResult<(Vec<BulkOrders>, Vec<BulkOrders>)> {
    let pair_key = &orderbook_pair.get_pair_key();
    let buy_position_bucket: ReadonlyBucket<u64> = ReadonlyBucket::multilevel(
        deps.storage,
        &[PREFIX_TICK, pair_key, OrderDirection::Buy.as_bytes()],
    );
    let mut buy_cursor = buy_position_bucket.range(None, None, OrderBy::Descending);

    let sell_position_bucket: ReadonlyBucket<u64> = ReadonlyBucket::multilevel(
        deps.storage,
        &[PREFIX_TICK, pair_key, OrderDirection::Sell.as_bytes()],
    );
    let mut sell_cursor = sell_position_bucket.range(None, None, OrderBy::Ascending);

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let mut i = 0;
    let mut j = 0;
    let min_vol = Uint128::from(10u128);

    let mut best_buy_price_list = vec![];
    let mut best_sell_price_list = vec![];
    let mut buy_bulk_orders_list = vec![];
    let mut sell_bulk_orders_list = vec![];

    while i < limit && j < limit {
        if best_sell_price_list.len() <= j {
            if let Some(Ok((k, _))) = sell_cursor.next() {
                let price =
                    Decimal::raw(u128::from_be_bytes(k.try_into().map_err(|_| {
                        StdError::generic_err("Error converting bytes to u128")
                    })?));
                best_sell_price_list.push(price);
            } else {
                break;
            }
        }
        let sell_price = best_sell_price_list[j];

        if best_buy_price_list.len() <= i {
            if let Some(Ok((k, _))) = buy_cursor.next() {
                let price =
                    Decimal::raw(u128::from_be_bytes(k.try_into().map_err(|_| {
                        StdError::generic_err("Error converting bytes to u128")
                    })?));
                best_buy_price_list.push(price);
            } else {
                break;
            }
        }
        let buy_price = best_buy_price_list[i];
        if buy_price < sell_price {
            break;
        }
        if buy_bulk_orders_list.len() <= i {
            if let Some(orders) = orderbook_pair.query_orders_by_price_and_direction(
                deps.as_ref().storage,
                buy_price,
                OrderDirection::Buy,
                None,
            ) {
                if orders.len() == 0 {
                    continue;
                }
                let bulk = BulkOrders::from_orders(&orders, buy_price, OrderDirection::Buy);
                buy_bulk_orders_list.push(bulk);
            } else {
                break;
            }
        };

        if sell_bulk_orders_list.len() <= j {
            if let Some(orders) = orderbook_pair.query_orders_by_price_and_direction(
                deps.as_ref().storage,
                sell_price,
                OrderDirection::Sell,
                None,
            ) {
                if orders.len() == 0 {
                    continue;
                }
                let bulk = BulkOrders::from_orders(&orders, sell_price, OrderDirection::Sell);
                sell_bulk_orders_list.push(bulk);
            } else {
                break;
            }
        };

        // list of buy orders and sell orders
        let buy_bulk_orders = &mut buy_bulk_orders_list[i];
        let sell_bulk_orders = &mut sell_bulk_orders_list[j];

        // match price
        let match_price = if buy_bulk_orders.average_order_id > sell_bulk_orders.average_order_id {
            buy_price
        } else {
            sell_price
        };

        let remaining_sell_volume = sell_bulk_orders.remaining_volume;
        let remaining_buy_volume = buy_bulk_orders.remaining_volume;
        let remaining_buy_ask_volume =
            Uint128::from(remaining_buy_volume * Decimal::one().atomics())
                .checked_div(match_price.atomics())?;

        // fill_base_volume = min(remaining_sell_volume, remaining_buy_ask_volume)
        // fill_quote_volume = fill_base_volume * match_price
        let fill_base_volume = Uint128::min(remaining_sell_volume, remaining_buy_ask_volume);
        let fill_quote_volume = Uint128::from(fill_base_volume * match_price);

        if fill_base_volume.is_zero() || fill_quote_volume.is_zero() {
            continue;
        }

        // In sell side
        // filled_volume = filled_volume + fill_base_volume
        // filled_ask_volume = filled_ask_volume + fill_quote_volume
        sell_bulk_orders.filled_volume += fill_base_volume;
        sell_bulk_orders.filled_ask_volume += fill_quote_volume;

        // In buy side
        // filled_volume = filled_volume + fill_quote_volume
        // filled_ask_volume = filled_ask_volume + fill_base_volume
        buy_bulk_orders.filled_volume += fill_quote_volume;
        buy_bulk_orders.filled_ask_volume += fill_base_volume;

        // In buy side
        // remaining_volume = remaining_volume - fill_quote_volume
        buy_bulk_orders.remaining_volume = buy_bulk_orders
            .remaining_volume
            .checked_sub(fill_quote_volume)?;

        // In sell side
        // remaining_volume = remaining_volume - fill_base_volume
        sell_bulk_orders.remaining_volume = sell_bulk_orders
            .remaining_volume
            .checked_sub(fill_base_volume)?;

        // get spread volume in buy side
        if buy_bulk_orders.filled_ask_volume >= buy_bulk_orders.volume {
            buy_bulk_orders.spread_volume += buy_bulk_orders
                .filled_ask_volume
                .checked_sub(buy_bulk_orders.volume)?;
            buy_bulk_orders.filled_ask_volume = buy_bulk_orders.volume;
        }

        // get spread volume in sell side
        if sell_bulk_orders.filled_ask_volume >= sell_bulk_orders.volume {
            sell_bulk_orders.spread_volume += sell_bulk_orders
                .filled_ask_volume
                .checked_sub(sell_bulk_orders.volume)?;
            sell_bulk_orders.filled_ask_volume = sell_bulk_orders.volume;
        }

        if buy_bulk_orders.remaining_volume <= min_vol {
            // buy out
            i += 1;
        }
        if sell_bulk_orders.remaining_volume <= min_vol {
            // sell out
            j += 1;
        }
    }

    return Ok((buy_bulk_orders_list, sell_bulk_orders_list));
}

// TODO: write test cases for this function
fn calculate_fee(
    deps: &DepsMut,
    amount: Uint128,
    relayer_quote_fee: Uint128,
    direction: OrderDirection,
    trader_ask_asset: &mut Asset,
    reward: &mut Executor,
    relayer: &mut Executor,
) -> StdResult<(Uint128, Uint128)> {
    let reward_fee: Uint128;
    let relayer_fee: Uint128;
    let contract_info = read_config(deps.storage)?;
    let commission_rate = Decimal::from_str(&contract_info.commission_rate)?;

    reward_fee = amount * commission_rate;

    match direction {
        OrderDirection::Buy => {
            relayer_fee = Uint128::min(Uint128::from(RELAY_FEE), amount);

            reward.reward_assets[0].amount += reward_fee;
            relayer.reward_assets[0].amount += relayer_fee;
        }
        OrderDirection::Sell => {
            relayer_fee = Uint128::min(relayer_quote_fee, amount);

            reward.reward_assets[1].amount += reward_fee;
            relayer.reward_assets[1].amount += relayer_fee;
        }
    }

    trader_ask_asset.amount = trader_ask_asset
        .amount
        .checked_sub(reward_fee + relayer_fee)
        .unwrap_or_default();
    return Ok((reward_fee, relayer_fee));
}

fn process_orders(
    deps: &DepsMut,
    orderbook_pair: &OrderBook,
    bulk_orders: &mut Vec<BulkOrders>,
    bulk_traders: &mut Vec<Payment>,
    reward: &mut Executor,
    relayer: &mut Executor,
) -> StdResult<()> {
    for bulk in bulk_orders.iter_mut() {
        let mut trader_ask_asset = Asset {
            info: match bulk.direction {
                OrderDirection::Buy => orderbook_pair.base_coin_info.to_normal(deps.api)?,
                OrderDirection::Sell => orderbook_pair.quote_coin_info.to_normal(deps.api)?,
            },
            amount: Uint128::zero(),
        };
        let relayer_quote_fee = Uint128::from(RELAY_FEE) * bulk.price;

        for order in bulk.orders.iter_mut() {
            // filled_offer = min(remain_offer, filled_volume)
            let filled_offer = Uint128::min(
                order
                    .offer_amount
                    .checked_sub(order.filled_offer_amount)
                    .unwrap_or_default(),
                bulk.filled_volume,
            );

            // filled_offer = min(remain_ask, filled_ask_volume)
            let filled_ask = Uint128::min(
                order
                    .ask_amount
                    .checked_sub(order.filled_ask_amount)
                    .unwrap_or_default(),
                bulk.filled_ask_volume,
            );

            if filled_offer.is_zero() || filled_ask.is_zero() {
                continue;
            }

            // filled_volume = filled_volume - filled_offer
            bulk.filled_volume = bulk
                .filled_volume
                .checked_sub(filled_offer)
                .unwrap_or_default();

            // filled_ask_volume = filled_ask_volume - filled_ask
            bulk.filled_ask_volume = bulk
                .filled_ask_volume
                .checked_sub(filled_ask)
                .unwrap_or_default();

            // fill order
            order.fill_order(filled_ask, filled_offer);

            // calculate fee
            if !filled_ask.is_zero() {
                trader_ask_asset.amount = filled_ask;
                let (reward_fee, relayer_fee) = calculate_fee(
                    deps,
                    filled_ask,
                    relayer_quote_fee,
                    bulk.direction,
                    &mut trader_ask_asset,
                    reward,
                    relayer,
                )?;
                order.reward_fee = reward_fee;
                order.relayer_fee = relayer_fee;
                if !trader_ask_asset.amount.is_zero() {
                    let trader_payment: Payment = Payment {
                        address: deps.api.addr_humanize(&order.bidder_addr)?,
                        asset: Asset {
                            info: trader_ask_asset.info.clone(),
                            amount: trader_ask_asset.amount,
                        },
                    };
                    bulk_traders.push(trader_payment);
                }
            }
        }
    }
    Ok(())
}

pub fn execute_matching_orders(
    deps: DepsMut,
    info: MessageInfo,
    asset_infos: [AssetInfo; 2],
    limit: Option<u32>,
) -> Result<Response, ContractError> {
    let contract_info = read_config(deps.storage)?;
    let relayer_addr = deps.api.addr_canonicalize(info.sender.as_str())?;
    let pair_key = pair_key(&[
        asset_infos[0].to_raw(deps.api)?,
        asset_infos[1].to_raw(deps.api)?,
    ]);
    let orderbook_pair = read_orderbook(deps.storage, &pair_key)?;

    let reward_wallet = contract_info.reward_address;

    let reward_assets = [
        Asset {
            info: orderbook_pair.base_coin_info.to_normal(deps.api)?,
            amount: Uint128::zero(),
        },
        Asset {
            info: orderbook_pair.quote_coin_info.to_normal(deps.api)?,
            amount: Uint128::zero(),
        },
    ];
    let mut reward = process_reward(
        deps.storage,
        &pair_key,
        reward_wallet,
        reward_assets.clone(),
    );

    let mut relayer = process_reward(deps.storage, &pair_key, relayer_addr, reward_assets);

    let mut messages: Vec<CosmosMsg> = vec![];

    let mut list_bidder: Vec<Payment> = vec![];
    let mut list_asker: Vec<Payment> = vec![];

    let mut ret_events: Vec<Event> = vec![];
    let mut total_reward: Vec<String> = Vec::new();

    let mut total_orders: u64 = 0;

    let (mut buy_list, mut sell_list) = execute_bulk_orders(&deps, orderbook_pair.clone(), limit)?;

    process_orders(
        &deps,
        &orderbook_pair,
        &mut buy_list,
        &mut list_bidder,
        &mut reward,
        &mut relayer,
    )?;

    process_orders(
        &deps,
        &orderbook_pair,
        &mut sell_list,
        &mut list_asker,
        &mut reward,
        &mut relayer,
    )?;

    for bulk in buy_list.iter_mut() {
        for buy_order in bulk.orders.iter_mut() {
            if buy_order.status != OrderStatus::Open {
                total_orders += 1;
                buy_order.match_order(deps.storage, &pair_key)?;
                ret_events.push(to_events(
                    &buy_order,
                    deps.api.addr_humanize(&buy_order.bidder_addr)?.to_string(),
                ));
            }
        }
    }

    for bulk in sell_list.iter_mut() {
        for sell_order in bulk.orders.iter_mut() {
            if sell_order.status != OrderStatus::Open {
                total_orders += 1;
                sell_order.match_order(deps.storage, &pair_key)?;
                ret_events.push(to_events(
                    &sell_order,
                    deps.api.addr_humanize(&sell_order.bidder_addr)?.to_string(),
                ));
            }
        }
    }

    process_list_trader(&deps, list_bidder, &mut messages)?;
    process_list_trader(&deps, list_asker, &mut messages)?;

    transfer_reward(&deps, &mut reward, &mut total_reward, &mut messages)?;
    transfer_reward(&deps, &mut relayer, &mut total_reward, &mut messages)?;

    store_reward(deps.storage, &pair_key, &reward)?;
    store_reward(deps.storage, &pair_key, &relayer)?;
    Ok(Response::new()
        .add_messages(messages)
        .add_attributes(vec![
            ("action", "execute_orderbook_pair"),
            (
                "pair",
                &format!("{} - {}", &asset_infos[0], &asset_infos[1]),
            ),
            ("total_matched_orders", &total_orders.to_string()),
            ("executor_reward", &format!("{:?}", &total_reward)),
        ])
        .add_events(ret_events))
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

    let pair_key = pair_key(&[
        asset_infos[0].to_raw(deps.api)?,
        asset_infos[1].to_raw(deps.api)?,
    ]);

    remove_orderbook(deps.storage, &pair_key);

    Ok(Response::new().add_attributes(vec![
        ("action", "remove_orderbook_pair"),
        (
            "pair",
            &format!("{} - {}", &asset_infos[0], &asset_infos[1]),
        ),
    ]))
}

pub fn query_order(
    deps: Deps,
    asset_infos: [AssetInfo; 2],
    order_id: u64,
) -> StdResult<OrderResponse> {
    let pair_key = pair_key(&[
        asset_infos[0].to_raw(deps.api)?,
        asset_infos[1].to_raw(deps.api)?,
    ]);
    let orderbook_pair = read_orderbook(deps.storage, &pair_key)?;
    let order = read_order(deps.storage, &pair_key, order_id)?;

    order.to_response(
        deps.api,
        orderbook_pair.base_coin_info.to_normal(deps.api)?,
        orderbook_pair.quote_coin_info.to_normal(deps.api)?,
    )
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
    let pair_key = pair_key(&[
        asset_infos[0].to_raw(deps.api)?,
        asset_infos[1].to_raw(deps.api)?,
    ]);
    let orderbook_pair = read_orderbook(deps.storage, &pair_key)?;

    let (direction_filter, direction_key): (Box<dyn Fn(&OrderDirection) -> bool>, Vec<u8>) =
        match direction {
            // copy value to closure
            Some(d) => (Box::new(move |x| d.eq(x)), d.as_bytes().to_vec()),
            None => (Box::new(|_| true), OrderDirection::Buy.as_bytes().to_vec()),
        };

    let orders: Option<Vec<Order>> = match filter {
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
        OrderFilter::None => match direction {
            Some(_) => read_orders_with_indexer::<OrderDirection>(
                deps.storage,
                &[PREFIX_ORDER_BY_DIRECTION, &pair_key, &direction_key],
                direction_filter,
                start_after,
                limit,
                order_by,
            )?,
            None => Some(read_orders(
                deps.storage,
                &pair_key,
                start_after,
                limit,
                order_by,
            )?),
        },
    };

    let resp = OrdersResponse {
        orders: orders
            .unwrap_or_default()
            .iter()
            .map(|order| {
                order.to_response(
                    deps.api,
                    orderbook_pair.base_coin_info.to_normal(deps.api)?,
                    orderbook_pair.quote_coin_info.to_normal(deps.api)?,
                )
            })
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

pub fn query_orderbook(deps: Deps, asset_infos: [AssetInfo; 2]) -> StdResult<OrderBookResponse> {
    let pair_key = pair_key(&[
        asset_infos[0].to_raw(deps.api)?,
        asset_infos[1].to_raw(deps.api)?,
    ]);
    let ob = read_orderbook(deps.storage, &pair_key)?;
    ob.to_response(deps.api)
}

pub fn query_orderbook_is_matchable(
    deps: Deps,
    asset_infos: [AssetInfo; 2],
) -> StdResult<OrderBookMatchableResponse> {
    let pair_key = pair_key(&[
        asset_infos[0].to_raw(deps.api)?,
        asset_infos[1].to_raw(deps.api)?,
    ]);
    let ob = read_orderbook(deps.storage, &pair_key)?;
    let (best_buy_price_list, best_sell_price_list) = ob
        .find_list_match_price(deps.storage, Some(30))
        .unwrap_or_default();

    Ok(OrderBookMatchableResponse {
        is_matchable: best_buy_price_list.len() != 0 && best_sell_price_list.len() != 0,
    })
}
