use std::str::FromStr;

use crate::orderbook::{BulkOrders, Executor, Order, OrderBook, OrderWithFee};
use crate::state::{
    increase_last_order_id, read_config, read_order, read_orderbook, read_reward, remove_order,
    remove_orderbook, store_order, store_reward, DEFAULT_LIMIT, MAX_LIMIT, PREFIX_TICK,
};
use cosmwasm_std::{
    attr, Addr, Api, Attribute, CanonicalAddr, CosmosMsg, Decimal, Deps, DepsMut, Event,
    MessageInfo, Order as OrderBy, Response, StdError, StdResult, Storage, Uint128,
};

use cosmwasm_storage::ReadonlyBucket;
use oraiswap::asset::{pair_key, Asset, AssetInfo};
use oraiswap::error::ContractError;
use oraiswap::limit_order::{OrderDirection, OrderStatus};

pub const RELAY_FEE: u128 = 300u128;
pub const MIN_VOLUME: u128 = 10u128;
const MIN_FEE: u128 = 1_000_000u128;
const SLIPPAGE_DEFAULT: &str = "0.01"; // spread default 1%

struct Payment {
    address: Addr,
    asset: Asset,
}

pub fn submit_order(
    deps: DepsMut,
    orderbook_pair: &OrderBook,
    sender: Addr,
    direction: OrderDirection,
    assets: [Asset; 2],
) -> Result<Response, ContractError> {
    assets[0].assert_if_asset_is_zero()?;
    assets[1].assert_if_asset_is_zero()?;

    let offer_amount = assets[0].amount;
    let ask_amount = assets[1].amount;

    if ask_amount.is_zero() {
        return Err(ContractError::AssetMustNotBeZero {});
    }

    let order_id = increase_last_order_id(deps.storage)?;

    store_order(
        deps.storage,
        &orderbook_pair.get_pair_key(),
        &Order {
            order_id,
            direction,
            bidder_addr: deps.api.addr_canonicalize(sender.as_str())?,
            offer_amount,
            ask_amount,
            filled_offer_amount: Uint128::zero(),
            filled_ask_amount: Uint128::zero(),
            status: OrderStatus::Open,
        },
        true,
    )?;

    // check if this order can be matched, then execute matching process

    let (matched, price) = match direction {
        OrderDirection::Buy => {
            let price = Decimal::from_ratio(offer_amount, ask_amount);
            let (lowest_sell_price, sell_found, _) =
                orderbook_pair.lowest_price(deps.storage, OrderDirection::Sell);

            (sell_found && lowest_sell_price <= price, price)
        }
        OrderDirection::Sell => {
            let price = Decimal::from_ratio(ask_amount, offer_amount);

            let (highest_buy_price, buy_found, _) =
                orderbook_pair.highest_price(deps.storage, OrderDirection::Buy);

            (buy_found && highest_buy_price >= price, price)
        }
    };

    let response = if matched {
        process_matching(deps, sender.clone(), orderbook_pair, order_id, price)?
    } else {
        Response::new()
    };

    Ok(response.add_attributes(vec![
        ("action", "submit_order"),
        ("order_type", "limit"),
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

pub fn submit_market_order(
    mut deps: DepsMut,
    orderbook_pair: &OrderBook,
    sender: Addr,
    direction: OrderDirection,
    offer_asset: Asset,
    slippage: Option<Decimal>,
) -> Result<Response, ContractError> {
    let order_id = increase_last_order_id(deps.storage)?;

    let slippage = slippage.unwrap_or(
        orderbook_pair
            .spread
            .unwrap_or(Decimal::from_str(SLIPPAGE_DEFAULT)?),
    );

    if slippage >= Decimal::one() {
        return Err(ContractError::SlippageMustLessThanOne { slippage });
    }

    let (best_price, price_threshold) = match direction {
        OrderDirection::Buy => {
            let (lowest_sell_price, sell_found, _) =
                orderbook_pair.lowest_price(deps.storage, OrderDirection::Sell);

            if !sell_found {
                (Decimal::zero(), Decimal::zero())
            } else {
                (
                    lowest_sell_price,
                    lowest_sell_price * (Decimal::one() + slippage),
                )
            }
        }
        OrderDirection::Sell => {
            let (highest_buy_price, buy_found, _) =
                orderbook_pair.highest_price(deps.storage, OrderDirection::Buy);

            if !buy_found {
                (Decimal::zero(), Decimal::zero())
            } else {
                (
                    highest_buy_price,
                    highest_buy_price * (Decimal::one() - slippage),
                )
            }
        }
    };

    if best_price.is_zero() {
        return Err(ContractError::CannotCreateMarketOrder {});
    }

    // with market order, ask_amount will be maximum amount can receive
    let max_ask_amount = (offer_asset.amount * Decimal::one().atomics())
        .checked_div(best_price.atomics())
        .unwrap();

    store_order(
        deps.storage,
        &orderbook_pair.get_pair_key(),
        &Order {
            order_id,
            direction,
            bidder_addr: deps.api.addr_canonicalize(sender.as_str())?,
            offer_amount: offer_asset.amount,
            ask_amount: max_ask_amount,
            filled_offer_amount: Uint128::zero(),
            filled_ask_amount: Uint128::zero(),
            status: OrderStatus::Open,
        },
        true,
    )?;

    // matching process
    let mut response = process_matching(
        deps.branch(),
        sender.clone(),
        orderbook_pair,
        order_id,
        price_threshold,
    )?;

    let order = read_order(deps.storage, &orderbook_pair.get_pair_key(), order_id);

    match order {
        Ok(order) => {
            let refund_amount = order.offer_amount.checked_sub(order.filled_offer_amount)?;
            let refund_asset = Asset {
                info: offer_asset.info.clone(),
                amount: refund_amount,
            };
            response = response
                .add_attribute("refund_amount", &refund_amount.to_string())
                .add_message(refund_asset.into_msg(None, &deps.querier, sender.clone())?)
        }
        Err(_) => {}
    }

    Ok(response.add_attributes(vec![
        ("action", "submit_market_order"),
        ("order_type", "market"),
        (
            "pair",
            &format!(
                "{} - {}",
                &orderbook_pair.base_coin_info.to_normal(deps.api)?,
                &orderbook_pair.quote_coin_info.to_normal(deps.api)?
            ),
        ),
        ("order_id", &order_id.to_string()),
        ("status", &format!("{:?}", OrderStatus::Open)),
        ("direction", &format!("{:?}", direction)),
        ("bidder_addr", sender.as_str()),
        (
            "offer_asset",
            &format!("{} {}", &offer_asset.amount, &offer_asset.info),
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
        attr("filled_offer_this_round", order.filled_offer_this_round),
        attr("filled_ask_this_round", order.filled_ask_this_round),
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
        if Uint128::from(reward_asset.amount) >= Uint128::from(MIN_FEE) {
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
            .find(|p| p.address == trader.address && p.asset.info.eq(&trader.asset.info))
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
                let bulk = BulkOrders::from_orders(&orders, buy_price, OrderDirection::Buy)?;
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
                let bulk = BulkOrders::from_orders(&orders, sell_price, OrderDirection::Sell)?;
                sell_bulk_orders_list.push(bulk);
            } else {
                break;
            }
        };

        let buy_bulk_orders = &mut buy_bulk_orders_list[i];
        let sell_bulk_orders = &mut sell_bulk_orders_list[j];

        let match_price = buy_price;
        let lef_sell_offer = sell_bulk_orders.volume;
        let lef_sell_ask = Uint128::from(lef_sell_offer * match_price);

        let sell_ask_amount = Uint128::min(buy_bulk_orders.volume, lef_sell_ask);

        // multiply by decimal atomics because we want to get good round values
        let sell_offer_amount = Uint128::min(
            Uint128::from(sell_ask_amount * Decimal::one().atomics())
                .checked_div(match_price.atomics())?,
            lef_sell_offer,
        );

        sell_bulk_orders.filled_volume += sell_offer_amount;
        sell_bulk_orders.filled_ask_volume += sell_ask_amount;

        buy_bulk_orders.filled_volume += sell_ask_amount;
        buy_bulk_orders.filled_ask_volume += sell_offer_amount;

        buy_bulk_orders.volume = buy_bulk_orders.volume.checked_sub(sell_ask_amount)?;
        sell_bulk_orders.volume = sell_bulk_orders.volume.checked_sub(sell_offer_amount)?;

        if buy_bulk_orders.volume <= MIN_VOLUME.into() {
            // buy out
            // buy_bulk_orders.ask_volume = Uint128::zero();
            i += 1;
        }
        if sell_bulk_orders.volume <= MIN_VOLUME.into() {
            // sell out
            // sell_bulk_orders.ask_volume = Uint128::zero();
            j += 1;
        }
    }

    return Ok((buy_bulk_orders_list, sell_bulk_orders_list));
}

pub fn calculate_fee(
    commission_rate: Decimal,
    relayer_quote_fee: Uint128,
    direction: OrderDirection,
    trader_ask_asset: &mut Asset,
    reward: &mut Executor,
    relayer: &mut Executor,
) -> StdResult<(Uint128, Uint128)> {
    let relayer_fee: Uint128;
    let reward_fee = trader_ask_asset.amount * commission_rate;
    let remaining_amount = trader_ask_asset.amount.checked_sub(reward_fee)?;
    match direction {
        OrderDirection::Buy => {
            relayer_fee = Uint128::min(Uint128::from(RELAY_FEE), remaining_amount);

            reward.reward_assets[0].amount += reward_fee;
            relayer.reward_assets[0].amount += relayer_fee;
        }
        OrderDirection::Sell => {
            relayer_fee = Uint128::min(relayer_quote_fee, remaining_amount);

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
    deps: &Deps,
    commission_rate: Decimal,
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
            let filled_offer = Uint128::min(
                order
                    .offer_amount
                    .checked_sub(order.filled_offer_amount)
                    .unwrap_or_default(),
                bulk.filled_volume,
            );

            let filled_ask = Uint128::min(
                order
                    .ask_amount
                    .checked_sub(order.filled_ask_amount)
                    .unwrap_or_default(),
                bulk.filled_ask_volume,
            );

            bulk.filled_volume = bulk
                .filled_volume
                .checked_sub(filled_offer)
                .unwrap_or_default();
            bulk.filled_ask_volume = bulk
                .filled_ask_volume
                .checked_sub(filled_ask)
                .unwrap_or_default();

            order.fill_order(filled_ask, filled_offer)?;

            if !filled_ask.is_zero() && !filled_offer.is_zero() {
                trader_ask_asset.amount = filled_ask;
                let (reward_fee, relayer_fee) = calculate_fee(
                    commission_rate,
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

fn process_payment_list_orders(
    deps: &Deps,
    commission_rate: Decimal,
    orderbook_pair: &OrderBook,
    orders: &mut Vec<OrderWithFee>,
    traders: &mut Vec<Payment>,
    reward: &mut Executor,
    relayer: &mut Executor,
) -> StdResult<()> {
    for order in orders {
        let filled_offer = order.filled_offer_this_round;
        let filled_ask = order.filled_ask_this_round;

        let relayer_quote_fee = Uint128::from(RELAY_FEE) * order.get_price();

        if !filled_ask.is_zero() && !filled_offer.is_zero() {
            let mut trader_ask_asset = Asset {
                info: match order.direction {
                    OrderDirection::Buy => orderbook_pair.base_coin_info.to_normal(deps.api)?,
                    OrderDirection::Sell => orderbook_pair.quote_coin_info.to_normal(deps.api)?,
                },
                amount: filled_ask,
            };

            let (reward_fee, relayer_fee) = calculate_fee(
                commission_rate,
                relayer_quote_fee,
                order.direction,
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
                traders.push(trader_payment);
            }
        }
    }

    Ok(())
}

pub fn matching_buy_order(
    deps: &DepsMut,
    orderbook_pair: OrderBook,
    order: &Order,
    max_buy_price: Decimal,
) -> StdResult<(OrderWithFee, Vec<OrderWithFee>)> {
    let pair_key = &orderbook_pair.get_pair_key();

    let sell_position_bucket: ReadonlyBucket<u64> = ReadonlyBucket::multilevel(
        deps.storage,
        &[PREFIX_TICK, pair_key, OrderDirection::Sell.as_bytes()],
    );

    let mut buy_order = OrderWithFee::from_order(order.to_owned());
    let mut sell_orders_matched: Vec<OrderWithFee> = vec![];
    let mut total_offer_filled = Uint128::zero();
    let mut total_ask_filled = Uint128::zero();

    let mut sell_cursor = sell_position_bucket.range(None, None, OrderBy::Ascending);
    loop {
        if let Some(Ok((k, _))) = sell_cursor.next() {
            let sell_price =
                Decimal::raw(u128::from_be_bytes(k.try_into().map_err(|_| {
                    StdError::generic_err("Error converting bytes to u128")
                })?));

            if max_buy_price < sell_price {
                break;
            }

            if let Some(orders) = orderbook_pair.query_orders_by_price_and_direction(
                deps.as_ref().storage,
                sell_price,
                OrderDirection::Sell,
                None,
            ) {
                if orders.len() == 0 {
                    continue;
                }

                let sell_orders_with_fee = OrderWithFee::from_orders(orders);

                let match_price = sell_price;

                for mut sell_order in sell_orders_with_fee {
                    let lef_sell_offer = sell_order
                        .offer_amount
                        .checked_sub(sell_order.filled_offer_amount)?;
                    let lef_sell_ask = sell_order
                        .ask_amount
                        .checked_sub(sell_order.filled_ask_amount)?;

                    let sell_ask_amount = Uint128::min(
                        buy_order.offer_amount.checked_sub(total_offer_filled)?,
                        lef_sell_ask,
                    );
                    // multiply by decimal atomics because we want to get good round values
                    let sell_offer_amount = Uint128::min(
                        Uint128::from(sell_ask_amount * Decimal::one().atomics())
                            .checked_div(match_price.atomics())?,
                        lef_sell_offer,
                    );

                    sell_order.fill_order(sell_ask_amount, sell_offer_amount)?;

                    total_offer_filled += sell_ask_amount;
                    total_ask_filled += sell_offer_amount;

                    sell_orders_matched.push(sell_order);
                    if buy_order.is_fulfilled() {
                        break;
                    }
                }
                if buy_order.is_fulfilled() {
                    break;
                }
            }
        } else {
            break;
        }
    }

    buy_order.fill_order(total_ask_filled, total_offer_filled)?;

    Ok((buy_order, sell_orders_matched))
}

pub fn matching_sell_order(
    deps: &DepsMut,
    orderbook_pair: OrderBook,
    order: &Order,
    min_sell_price: Decimal,
) -> StdResult<(OrderWithFee, Vec<OrderWithFee>)> {
    let pair_key = &orderbook_pair.get_pair_key();

    let buy_position_bucket: ReadonlyBucket<u64> = ReadonlyBucket::multilevel(
        deps.storage,
        &[PREFIX_TICK, pair_key, OrderDirection::Buy.as_bytes()],
    );

    let mut sell_order = OrderWithFee::from_order(order.to_owned());
    let mut buy_orders_matched: Vec<OrderWithFee> = vec![];

    let mut buy_cursor = buy_position_bucket.range(None, None, OrderBy::Descending);
    let mut total_offer_filled = Uint128::zero();
    let mut total_ask_filled = Uint128::zero();

    loop {
        if let Some(Ok((k, _))) = buy_cursor.next() {
            let buy_price =
                Decimal::raw(u128::from_be_bytes(k.try_into().map_err(|_| {
                    StdError::generic_err("Error converting bytes to u128")
                })?));

            if min_sell_price < buy_price {
                break;
            }

            if let Some(orders) = orderbook_pair.query_orders_by_price_and_direction(
                deps.as_ref().storage,
                buy_price,
                OrderDirection::Buy,
                None,
            ) {
                if orders.len() == 0 {
                    continue;
                }

                let buy_orders_with_fee = OrderWithFee::from_orders(orders);

                let match_price = buy_price;

                for mut buy_order in buy_orders_with_fee {
                    let lef_buy_offer = buy_order
                        .offer_amount
                        .checked_sub(buy_order.filled_offer_amount)?;

                    let lef_buy_ask = buy_order
                        .ask_amount
                        .checked_sub(buy_order.filled_ask_amount)?;

                    let buy_ask_amount = Uint128::min(
                        sell_order.offer_amount.checked_sub(total_offer_filled)?,
                        lef_buy_ask,
                    );
                    // multiply by decimal atomics because we want to get good round values
                    let buy_offer_amount =
                        Uint128::min(buy_ask_amount * match_price, lef_buy_offer);

                    buy_order.fill_order(buy_ask_amount, buy_offer_amount)?;

                    total_offer_filled += buy_ask_amount;
                    total_ask_filled += buy_offer_amount;

                    buy_orders_matched.push(buy_order);
                    if sell_order.is_fulfilled() {
                        break;
                    }
                }
                if sell_order.is_fulfilled() {
                    break;
                }
            }
        } else {
            break;
        }
    }

    sell_order.fill_order(total_ask_filled, total_ask_filled)?;

    Ok((sell_order, buy_orders_matched))
}

pub fn process_matching(
    deps: DepsMut,
    sender: Addr,
    orderbook_pair: &OrderBook,
    order_id: u64,
    price_threshold: Decimal,
) -> Result<Response, ContractError> {
    let contract_info = read_config(deps.storage)?;
    let commission_rate = Decimal::from_str(&contract_info.commission_rate)?;

    // get default operator to receive reward
    let relayer_addr = match contract_info.operator {
        Some(addr) => addr,
        None => deps.api.addr_canonicalize(sender.as_str())?,
    };
    let pair_key = orderbook_pair.get_pair_key();

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
        contract_info.reward_address,
        reward_assets.clone(),
    );

    let mut relayer = process_reward(deps.storage, &pair_key, relayer_addr, reward_assets);

    let mut messages: Vec<CosmosMsg> = vec![];
    let mut list_trader: Vec<Payment> = vec![];
    let mut ret_events: Vec<Event> = vec![];
    let mut total_reward: Vec<String> = Vec::new();
    let mut total_orders: u64 = 0;

    let order = read_order(deps.storage, &pair_key, order_id)?;

    let (offer_order_with_fee, mut matched_orders) = match order.direction {
        OrderDirection::Buy => {
            matching_buy_order(&deps, orderbook_pair.clone(), &order, price_threshold)?
        }
        OrderDirection::Sell => {
            matching_sell_order(&deps, orderbook_pair.clone(), &order, price_threshold)?
        }
    };

    if matched_orders.len() == 0 {
        return Ok(Response::default());
    }

    // process calc payment and fee
    matched_orders.push(offer_order_with_fee);
    process_payment_list_orders(
        &deps.as_ref(),
        commission_rate,
        orderbook_pair,
        &mut matched_orders,
        &mut list_trader,
        &mut reward,
        &mut relayer,
    )?;

    for order_matched in matched_orders.iter_mut() {
        if order_matched.status != OrderStatus::Open {
            total_orders += 1;
            order_matched.match_order(deps.storage, &pair_key)?;
            ret_events.push(to_events(
                &order_matched,
                deps.api
                    .addr_humanize(&order_matched.bidder_addr)?
                    .to_string(),
            ));
        }
    }

    process_list_trader(&deps, list_trader, &mut messages)?;

    transfer_reward(&deps, &mut reward, &mut total_reward, &mut messages)?;
    transfer_reward(&deps, &mut relayer, &mut total_reward, &mut messages)?;

    store_reward(deps.storage, &pair_key, &reward)?;
    store_reward(deps.storage, &pair_key, &relayer)?;

    Ok(Response::new()
        .add_messages(messages)
        .add_attributes(vec![
            ("total_matched_orders", &total_orders.to_string()),
            ("executor_reward", &format!("{:?}", &total_reward)),
        ])
        .add_events(ret_events))
}

pub fn execute_matching_orders(
    deps: DepsMut,
    info: MessageInfo,
    asset_infos: [AssetInfo; 2],
    limit: Option<u32>,
) -> Result<Response, ContractError> {
    let contract_info = read_config(deps.storage)?;
    let commission_rate = Decimal::from_str(&contract_info.commission_rate)?;

    // get default operator to receive reward
    let relayer_addr = match contract_info.operator {
        Some(addr) => addr,
        None => deps.api.addr_canonicalize(info.sender.as_str())?,
    };
    let pair_key = pair_key(&[
        asset_infos[0].to_raw(deps.api)?,
        asset_infos[1].to_raw(deps.api)?,
    ]);
    let orderbook_pair = read_orderbook(deps.storage, &pair_key)?;
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
        contract_info.reward_address,
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

    if buy_list.len() == 0 || sell_list.len() == 0 {
        return Err(ContractError::UnableToExecuteMatching {});
    }

    process_orders(
        &deps.as_ref(),
        commission_rate,
        &orderbook_pair,
        &mut buy_list,
        &mut list_bidder,
        &mut reward,
        &mut relayer,
    )?;
    process_orders(
        &deps.as_ref(),
        commission_rate,
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

pub fn get_paid_and_quote_assets(
    api: &dyn Api,
    orderbook_pair: &OrderBook,
    assets: [Asset; 2],
    direction: OrderDirection,
) -> StdResult<([Asset; 2], Asset)> {
    let mut assets_reverse = assets.clone();
    assets_reverse.reverse();
    let paid_assets: [Asset; 2];
    let quote_asset: Asset;

    if orderbook_pair.base_coin_info.to_normal(api)? == assets[0].info {
        paid_assets = match direction {
            OrderDirection::Buy => assets_reverse,
            OrderDirection::Sell => assets.clone(),
        };
        quote_asset = assets[1].clone();
    } else {
        paid_assets = match direction {
            OrderDirection::Buy => assets.clone(),
            OrderDirection::Sell => assets_reverse,
        };
        quote_asset = assets[0].clone();
    }
    Ok((paid_assets, quote_asset))
}

pub fn get_market_asset(
    api: &dyn Api,
    orderbook_pair: &OrderBook,
    direction: OrderDirection,
    market_price: Decimal,
    base_amount: Uint128,
) -> StdResult<([Asset; 2], Asset)> {
    let quote_amount = Uint128::from(base_amount * market_price);
    let quote_asset = Asset {
        info: orderbook_pair.quote_coin_info.to_normal(api)?,
        amount: quote_amount,
    };
    let mut assets = [
        Asset {
            info: orderbook_pair.quote_coin_info.to_normal(api)?,
            amount: quote_amount,
        },
        Asset {
            info: orderbook_pair.base_coin_info.to_normal(api)?,
            amount: base_amount,
        },
    ];
    let paid_assets = match direction {
        OrderDirection::Buy => assets.clone(),
        OrderDirection::Sell => {
            assets.reverse();
            assets.clone()
        }
    };
    Ok((paid_assets, quote_asset))
}

pub fn get_native_asset(info: &MessageInfo, asset_info: AssetInfo) -> StdResult<Asset> {
    if let AssetInfo::NativeToken { denom } = asset_info.clone() {
        //check funds includes To token
        if let Some(native_coin) = info.funds.iter().find(|a| a.denom.eq(&denom)) {
            let amount = native_coin.amount;
            let asset = Asset {
                info: asset_info.clone(),
                amount: amount.clone(),
            };
            return Ok(asset);
        } else {
            return Err(StdError::generic_err(
                "Cannot find the native token that matches the input",
            ));
        };
    } else {
        return Err(StdError::generic_err("invalid cw20 hook message"));
    }
}
