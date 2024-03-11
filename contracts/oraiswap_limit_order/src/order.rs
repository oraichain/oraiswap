use std::str::FromStr;

use crate::orderbook::{Executor, Order, OrderBook, OrderWithFee};
use crate::state::{
    increase_last_order_id, read_config, read_order, read_orderbook, read_reward, remove_order,
    remove_orderbook, store_order, store_reward, PREFIX_TICK,
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

    // with market order, ask_amount will be maximum amount can receive
    let (best_price, price_threshold, max_ask_amount) = match direction {
        OrderDirection::Buy => {
            let (lowest_sell_price, sell_found, _) =
                orderbook_pair.lowest_price(deps.storage, OrderDirection::Sell);

            if !sell_found {
                (Decimal::zero(), Decimal::zero(), Uint128::zero())
            } else {
                (
                    lowest_sell_price,
                    lowest_sell_price * (Decimal::one() + slippage),
                    (offer_asset.amount * Decimal::one().atomics())
                        .checked_div(lowest_sell_price.atomics())
                        .unwrap(),
                )
            }
        }
        OrderDirection::Sell => {
            let (highest_buy_price, buy_found, _) =
                orderbook_pair.highest_price(deps.storage, OrderDirection::Buy);

            if !buy_found {
                (Decimal::zero(), Decimal::zero(), Uint128::zero())
            } else {
                (
                    highest_buy_price,
                    highest_buy_price * (Decimal::one() - slippage),
                    offer_asset.amount * highest_buy_price,
                )
            }
        }
    };

    if best_price.is_zero() {
        return Err(ContractError::CannotCreateMarketOrder {});
    }

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

    // after matching process, if order still exist => not fulfilled => refund
    let order = read_order(deps.storage, &orderbook_pair.get_pair_key(), order_id);

    match order {
        Ok(order) => {
            let refund_amount = order.offer_amount.checked_sub(order.filled_offer_amount)?;
            let refund_asset = Asset {
                info: offer_asset.info.clone(),
                amount: refund_amount,
            };
            // remove this order
            remove_order(deps.storage, &orderbook_pair.get_pair_key(), &order)?;
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

// pub fn matching_buy_order(
//     deps: &DepsMut,
//     orderbook_pair: OrderBook,
//     order: &Order,
//     max_buy_price: Decimal,
// ) -> StdResult<(OrderWithFee, Vec<OrderWithFee>)> {
//     let pair_key = &orderbook_pair.get_pair_key();

//     let sell_position_bucket: ReadonlyBucket<u64> = ReadonlyBucket::multilevel(
//         deps.storage,
//         &[PREFIX_TICK, pair_key, OrderDirection::Sell.as_bytes()],
//     );

//     let mut buy_order = OrderWithFee::from_order(order.to_owned());
//     let mut sell_orders_matched: Vec<OrderWithFee> = vec![];
//     let mut total_offer_filled = Uint128::zero();
//     let mut total_ask_filled = Uint128::zero();

//     let mut sell_cursor = sell_position_bucket.range(None, None, OrderBy::Ascending);
//     loop {
//         if let Some(Ok((k, _))) = sell_cursor.next() {
//             let sell_price =
//                 Decimal::raw(u128::from_be_bytes(k.try_into().map_err(|_| {
//                     StdError::generic_err("Error converting bytes to u128")
//                 })?));

//             if max_buy_price < sell_price {
//                 break;
//             }

//             if buy_order.is_fulfilled() {
//                 break;
//             }

//             if let Some(orders) = orderbook_pair.query_orders_by_price_and_direction(
//                 deps.as_ref().storage,
//                 sell_price,
//                 OrderDirection::Sell,
//                 None,
//             ) {
//                 if orders.len() == 0 {
//                     continue;
//                 }
//                 if buy_order.is_fulfilled() {
//                     break;
//                 }

//                 let sell_orders_with_fee = OrderWithFee::from_orders(orders);

//                 let match_price = sell_price;

//                 for mut sell_order in sell_orders_with_fee {
//                     if buy_order.is_fulfilled() {
//                         break;
//                     }
//                     // remaining ask & offer of buy order
//                     let lef_buy_ask = buy_order.ask_amount.checked_sub(total_ask_filled)?;
//                     let lef_buy_offer = buy_order.offer_amount.checked_sub(total_offer_filled)?;

//                     // remaining offer & ask of sell order
//                     let lef_sell_offer = sell_order
//                         .offer_amount
//                         .checked_sub(sell_order.filled_offer_amount)?;
//                     let lef_sell_ask = sell_order
//                         .ask_amount
//                         .checked_sub(sell_order.filled_ask_amount)?;

//                     // ask_amount of buy_order <= min(lef_buy_ask, lef_sell_offer)
//                     let mut buy_ask_amount = Uint128::min(lef_buy_ask, lef_sell_offer);
//                     let mut buy_offer_amount = buy_ask_amount * match_price;

//                     // if sell_offer_amount > lef_sell_offer, we need re calc ask_amount
//                     if buy_offer_amount > lef_buy_offer {
//                         buy_offer_amount = lef_buy_offer;
//                         buy_ask_amount = Uint128::from(buy_offer_amount * Decimal::one().atomics())
//                             .checked_div(match_price.atomics())?;
//                     }

//                     // ask_amount receive of sell order = min(actual, expect)
//                     let sell_ask_amount = Uint128::min(buy_offer_amount, lef_sell_ask);

//                     sell_order.fill_order(sell_ask_amount, buy_ask_amount)?;

//                     total_offer_filled += buy_offer_amount;
//                     total_ask_filled += buy_ask_amount;

//                     sell_orders_matched.push(sell_order);
//                 }
//             }
//         } else {
//             break;
//         }
//     }

//     buy_order.fill_order(total_ask_filled, total_offer_filled)?;

//     Ok((buy_order, sell_orders_matched))
// }

pub fn matching_order(
    deps: &DepsMut,
    orderbook_pair: OrderBook,
    order: &Order,
    order_price: Decimal,
) -> StdResult<(OrderWithFee, Vec<OrderWithFee>)> {
    let pair_key = &orderbook_pair.get_pair_key();
    let matched_orders_direction = match order.direction {
        OrderDirection::Buy => OrderDirection::Sell,
        OrderDirection::Sell => OrderDirection::Buy,
    };

    let positions_bucket: ReadonlyBucket<u64> = ReadonlyBucket::multilevel(
        deps.storage,
        &[PREFIX_TICK, pair_key, matched_orders_direction.as_bytes()],
    );

    let mut user_order = OrderWithFee::from_order(order.to_owned());
    let mut orders_matched: Vec<OrderWithFee> = vec![];
    let mut total_offer_filled = Uint128::zero();
    let mut total_ask_filled = Uint128::zero();
    let sort_order = match order.direction {
        OrderDirection::Buy => OrderBy::Ascending,
        OrderDirection::Sell => OrderBy::Descending,
    };

    let mut cursor = positions_bucket.range(None, None, sort_order);
    loop {
        if let Some(Ok((k, _))) = cursor.next() {
            let match_price =
                Decimal::raw(u128::from_be_bytes(k.try_into().map_err(|_| {
                    StdError::generic_err("Error converting bytes to u128")
                })?));

            match order.direction {
                OrderDirection::Buy => {
                    if order_price < match_price {
                        break;
                    }
                }
                OrderDirection::Sell => {
                    if order_price > match_price {
                        break;
                    }
                }
            }

            if user_order.will_fulfilled(total_ask_filled, total_offer_filled) {
                break;
            }

            if let Some(orders) = orderbook_pair.query_orders_by_price_and_direction(
                deps.as_ref().storage,
                match_price,
                matched_orders_direction,
                None,
            ) {
                if orders.len() == 0 {
                    continue;
                }

                let match_orders_with_fees = OrderWithFee::from_orders(orders);

                for mut match_order in match_orders_with_fees {
                    if user_order.will_fulfilled(total_ask_filled, total_offer_filled) {
                        break;
                    }
                    // remaining ask & offer of buy order
                    let lef_user_ask = user_order.ask_amount.checked_sub(total_ask_filled)?;
                    let lef_user_offer = user_order.offer_amount.checked_sub(total_offer_filled)?;

                    // remaining offer & ask of sell order
                    let lef_match_offer = match_order
                        .offer_amount
                        .checked_sub(match_order.filled_offer_amount)?;
                    let lef_match_ask = match_order
                        .ask_amount
                        .checked_sub(match_order.filled_ask_amount)?;

                    // ask_amount of user_order <= min(lef_buy_ask, lef_sell_offer)
                    let mut user_ask_amount = Uint128::min(lef_user_ask, lef_match_offer);
                    let mut user_offer_amount = match order.direction {
                        OrderDirection::Buy => user_ask_amount * match_price,
                        OrderDirection::Sell => {
                            Uint128::from(user_ask_amount * Decimal::one().atomics())
                                .checked_div(match_price.atomics())?
                        }
                    };

                    // if sell_offer_amount > lef_sell_offer, we need re calc ask_amount
                    if user_offer_amount > lef_user_offer {
                        user_offer_amount = lef_user_offer;
                        user_ask_amount = match order.direction {
                            OrderDirection::Buy => {
                                Uint128::from(user_offer_amount * Decimal::one().atomics())
                                    .checked_div(match_price.atomics())?
                            }
                            OrderDirection::Sell => user_offer_amount * match_price,
                        }
                    }

                    // ask_amount receive of sell order = min(actual, expect)
                    let match_ask_amount = Uint128::min(user_offer_amount, lef_match_ask);

                    match_order.fill_order(match_ask_amount, user_ask_amount)?;

                    total_offer_filled += user_offer_amount;
                    total_ask_filled += user_ask_amount;

                    orders_matched.push(match_order);
                }
            }
        } else {
            break;
        }
    }

    user_order.fill_order(total_ask_filled, total_offer_filled)?;

    Ok((user_order, orders_matched))
}

// pub fn matching_sell_order(
//     deps: &DepsMut,
//     orderbook_pair: OrderBook,
//     order: &Order,
//     min_sell_price: Decimal,
// ) -> StdResult<(OrderWithFee, Vec<OrderWithFee>)> {
//     let pair_key = &orderbook_pair.get_pair_key();

//     let buy_position_bucket: ReadonlyBucket<u64> = ReadonlyBucket::multilevel(
//         deps.storage,
//         &[PREFIX_TICK, pair_key, OrderDirection::Buy.as_bytes()],
//     );

//     let mut sell_order = OrderWithFee::from_order(order.to_owned());
//     let mut buy_orders_matched: Vec<OrderWithFee> = vec![];

//     let mut buy_cursor = buy_position_bucket.range(None, None, OrderBy::Descending);
//     let mut total_offer_filled = Uint128::zero();
//     let mut total_ask_filled = Uint128::zero();

//     loop {
//         if let Some(Ok((k, _))) = buy_cursor.next() {
//             let buy_price =
//                 Decimal::raw(u128::from_be_bytes(k.try_into().map_err(|_| {
//                     StdError::generic_err("Error converting bytes to u128")
//                 })?));

//             if min_sell_price > buy_price {
//                 break;
//             }

//             if let Some(orders) = orderbook_pair.query_orders_by_price_and_direction(
//                 deps.as_ref().storage,
//                 buy_price,
//                 OrderDirection::Buy,
//                 None,
//             ) {
//                 if orders.len() == 0 {
//                     continue;
//                 }

//                 let buy_orders_with_fee = OrderWithFee::from_orders(orders);

//                 let match_price = buy_price;

//                 for mut buy_order in buy_orders_with_fee {
//                     // remaining ask & offer of sell order
//                     let lef_sell_ask = sell_order.ask_amount.checked_sub(total_ask_filled)?;
//                     let lef_sell_offer = sell_order.offer_amount.checked_sub(total_offer_filled)?;

//                     // remaining offer & ask of buy order
//                     let lef_buy_offer = buy_order
//                         .offer_amount
//                         .checked_sub(buy_order.filled_offer_amount)?;
//                     let lef_buy_ask = buy_order
//                         .ask_amount
//                         .checked_sub(buy_order.filled_ask_amount)?;

//                     // ask_amount of sell order <= min(lef_sell_ask, lef_buy_offer)
//                     let mut sell_ask_amount = Uint128::min(lef_sell_ask, lef_buy_offer);
//                     let mut sell_offer_amount =
//                         Uint128::from(sell_ask_amount * Decimal::one().atomics())
//                             .checked_div(match_price.atomics())?;
//                     // if sell_offer_amount > lef_sell_offer, we need re calc ask_amount
//                     if sell_offer_amount > lef_sell_offer {
//                         sell_offer_amount = lef_sell_offer;
//                         sell_ask_amount = sell_offer_amount * match_price;
//                     }

//                     // ask_amount receive of buy order = min(actual, expect)
//                     let buy_ask_amount = Uint128::min(sell_offer_amount, lef_buy_ask);

//                     buy_order.fill_order(buy_ask_amount, sell_ask_amount)?;

//                     total_offer_filled += sell_offer_amount;
//                     total_ask_filled += sell_ask_amount;

//                     buy_orders_matched.push(buy_order);

//                     if sell_order.is_fulfilled() {
//                         break;
//                     }
//                 }
//                 if sell_order.is_fulfilled() {
//                     break;
//                 }
//             }
//         } else {
//             break;
//         }
//     }

//     sell_order.fill_order(total_ask_filled, total_offer_filled)?;

//     Ok((sell_order, buy_orders_matched))
// }

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
    let mut matched_events: Vec<Event> = vec![];
    let mut total_reward: Vec<String> = Vec::new();

    let order = read_order(deps.storage, &pair_key, order_id)?;

    let (offer_order_with_fee, mut matched_orders) =
        matching_order(&deps, orderbook_pair.clone(), &order, price_threshold)?;

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
            order_matched.match_order(deps.storage, &pair_key)?;
            matched_events.push(to_events(
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
            ("total_matched_orders", &matched_events.len().to_string()),
            ("executor_reward", &format!("{:?}", &total_reward)),
        ])
        .add_events(matched_events))
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
