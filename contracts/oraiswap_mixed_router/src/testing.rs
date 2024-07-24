use cosmwasm_std::{to_json_binary, Addr, Coin, Decimal, Uint128};
use cw20::Cw20ExecuteMsg;
use oraiswap::asset::{Asset, AssetInfo, ORAI_DENOM};
use oraiswap::create_entry_points_testing;
use oraiswap::mixed_router::{
    ExecuteMsg, InstantiateMsg, QueryMsg, SimulateSwapOperationsResponse, SwapOperation,
};

use oraiswap::testing::{MockApp, APP_OWNER, ATOM_DENOM};
use oraiswap_v3::liquidity::Liquidity;
use oraiswap_v3::msg::ExecuteMsg as OraiswapV3ExecuteMsg;
use oraiswap_v3::percentage::Percentage;
use oraiswap_v3::sqrt_price::{calculate_sqrt_price, SqrtPrice};
use oraiswap_v3::{FeeTier, PoolKey, MAX_TICK, MIN_TICK};
#[test]
fn simulate_swap_operations_test() {
    let mut app = MockApp::new(&[(
        &"addr0000".to_string(),
        &[
            Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(1000u128),
            },
            Coin {
                denom: ATOM_DENOM.to_string(),
                amount: Uint128::from(1000u128),
            },
        ],
    )]);

    app.set_oracle_contract(Box::new(create_entry_points_testing!(oraiswap_oracle)));

    app.set_token_contract(Box::new(create_entry_points_testing!(oraiswap_token)));

    app.set_factory_and_pair_contract(
        Box::new(
            create_entry_points_testing!(oraiswap_factory)
                .with_reply(oraiswap_factory::contract::reply),
        ),
        Box::new(
            create_entry_points_testing!(oraiswap_pair).with_reply(oraiswap_pair::contract::reply),
        ),
    );

    // set tax rate as 0.3%
    app.set_tax(
        Decimal::permille(3),
        &[
            (&ORAI_DENOM.to_string(), &Uint128::from(10000000u128)),
            (&ATOM_DENOM.to_string(), &Uint128::from(10000000u128)),
        ],
    );

    let asset_infos = [
        AssetInfo::NativeToken {
            denom: ORAI_DENOM.to_string(),
        },
        AssetInfo::NativeToken {
            denom: ATOM_DENOM.to_string(),
        },
    ];

    // create pair
    let pair_addr = app.create_pair(asset_infos.clone()).unwrap();

    // provide liquidity
    // successfully provide liquidity for the exist pool
    let msg = oraiswap::pair::ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ATOM_DENOM.to_string(),
                },
                amount: Uint128::from(100u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(100u128),
            },
        ],
        slippage_tolerance: None,
        receiver: None,
    };

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            pair_addr.clone(),
            &msg,
            &[
                Coin {
                    denom: ORAI_DENOM.to_string(),
                    amount: Uint128::from(100u128),
                },
                Coin {
                    denom: ATOM_DENOM.to_string(),
                    amount: Uint128::from(100u128),
                },
            ],
        )
        .unwrap();

    let msg = InstantiateMsg {
        factory_addr: app.factory_addr.clone(),
        factory_addr_v2: Addr::unchecked("addr0000_v2"),
        oraiswap_v3: Addr::unchecked("oraiswap_v3"),
    };

    let code_id = app.upload(Box::new(create_entry_points_testing!(crate)));

    // we can just call .unwrap() to assert this was a success
    let router_addr = app
        .instantiate(code_id, Addr::unchecked("addr0000"), &msg, &[], "router")
        .unwrap();

    let msg = QueryMsg::SimulateSwapOperations {
        offer_amount: Uint128::from(100u128),
        operations: vec![SwapOperation::OraiSwap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
            ask_asset_info: AssetInfo::NativeToken {
                denom: ATOM_DENOM.to_string(),
            },
        }],
    };

    let res: SimulateSwapOperationsResponse = app.query(router_addr, &msg).unwrap();
    println!("{:?}", res);
}

#[test]
fn execute_swap_operations() {
    let mut app = MockApp::new(&[(
        &"addr0000".to_string(),
        &[
            Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(1000u128),
            },
            Coin {
                denom: ATOM_DENOM.to_string(),
                amount: Uint128::from(1000u128),
            },
        ],
    )]);

    app.set_oracle_contract(Box::new(create_entry_points_testing!(oraiswap_oracle)));

    app.set_token_contract(Box::new(create_entry_points_testing!(oraiswap_token)));

    app.set_factory_and_pair_contract(
        Box::new(
            create_entry_points_testing!(oraiswap_factory)
                .with_reply(oraiswap_factory::contract::reply),
        ),
        Box::new(
            create_entry_points_testing!(oraiswap_pair).with_reply(oraiswap_pair::contract::reply),
        ),
    );
    // set tax rate as 0.3%
    app.set_tax(
        Decimal::permille(3),
        &[
            (&ORAI_DENOM.to_string(), &Uint128::from(10000000u128)),
            (&ATOM_DENOM.to_string(), &Uint128::from(10000000u128)),
        ],
    );

    let asset_addr = app.create_token("asset");

    app.set_token_balances(&[(
        &"asset".to_string(),
        &[(&"addr0000".to_string(), &Uint128::from(1000000u128))],
    )]);

    let asset_infos1 = [
        AssetInfo::NativeToken {
            denom: ORAI_DENOM.to_string(),
        },
        AssetInfo::Token {
            contract_addr: asset_addr.clone(),
        },
    ];

    let asset_infos2 = [
        AssetInfo::NativeToken {
            denom: ATOM_DENOM.to_string(),
        },
        AssetInfo::Token {
            contract_addr: asset_addr.clone(),
        },
    ];

    // create pair
    let pair_addr1 = app.create_pair(asset_infos1.clone()).unwrap();
    let pair_addr2 = app.create_pair(asset_infos2.clone()).unwrap();

    // provide liquidity
    let msg = oraiswap::pair::ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                amount: Uint128::from(100u128),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: asset_addr.clone(),
                },
                amount: Uint128::from(100u128),
            },
        ],
        slippage_tolerance: None,
        receiver: None,
    };

    // set allowance
    app.execute(
        Addr::unchecked("addr0000"),
        asset_addr.clone(),
        &cw20::Cw20ExecuteMsg::IncreaseAllowance {
            spender: pair_addr1.to_string(),
            amount: Uint128::from(100u128),
            expires: None,
        },
        &[],
    )
    .unwrap();

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            pair_addr1.clone(),
            &msg,
            &[Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(100u128),
            }],
        )
        .unwrap();

    let msg = oraiswap::pair::ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::NativeToken {
                    denom: ATOM_DENOM.to_string(),
                },
                amount: Uint128::from(100u128),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: asset_addr.clone(),
                },
                amount: Uint128::from(100u128),
            },
        ],
        slippage_tolerance: None,
        receiver: None,
    };

    // set allowance
    app.execute(
        Addr::unchecked("addr0000"),
        asset_addr.clone(),
        &cw20::Cw20ExecuteMsg::IncreaseAllowance {
            spender: pair_addr2.to_string(),
            amount: Uint128::from(100u128),
            expires: None,
        },
        &[],
    )
    .unwrap();

    let _res = app
        .execute(
            Addr::unchecked("addr0000"),
            pair_addr2.clone(),
            &msg,
            &[Coin {
                denom: ATOM_DENOM.to_string(),
                amount: Uint128::from(100u128),
            }],
        )
        .unwrap();

    let msg = InstantiateMsg {
        factory_addr: app.factory_addr.clone(),
        factory_addr_v2: Addr::unchecked("addr0000_v2"),
        oraiswap_v3: Addr::unchecked("oraiswap_v3"),
    };

    let code_id = app.upload(Box::new(create_entry_points_testing!(crate)));

    // we can just call .unwrap() to assert this was a success
    let router_addr = app
        .instantiate(code_id, Addr::unchecked("addr0000"), &msg, &[], "router")
        .unwrap();

    let msg: ExecuteMsg = ExecuteMsg::ExecuteSwapOperations {
        operations: vec![],
        minimum_receive: None,
        to: None,
    };

    let res = app.execute(Addr::unchecked("addr0000"), router_addr.clone(), &msg, &[]);
    app.assert_fail(res);

    let msg = ExecuteMsg::ExecuteSwapOperations {
        operations: vec![
            SwapOperation::OraiSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: asset_addr.clone(),
                },
            },
            SwapOperation::OraiSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: ATOM_DENOM.to_string(),
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: asset_addr.clone(),
                },
            },
        ],
        minimum_receive: None,
        to: None,
    };

    let res = app
        .execute(
            Addr::unchecked("addr0000"),
            router_addr.clone(),
            &msg,
            &[
                Coin {
                    denom: ORAI_DENOM.to_string(),
                    amount: Uint128::from(100u128),
                },
                Coin {
                    denom: ATOM_DENOM.to_string(),
                    amount: Uint128::from(100u128),
                },
            ],
        )
        .unwrap();

    println!("{:?}", res.events);

    // test swap on whitelisted pool

    // enable  whitelisted pool
    let _res = app
        .execute(
            Addr::unchecked("admin"),
            pair_addr1.clone(),
            &oraiswap::pair::ExecuteMsg::EnableWhitelist { status: true },
            &[],
        )
        .unwrap();
    let _res = app
        .execute(
            Addr::unchecked("admin"),
            pair_addr2.clone(),
            &oraiswap::pair::ExecuteMsg::EnableWhitelist { status: true },
            &[],
        )
        .unwrap();

    // whitelist swap router
    app.execute(
        Addr::unchecked("admin"),
        pair_addr1.clone(),
        &oraiswap::pair::ExecuteMsg::RegisterTrader {
            traders: vec![router_addr.clone()],
        },
        &[],
    )
    .unwrap();
    app.execute(
        Addr::unchecked("admin"),
        pair_addr2.clone(),
        &oraiswap::pair::ExecuteMsg::RegisterTrader {
            traders: vec![router_addr.clone()],
        },
        &[],
    )
    .unwrap();

    // swap will be failed

    let res = app.execute(
        Addr::unchecked("addr0000"),
        router_addr.clone(),
        &msg,
        &[
            Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(100u128),
            },
            Coin {
                denom: ATOM_DENOM.to_string(),
                amount: Uint128::from(100u128),
            },
        ],
    );

    app.assert_fail(res);

    // whitelist trader
    app.execute(
        Addr::unchecked("admin"),
        pair_addr1.clone(),
        &oraiswap::pair::ExecuteMsg::RegisterTrader {
            traders: vec![Addr::unchecked("addr0000")],
        },
        &[],
    )
    .unwrap();
    app.execute(
        Addr::unchecked("admin"),
        pair_addr2.clone(),
        &oraiswap::pair::ExecuteMsg::RegisterTrader {
            traders: vec![Addr::unchecked("addr0000")],
        },
        &[],
    )
    .unwrap();

    // swap successfully
    app.execute(
        Addr::unchecked("addr0000"),
        router_addr.clone(),
        &msg,
        &[
            Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(100u128),
            },
            Coin {
                denom: ATOM_DENOM.to_string(),
                amount: Uint128::from(100u128),
            },
        ],
    )
    .unwrap();
}

fn init_v3(
    app: &mut MockApp,
    token_x: Addr,
    token_x_name: &str,
    token_y: Addr,
    token_y_name: &str,
) -> PoolKey {
    app.create_v3(Box::new(create_entry_points_testing!(oraiswap_v3)));

    let protocol_fee = Percentage(100);
    let fee_tier = FeeTier::new(protocol_fee, 10).unwrap();
    app.execute(
        Addr::unchecked(APP_OWNER),
        app.v3_addr.clone(),
        &OraiswapV3ExecuteMsg::AddFeeTier { fee_tier },
        &[],
    )
    .unwrap();

    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();

    app.execute(
        Addr::unchecked(APP_OWNER),
        app.v3_addr.clone(),
        &OraiswapV3ExecuteMsg::CreatePool {
            token_0: token_x.to_string(),
            token_1: token_y.to_string(),
            fee_tier,
            init_sqrt_price,
            init_tick,
        },
        &[],
    )
    .unwrap();

    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();

    let lower_tick_index = -20;
    let middle_tick_index = -10;
    let upper_tick_index = 10;

    let liquidity_delta = Liquidity(1000000000000000000);
    app.approve_token(
        token_x_name,
        "addr0000",
        app.v3_addr.clone().as_str(),
        Uint128::MAX,
    );
    app.approve_token(
        token_y_name,
        "addr0000",
        app.v3_addr.clone().as_str(),
        Uint128::MAX,
    );

    app.execute(
        Addr::unchecked("addr0000"),
        app.v3_addr.clone(),
        &OraiswapV3ExecuteMsg::CreatePosition {
            pool_key: pool_key.clone(),
            lower_tick: lower_tick_index,
            upper_tick: upper_tick_index,
            liquidity_delta,
            slippage_limit_lower: SqrtPrice::from_tick(MIN_TICK).unwrap(),
            slippage_limit_upper: SqrtPrice::from_tick(MAX_TICK).unwrap(),
        },
        &[],
    )
    .unwrap();

    app.execute(
        Addr::unchecked("addr0000"),
        app.v3_addr.clone(),
        &OraiswapV3ExecuteMsg::CreatePosition {
            pool_key: pool_key.clone(),
            lower_tick: lower_tick_index - 20,
            upper_tick: middle_tick_index,
            liquidity_delta,
            slippage_limit_lower: SqrtPrice::from_tick(MIN_TICK).unwrap(),
            slippage_limit_upper: SqrtPrice::from_tick(MAX_TICK).unwrap(),
        },
        &[],
    )
    .unwrap();

    pool_key
}
#[test]
fn simulate_mixed_swap() {
    let mut app = MockApp::new(&[(
        &"addr0000".to_string(),
        &[
            Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(1000000000000u128),
            },
            Coin {
                denom: ATOM_DENOM.to_string(),
                amount: Uint128::from(1000000000000u128),
            },
        ],
    )]);

    app.set_token_contract(Box::new(create_entry_points_testing!(oraiswap_token)));

    let token_x_name = "tokenx";
    let token_y_name = "tokeny";
    let token_x = app.create_token(token_x_name);
    let token_y = app.create_token(token_y_name);

    app.set_token_balances(&[(
        &"tokenx".to_string(),
        &[(&"addr0000".to_string(), &Uint128::from(1000000000000u128))],
    )]);
    app.set_token_balances(&[(
        &"tokeny".to_string(),
        &[(&"addr0000".to_string(), &Uint128::from(1000000000000u128))],
    )]);

    let pool_key = init_v3(
        &mut app,
        token_x.clone(),
        token_x_name,
        token_y.clone(),
        token_y_name,
    );

    app.set_oracle_contract(Box::new(create_entry_points_testing!(oraiswap_oracle)));

    app.set_factory_and_pair_contract(
        Box::new(
            create_entry_points_testing!(oraiswap_factory)
                .with_reply(oraiswap_factory::contract::reply),
        ),
        Box::new(
            create_entry_points_testing!(oraiswap_pair).with_reply(oraiswap_pair::contract::reply),
        ),
    );

    // set tax rate as 0.3%
    app.set_tax(
        Decimal::permille(3),
        &[
            (&ORAI_DENOM.to_string(), &Uint128::from(10000000u128)),
            (&ATOM_DENOM.to_string(), &Uint128::from(10000000u128)),
        ],
    );

    let asset_infos = [
        AssetInfo::Token {
            contract_addr: token_x.clone(),
        },
        AssetInfo::Token {
            contract_addr: token_y.clone(),
        },
    ];

    // create pair
    let pair_addr = app.create_pair(asset_infos.clone()).unwrap();

    // provide liquidity
    // successfully provide liquidity for the exist pool

    app.approve_token(
        token_x_name,
        "addr0000",
        pair_addr.clone().as_str(),
        Uint128::MAX,
    );
    app.approve_token(
        token_y_name,
        "addr0000",
        pair_addr.clone().as_str(),
        Uint128::MAX,
    );
    let msg = oraiswap::pair::ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: token_x.clone(),
                },
                amount: Uint128::from(1000000000u128),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: token_y.clone(),
                },
                amount: Uint128::from(100000000u128),
            },
        ],
        slippage_tolerance: None,
        receiver: None,
    };

    let _res = app
        .execute(Addr::unchecked("addr0000"), pair_addr.clone(), &msg, &[])
        .unwrap();

    let msg = InstantiateMsg {
        factory_addr: app.factory_addr.clone(),
        factory_addr_v2: Addr::unchecked("addr0000_v2"),
        oraiswap_v3: app.v3_addr.clone(),
    };

    let code_id = app.upload(Box::new(create_entry_points_testing!(crate)));

    // we can just call .unwrap() to assert this was a success
    let router_addr = app
        .instantiate(code_id, Addr::unchecked("addr0000"), &msg, &[], "router")
        .unwrap();

    let msg = QueryMsg::SimulateSwapOperations {
        offer_amount: Uint128::from(1000000u128),
        operations: vec![
            SwapOperation::OraiSwap {
                offer_asset_info: AssetInfo::Token {
                    contract_addr: token_x.clone(),
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: token_y.clone(),
                },
            },
            SwapOperation::SwapV3 {
                pool_key,
                x_to_y: false,
            },
        ],
    };

    let res: SimulateSwapOperationsResponse = app.query(router_addr, &msg).unwrap();

    assert_eq!(res.amount, Uint128::new(99599));
}

#[test]
fn execute_mixed_swap_operations() {
    let mut app = MockApp::new(&[(
        &"addr0000".to_string(),
        &[
            Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(1000000000000u128),
            },
            Coin {
                denom: ATOM_DENOM.to_string(),
                amount: Uint128::from(1000000000000u128),
            },
        ],
    )]);

    app.set_token_contract(Box::new(create_entry_points_testing!(oraiswap_token)));

    let token_x_name = "tokenx";
    let token_y_name = "tokeny";
    let token_x = app.create_token(token_x_name);
    let token_y = app.create_token(token_y_name);

    app.set_token_balances(&[(
        &"tokenx".to_string(),
        &[(&"addr0000".to_string(), &Uint128::from(1000000000000u128))],
    )]);
    app.set_token_balances(&[(
        &"tokeny".to_string(),
        &[(&"addr0000".to_string(), &Uint128::from(1000000000000u128))],
    )]);

    let pool_key = init_v3(
        &mut app,
        token_x.clone(),
        token_x_name,
        token_y.clone(),
        token_y_name,
    );

    app.set_oracle_contract(Box::new(create_entry_points_testing!(oraiswap_oracle)));

    app.set_factory_and_pair_contract(
        Box::new(
            create_entry_points_testing!(oraiswap_factory)
                .with_reply(oraiswap_factory::contract::reply),
        ),
        Box::new(
            create_entry_points_testing!(oraiswap_pair).with_reply(oraiswap_pair::contract::reply),
        ),
    );

    // set tax rate as 0.3%
    app.set_tax(
        Decimal::permille(3),
        &[
            (&ORAI_DENOM.to_string(), &Uint128::from(10000000u128)),
            (&ATOM_DENOM.to_string(), &Uint128::from(10000000u128)),
        ],
    );

    let asset_infos = [
        AssetInfo::Token {
            contract_addr: token_x.clone(),
        },
        AssetInfo::Token {
            contract_addr: token_y.clone(),
        },
    ];

    // create pair
    let pair_addr = app.create_pair(asset_infos.clone()).unwrap();

    // provide liquidity
    // successfully provide liquidity for the exist pool

    app.approve_token(
        token_x_name,
        "addr0000",
        pair_addr.clone().as_str(),
        Uint128::MAX,
    );
    app.approve_token(
        token_y_name,
        "addr0000",
        pair_addr.clone().as_str(),
        Uint128::MAX,
    );
    let msg = oraiswap::pair::ExecuteMsg::ProvideLiquidity {
        assets: [
            Asset {
                info: AssetInfo::Token {
                    contract_addr: token_x.clone(),
                },
                amount: Uint128::from(1000000000u128),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: token_y.clone(),
                },
                amount: Uint128::from(100000000u128),
            },
        ],
        slippage_tolerance: None,
        receiver: None,
    };

    let _res = app
        .execute(Addr::unchecked("addr0000"), pair_addr.clone(), &msg, &[])
        .unwrap();

    let msg = InstantiateMsg {
        factory_addr: app.factory_addr.clone(),
        factory_addr_v2: Addr::unchecked("addr0000_v2"),
        oraiswap_v3: app.v3_addr.clone(),
    };

    let code_id = app.upload(Box::new(create_entry_points_testing!(crate)));

    // we can just call .unwrap() to assert this was a success
    let router_addr = app
        .instantiate(code_id, Addr::unchecked("addr0000"), &msg, &[], "router")
        .unwrap();

    // first case: invalid route, can't swap
    let msg_swap = ExecuteMsg::ExecuteSwapOperations {
        operations: vec![
            SwapOperation::OraiSwap {
                offer_asset_info: AssetInfo::Token {
                    contract_addr: token_x.clone(),
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: token_y.clone(),
                },
            },
            SwapOperation::SwapV3 {
                pool_key: pool_key.clone(),
                x_to_y: true,
            },
        ],
        minimum_receive: None,
        to: None,
    };

    let err = app.execute(
        Addr::unchecked("addr0000"),
        token_x.clone(),
        &Cw20ExecuteMsg::Send {
            contract: router_addr.to_string(),
            amount: Uint128::new(1000000),
            msg: to_json_binary(&msg_swap).unwrap(),
        },
        &[],
    );
    app.assert_fail(err);

    // case 2: swap successful
    let msg_swap = ExecuteMsg::ExecuteSwapOperations {
        operations: vec![
            SwapOperation::OraiSwap {
                offer_asset_info: AssetInfo::Token {
                    contract_addr: token_x.clone(),
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: token_y.clone(),
                },
            },
            SwapOperation::SwapV3 {
                pool_key,
                x_to_y: false,
            },
        ],
        minimum_receive: None,
        to: None,
    };

    let res = app
        .execute(
            Addr::unchecked("addr0000"),
            token_x.clone(),
            &Cw20ExecuteMsg::Send {
                contract: router_addr.to_string(),
                amount: Uint128::new(1000000),
                msg: to_json_binary(&msg_swap).unwrap(),
            },
            &[],
        )
        .unwrap();
    println!("{:?}", res.events);
}
