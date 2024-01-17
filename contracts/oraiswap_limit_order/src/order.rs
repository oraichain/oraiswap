use std::str::FromStr;

use crate::orderbook::{BulkOrders, Executor, Order, OrderBook, OrderWithFee};
use crate::state::{
    increase_last_order_id, read_config, read_order, read_orderbook, read_reward, remove_order,
    remove_orderbook, store_order, store_reward, DEFAULT_LIMIT, MAX_LIMIT, PREFIX_TICK,
};
use cosmwasm_std::{
    attr, Addr, Api, Attribute, CanonicalAddr, CosmosMsg, Decimal, DepsMut, Event, MessageInfo,
    Order as OrderBy, Response, StdError, StdResult, Storage, Uint128,
};

use cosmwasm_storage::ReadonlyBucket;
use oraiswap::asset::{pair_key, Asset, AssetInfo};
use oraiswap::error::ContractError;
use oraiswap::limit_order::{OrderDirection, OrderStatus};

const RELAY_FEE: u128 = 300u128;

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
    let mut ask_amount = assets[1].amount;

    let (highest_buy_price, buy_found, _) =
        orderbook_pair.highest_price(deps.storage, OrderDirection::Buy);
    let (lowest_sell_price, sell_found, _) =
        orderbook_pair.lowest_price(deps.storage, OrderDirection::Sell);

    // check spread for submit order
    if let Some(spread) = orderbook_pair.spread {
        if spread >= Decimal::one() {
            return Err(ContractError::SlippageMustLessThanOne { slippage: spread });
        }
        let buy_spread_factor = Decimal::one() - spread;
        let sell_spread_factor = Decimal::one() + spread;
        if buy_found && sell_found {
            match direction {
                OrderDirection::Buy => {
                    let mut price = Decimal::from_ratio(offer_amount, ask_amount);
                    let spread_price = lowest_sell_price * sell_spread_factor;
                    if price.ge(&(spread_price)) {
                        price = spread_price;
                        ask_amount = Uint128::from(offer_amount * Decimal::one().atomics())
                            .checked_div(price.atomics())
                            .unwrap_or_default();
                    }
                }
                OrderDirection::Sell => {
                    let mut price = Decimal::from_ratio(ask_amount, offer_amount);
                    let spread_price = highest_buy_price * buy_spread_factor;
                    if spread_price.is_zero() {
                        return Err(ContractError::PriceMustNotBeZero {
                            price: spread_price,
                        });
                    }
                    if spread_price.ge(&price) {
                        price = spread_price;
                        ask_amount = Uint128::from(offer_amount * price);
                    }
                }
            };
        }
    }

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

    Ok(Response::new().add_attributes(vec![
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
    deps: DepsMut,
    orderbook_pair: &OrderBook,
    sender: Addr,
    direction: OrderDirection,
    assets: [Asset; 2],
    refund_amount: Uint128,
) -> Result<Response, ContractError> {
    assets[0].assert_if_asset_is_zero()?;
    assets[1].assert_if_asset_is_zero()?;
    let order_id = increase_last_order_id(deps.storage)?;
    store_order(
        deps.storage,
        &orderbook_pair.get_pair_key(),
        &Order {
            order_id,
            direction,
            bidder_addr: deps.api.addr_canonicalize(sender.as_str())?,
            offer_amount: assets[0].amount,
            ask_amount: assets[1].amount,
            filled_offer_amount: Uint128::zero(),
            filled_ask_amount: Uint128::zero(),
            status: OrderStatus::Open,
        },
        true,
    )?;

    // prepare refund message
    let bidder_refund = Asset {
        info: match direction {
            OrderDirection::Buy => orderbook_pair.quote_coin_info.to_normal(deps.api)?,
            OrderDirection::Sell => orderbook_pair.base_coin_info.to_normal(deps.api)?,
        },
        amount: refund_amount,
    };

    // Build refund msg
    let messages = if refund_amount > Uint128::zero() {
        vec![bidder_refund
            .clone()
            .into_msg(None, &deps.querier, sender.clone())?]
    } else {
        vec![]
    };

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("action", "submit_order"),
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

        if sell_ask_amount.is_zero() || sell_offer_amount.is_zero() {
            continue;
        }

        sell_bulk_orders.filled_volume += sell_offer_amount;
        sell_bulk_orders.filled_ask_volume += sell_ask_amount;

        buy_bulk_orders.filled_volume += sell_ask_amount;
        buy_bulk_orders.filled_ask_volume += sell_offer_amount;

        buy_bulk_orders.volume = buy_bulk_orders.volume.checked_sub(sell_ask_amount)?;
        sell_bulk_orders.volume = sell_bulk_orders.volume.checked_sub(sell_offer_amount)?;

        if buy_bulk_orders.filled_ask_volume >= buy_bulk_orders.ask_volume {
            buy_bulk_orders.spread_volume = buy_bulk_orders
                .filled_ask_volume
                .checked_sub(buy_bulk_orders.ask_volume)?;
            buy_bulk_orders.filled_ask_volume = buy_bulk_orders
                .filled_ask_volume
                .checked_sub(buy_bulk_orders.spread_volume)?;
            buy_bulk_orders.ask_volume = Uint128::zero();
        }
        if sell_bulk_orders.filled_ask_volume >= sell_bulk_orders.ask_volume {
            sell_bulk_orders.spread_volume = sell_bulk_orders
                .filled_ask_volume
                .checked_sub(sell_bulk_orders.ask_volume)?;
            sell_bulk_orders.filled_ask_volume = sell_bulk_orders
                .filled_ask_volume
                .checked_sub(sell_bulk_orders.spread_volume)?;
            sell_bulk_orders.ask_volume = Uint128::zero();
        }

        if buy_bulk_orders.volume <= min_vol {
            // buy out
            buy_bulk_orders.ask_volume = Uint128::zero();
            i += 1;
        }
        if sell_bulk_orders.volume <= min_vol {
            // sell out
            sell_bulk_orders.ask_volume = Uint128::zero();
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

            if filled_offer.is_zero() || filled_ask.is_zero() {
                continue;
            }

            bulk.filled_volume = bulk
                .filled_volume
                .checked_sub(filled_offer)
                .unwrap_or_default();
            bulk.filled_ask_volume = bulk
                .filled_ask_volume
                .checked_sub(filled_ask)
                .unwrap_or_default();

            order.fill_order(filled_ask, filled_offer);

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
    slippage: Option<Decimal>,
) -> StdResult<([Asset; 2], Asset)> {
    let slippage_price = if let Some(slippage) = slippage {
        if slippage >= Decimal::one() {
            return Err(StdError::generic_err(
                ContractError::SlippageMustLessThanOne { slippage }.to_string(),
            ));
        }
        match direction {
            OrderDirection::Buy => market_price * (Decimal::one() + slippage),
            OrderDirection::Sell => market_price * (Decimal::one() - slippage),
        }
    } else {
        market_price
    };
    let quote_amount = Uint128::from(base_amount * slippage_price);
    let quote_asset = Asset {
        info: orderbook_pair.quote_coin_info.to_normal(api)?,
        amount: quote_amount,
    };
    let paid_assets = match direction {
        OrderDirection::Buy => [
            Asset {
                info: orderbook_pair.quote_coin_info.to_normal(api)?,
                amount: quote_amount,
            },
            Asset {
                info: orderbook_pair.base_coin_info.to_normal(api)?,
                amount: base_amount,
            },
        ],
        OrderDirection::Sell => [
            Asset {
                info: orderbook_pair.base_coin_info.to_normal(api)?,
                amount: base_amount,
            },
            Asset {
                info: orderbook_pair.quote_coin_info.to_normal(api)?,
                amount: quote_amount,
            },
        ],
    };
    Ok((paid_assets, quote_asset))
}
