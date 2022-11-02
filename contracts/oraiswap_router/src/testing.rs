use cosmwasm_std::{Coin, Decimal, Uint128};
use oraiswap::asset::{Asset, AssetInfo, ORAI_DENOM};
use oraiswap::router::{
    ExecuteMsg, InstantiateMsg, QueryMsg, SimulateSwapOperationsResponse, SwapOperation,
};

use oraiswap::mock_app::{MockApp, ATOM_DENOM};

#[test]
fn simulate_swap_operations_test() {
    let mut app = MockApp::new();

    app.set_balance(
        "addr0000".into(),
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
    );

    app.set_oracle_contract(oraiswap_oracle::testutils::contract());

    app.set_token_contract(oraiswap_token::testutils::contract());

    app.set_factory_and_pair_contract(
        oraiswap_factory::testutils::contract(),
        oraiswap_pair::testutils::contract(),
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
    let pair_addr = app.set_pair(asset_infos.clone()).unwrap();

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
            "addr0000".into(),
            pair_addr,
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
    };

    let code_id = app.upload(crate::testutils::contract());

    // we can just call .unwrap() to assert this was a success
    let router_addr = app
        .instantiate(code_id, "addr0000".into(), &msg, &[], "router")
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
fn handle_swap_operations() {
    let mut app = MockApp::new();

    app.set_oracle_contract(oraiswap_oracle::testutils::contract());

    app.set_token_contract(oraiswap_token::testutils::contract());

    app.set_factory_and_pair_contract(
        oraiswap_factory::testutils::contract(),
        oraiswap_pair::testutils::contract(),
    );

    // set tax rate as 0.3%
    app.set_tax(
        Decimal::permille(3),
        &[
            (&ORAI_DENOM.to_string(), &Uint128::from(10000000u128)),
            (&ATOM_DENOM.to_string(), &Uint128::from(10000000u128)),
        ],
    );

    app.set_balance(
        "addr0000".into(),
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
    let pair_addr1 = app.set_pair(asset_infos1.clone()).unwrap();
    let pair_addr2 = app.set_pair(asset_infos2.clone()).unwrap();

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
        "addr0000".into(),
        asset_addr.clone(),
        &oraiswap_token::msg::ExecuteMsg::IncreaseAllowance {
            spender: pair_addr1.clone(),
            amount: Uint128::from(100u128),
            expires: None,
        },
        &[],
    )
    .unwrap();

    let _res = app
        .execute(
            "addr0000".into(),
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
        "addr0000".into(),
        asset_addr.clone(),
        &oraiswap_token::msg::ExecuteMsg::IncreaseAllowance {
            spender: pair_addr2.clone(),
            amount: Uint128::from(100u128),
            expires: None,
        },
        &[],
    )
    .unwrap();

    let _res = app
        .execute(
            "addr0000".into(),
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
    };

    let code_id = app.upload(crate::testutils::contract());

    // we can just call .unwrap() to assert this was a success
    let router_addr = app
        .instantiate(code_id, "addr0000".into(), &msg, &[], "router")
        .unwrap();

    let msg = ExecuteMsg::ExecuteSwapOperations {
        operations: vec![],
        minimum_receive: None,
        to: None,
    };

    let res = app.execute("addr0000".into(), router_addr.clone(), &msg, &[]);
    match res {
        Err(err) => assert_eq!(err, "must provide operations"),
        _ => panic!("DO NOT ENTER HERE"),
    }

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
            "addr0000".into(),
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

    println!("{:?}", res.attributes);
}
