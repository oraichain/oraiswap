use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{from_binary, to_binary, Coin, CosmosMsg, Decimal, Uint128, WasmMsg};
use oraiswap::error::ContractError;

use crate::contract::{handle, init, query};
use crate::operations::assert_operations;
use crate::testing::mock_querier::mock_dependencies;

use cw20::{Cw20HandleMsg, Cw20ReceiveMsg};
use oraiswap::asset::{Asset, AssetInfo, ATOM_DENOM, ORAI_DENOM};
use oraiswap::pair::HandleMsg as PairHandleMsg;
use oraiswap::router::{
    ConfigResponse, Cw20HookMsg, HandleMsg, InitMsg, QueryMsg, SimulateSwapOperationsResponse,
    SwapOperation,
};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        factory_addr: "oraiswapfactory".into(),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

    // it worked, let's query the state
    let config: ConfigResponse =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!("oraiswapfactory", config.factory_addr.as_str());
}

#[test]
fn simulate_swap_operations_test() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        factory_addr: "oraiswapfactory".into(),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

    // set tax rate as 5%
    deps.querier.with_tax(
        Decimal::permille(3),
        &[
            (&ORAI_DENOM.to_string(), &Uint128::from(10000000u128)),
            (&ATOM_DENOM.to_string(), &Uint128::from(10000000u128)),
        ],
    );

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

    deps.querier.with_oraiswap_pairs(&[(
        &format!("{}{}", ORAI_DENOM.to_string(), ATOM_DENOM.to_string()),
        &ATOM_DENOM.to_string(),
    )]);

    let _res: SimulateSwapOperationsResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
}

#[test]
fn handle_swap_operations() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_token_balances(&[(
        &"asset0002".to_string(),
        &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(1000000u128))],
    )]);

    let msg = InitMsg {
        factory_addr: "oraiswapfactory".into(),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = HandleMsg::ExecuteSwapOperations {
        operations: vec![],
        minimum_receive: None,
        to: None,
    };

    let info = mock_info("addr0000", &[]);
    let res = handle(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(err) => assert_eq!(err, ContractError::NoSwapOperation {}),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let msg = HandleMsg::ExecuteSwapOperations {
        operations: vec![
            SwapOperation::OraiSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: "ukrw".to_string(),
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: "asset0001".into(),
                },
            },
            SwapOperation::OraiSwap {
                offer_asset_info: AssetInfo::Token {
                    contract_addr: "asset0001".into(),
                },
                ask_asset_info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
            },
            SwapOperation::OraiSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: "asset0002".into(),
                },
            },
        ],
        minimum_receive: Some(Uint128::from(1000000u128)),
        to: None,
    };

    let info = mock_info("addr0000", &[]);
    let res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.into(),
                send: vec![],
                msg: to_binary(&HandleMsg::ExecuteSwapOperation {
                    operation: SwapOperation::OraiSwap {
                        offer_asset_info: AssetInfo::NativeToken {
                            denom: "ukrw".to_string(),
                        },
                        ask_asset_info: AssetInfo::Token {
                            contract_addr: "asset0001".into(),
                        },
                    },
                    to: None,
                })
                .unwrap(),
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.into(),
                send: vec![],
                msg: to_binary(&HandleMsg::ExecuteSwapOperation {
                    operation: SwapOperation::OraiSwap {
                        offer_asset_info: AssetInfo::Token {
                            contract_addr: "asset0001".into(),
                        },
                        ask_asset_info: AssetInfo::NativeToken {
                            denom: ORAI_DENOM.to_string(),
                        },
                    },
                    to: None,
                })
                .unwrap(),
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.into(),
                send: vec![],
                msg: to_binary(&HandleMsg::ExecuteSwapOperation {
                    operation: SwapOperation::OraiSwap {
                        offer_asset_info: AssetInfo::NativeToken {
                            denom: ORAI_DENOM.to_string(),
                        },
                        ask_asset_info: AssetInfo::Token {
                            contract_addr: "asset0002".into(),
                        },
                    },
                    to: Some("addr0000".into()),
                })
                .unwrap(),
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.into(),
                send: vec![],
                msg: to_binary(&HandleMsg::AssertMinimumReceive {
                    asset_info: AssetInfo::Token {
                        contract_addr: "asset0002".into(),
                    },
                    prev_balance: Uint128::zero(),
                    minimum_receive: Uint128::from(1000000u128),
                    receiver: "addr0000".into(),
                })
                .unwrap(),
            }),
        ]
    );

    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: "addr0000".into(),
        amount: Uint128::from(1000000u128),
        msg: to_binary(&Cw20HookMsg::ExecuteSwapOperations {
            operations: vec![
                SwapOperation::OraiSwap {
                    offer_asset_info: AssetInfo::NativeToken {
                        denom: "ukrw".to_string(),
                    },
                    ask_asset_info: AssetInfo::Token {
                        contract_addr: "asset0001".into(),
                    },
                },
                SwapOperation::OraiSwap {
                    offer_asset_info: AssetInfo::Token {
                        contract_addr: "asset0001".into(),
                    },
                    ask_asset_info: AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                },
                SwapOperation::OraiSwap {
                    offer_asset_info: AssetInfo::NativeToken {
                        denom: ORAI_DENOM.to_string(),
                    },
                    ask_asset_info: AssetInfo::Token {
                        contract_addr: "asset0002".into(),
                    },
                },
            ],
            minimum_receive: None,
            to: Some("addr0002".into()),
        })
        .ok(),
    });

    let info = mock_info("asset0000", &[]);
    let res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.into(),
                send: vec![],
                msg: to_binary(&HandleMsg::ExecuteSwapOperation {
                    operation: SwapOperation::OraiSwap {
                        offer_asset_info: AssetInfo::NativeToken {
                            denom: "ukrw".to_string(),
                        },
                        ask_asset_info: AssetInfo::Token {
                            contract_addr: "asset0001".into(),
                        },
                    },
                    to: None,
                })
                .unwrap(),
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.into(),
                send: vec![],
                msg: to_binary(&HandleMsg::ExecuteSwapOperation {
                    operation: SwapOperation::OraiSwap {
                        offer_asset_info: AssetInfo::Token {
                            contract_addr: "asset0001".into(),
                        },
                        ask_asset_info: AssetInfo::NativeToken {
                            denom: ORAI_DENOM.to_string(),
                        },
                    },
                    to: None,
                })
                .unwrap(),
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.into(),
                send: vec![],
                msg: to_binary(&HandleMsg::ExecuteSwapOperation {
                    operation: SwapOperation::OraiSwap {
                        offer_asset_info: AssetInfo::NativeToken {
                            denom: ORAI_DENOM.to_string(),
                        },
                        ask_asset_info: AssetInfo::Token {
                            contract_addr: "asset0002".into(),
                        },
                    },
                    to: Some("addr0002".into()),
                })
                .unwrap(),
            })
        ]
    );
}

#[test]
fn handle_swap_operation() {
    let mut deps = mock_dependencies(&[]);
    let msg = InitMsg {
        factory_addr: "oraiswapfactory".into(),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

    deps.querier
        .with_oraiswap_pairs(&[(&"uusdasset".to_string(), &"pair".to_string())]);
    deps.querier.with_tax(
        Decimal::percent(5),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );
    deps.querier.with_balance(&[(
        MOCK_CONTRACT_ADDR.to_string(),
        &[Coin {
            amount: Uint128::from(1000000u128),
            denom: "uusd".to_string(),
        }],
    )]);

    deps.querier
        .with_oraiswap_pairs(&[(&"assetuusd".to_string(), &"pair".to_string())]);
    deps.querier.with_token_balances(&[(
        &"asset".to_string(),
        &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(1000000u128))],
    )]);

    let msg = HandleMsg::ExecuteSwapOperation {
        operation: SwapOperation::OraiSwap {
            offer_asset_info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
            ask_asset_info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
        },
        to: Some("addr0000".into()),
    };

    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset".into(),
            send: vec![],
            msg: to_binary(&Cw20HandleMsg::Send {
                contract: "pair".into(),
                amount: Uint128::from(1000000u128),
                msg: to_binary(&PairHandleMsg::Swap {
                    offer_asset: Asset {
                        info: AssetInfo::Token {
                            contract_addr: "asset".into(),
                        },
                        amount: Uint128::from(1000000u128),
                    },
                    belief_price: None,
                    max_spread: None,
                    to: Some("addr0000".into()),
                })
                .ok()
            })
            .unwrap()
        })]
    );
}

#[test]
fn query_buy_with_routes() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        factory_addr: "oraiswapfactory".into(),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

    // set tax rate as 5%
    deps.querier.with_tax(
        Decimal::percent(5),
        &[
            (&"uusd".to_string(), &Uint128::from(1000000u128)),
            (&"ukrw".to_string(), &Uint128::from(1000000u128)),
        ],
    );

    let msg = QueryMsg::SimulateSwapOperations {
        offer_amount: Uint128::from(1000000u128),
        operations: vec![
            SwapOperation::OraiSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: "ukrw".to_string(),
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: "asset0000".into(),
                },
            },
            SwapOperation::OraiSwap {
                offer_asset_info: AssetInfo::Token {
                    contract_addr: "asset0000".into(),
                },
                ask_asset_info: AssetInfo::NativeToken {
                    denom: ORAI_DENOM.to_string(),
                },
            },
        ],
    };

    deps.querier.with_oraiswap_pairs(&[
        (&"ukrwasset0000".to_string(), &"pair0000".to_string()),
        (&"asset0000orai".to_string(), &"pair0001".to_string()),
    ]);

    let res: SimulateSwapOperationsResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(
        res,
        SimulateSwapOperationsResponse {
            amount: Uint128::from(952380u128), // tax charged 1 times uusd => ukrw, ukrw => asset0000, asset0000 => orai
        }
    );
}

#[test]
fn assert_minimum_receive_native_token() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_balance(&[(
        "addr0000".to_string(),
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    )]);

    let info = mock_info("addr0000", &[]);
    // success
    let msg = HandleMsg::AssertMinimumReceive {
        asset_info: AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        prev_balance: Uint128::zero(),
        minimum_receive: Uint128::from(1000000u128),
        receiver: "addr0000".into(),
    };
    let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // assertion failed; native token
    let msg = HandleMsg::AssertMinimumReceive {
        asset_info: AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        prev_balance: Uint128::zero(),
        minimum_receive: Uint128::from(1000001u128),
        receiver: "addr0000".into(),
    };
    let res = handle(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(err) => assert_eq!(
            err,
            ContractError::SwapAssertionFailure {
                minium_receive: 1000001u128.into(),
                swap_amount: 1000000u128.into()
            }
        ),
        _ => panic!("DO NOT ENTER HERE"),
    }
}

#[test]
fn assert_minimum_receive_token() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_token_balances(&[(
        &"token0000".to_string(),
        &[(&"addr0000".to_string(), &Uint128::from(1000000u128))],
    )]);

    let info = mock_info("addr0000", &[]);
    // success
    let msg = HandleMsg::AssertMinimumReceive {
        asset_info: AssetInfo::Token {
            contract_addr: "token0000".into(),
        },
        prev_balance: Uint128::zero(),
        minimum_receive: Uint128::from(1000000u128),
        receiver: "addr0000".into(),
    };
    let _res = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // assertion failed; native token
    let msg = HandleMsg::AssertMinimumReceive {
        asset_info: AssetInfo::Token {
            contract_addr: "token0000".into(),
        },
        prev_balance: Uint128::zero(),
        minimum_receive: Uint128::from(1000001u128),
        receiver: "addr0000".into(),
    };
    let res = handle(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(err) => assert_eq!(
            err,
            ContractError::SwapAssertionFailure {
                minium_receive: 1000001u128.into(),
                swap_amount: 1000000u128.into()
            }
        ),
        _ => panic!("DO NOT ENTER HERE"),
    }
}

#[test]
fn test_invalid_operations() {
    // empty error
    assert!(assert_operations(&[]).is_err());

    // orai output
    assert!(assert_operations(&vec![
        SwapOperation::OraiSwap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: "ukrw".to_string(),
            },
            ask_asset_info: AssetInfo::Token {
                contract_addr: "asset0001".into(),
            },
        },
        SwapOperation::OraiSwap {
            offer_asset_info: AssetInfo::Token {
                contract_addr: "asset0001".into(),
            },
            ask_asset_info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
        }
    ])
    .is_ok());

    // asset0002 output
    assert!(assert_operations(&vec![
        SwapOperation::OraiSwap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: "ukrw".to_string(),
            },
            ask_asset_info: AssetInfo::Token {
                contract_addr: "asset0001".into(),
            },
        },
        SwapOperation::OraiSwap {
            offer_asset_info: AssetInfo::Token {
                contract_addr: "asset0001".into(),
            },
            ask_asset_info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
        },
        SwapOperation::OraiSwap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
            ask_asset_info: AssetInfo::Token {
                contract_addr: "asset0002".into(),
            },
        },
    ])
    .is_ok());

    // multiple output token types error
    assert!(assert_operations(&vec![
        SwapOperation::OraiSwap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: "ukrw".to_string(),
            },
            ask_asset_info: AssetInfo::Token {
                contract_addr: "asset0001".into(),
            },
        },
        SwapOperation::OraiSwap {
            offer_asset_info: AssetInfo::Token {
                contract_addr: "asset0001".into(),
            },
            ask_asset_info: AssetInfo::NativeToken {
                denom: "uaud".to_string(),
            },
        },
        SwapOperation::OraiSwap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
            ask_asset_info: AssetInfo::Token {
                contract_addr: "asset0002".into(),
            },
        },
    ])
    .is_err());
}
