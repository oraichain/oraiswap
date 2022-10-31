use std::ops::{Div, Mul};

use cosmwasm_std::{
    attr, coin, from_binary,
    testing::{mock_env, mock_info},
    to_binary, BankMsg, Binary, CosmosMsg, CustomQuery, Decimal, Empty, HumanAddr, QueryRequest,
    StdError, Uint128, WasmMsg, WasmQuery,
};
use cw20::{Cw20HandleMsg, Cw20ReceiveMsg};
use oraiswap::{
    asset::{Asset, AssetInfo, DECIMAL_FRACTION, ORAI_DENOM},
    converter::{
        ConvertInfoResponse, Cw20HookMsg, HandleMsg, InitMsg, QueryMsg, TokenInfo, TokenRatio,
    },
    mock_app::{mock_dependencies, ATOM_DENOM},
    Decimal256, Uint256,
};

use crate::contract::{handle, init, query};

#[test]
fn test_decimal() {
    let t: Uint256 = Uint256::from(DECIMAL_FRACTION);
    let decimal = Decimal::from_ratio(10u128.pow(18), 10u128.pow(6));
    let denom: Uint256 = t.mul(Decimal256::from(decimal));
    println!("denom: {:?}", denom);
    let val = Decimal256::from_ratio(t, denom);
    println!("decimal: {}", val);
    println!("check: {}", Uint256::from(10u128.pow(20)).mul(val));
}
#[test]
fn test_convert_reverse() {
    let mut deps = mock_dependencies(&[
        coin(10000000000u128, ORAI_DENOM),
        coin(20000000000u128, ATOM_DENOM),
    ]);

    let msg = InitMsg {};
    let info = mock_info("addr", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();

    //pair1
    let msg = HandleMsg::UpdatePair {
        from: TokenInfo {
            info: AssetInfo::Token {
                contract_addr: "asset1".into(),
            },
            decimals: 18,
        },
        to: TokenInfo {
            info: AssetInfo::Token {
                contract_addr: "asset2".into(),
            },
            decimals: 6,
        },
    };

    //register pair1
    let info = mock_info("addr", &[]);
    handle(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    //convert_reverse asset2 to asset1
    let info = mock_info("asset2", &[]);
    let convert_msg = Cw20HookMsg::ConvertReverse {
        from: AssetInfo::Token {
            contract_addr: "asset1".into(),
        },
    };
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        amount: Uint128::from(1u64),
        sender: info.sender.clone(),
        msg: Some(to_binary(&convert_msg).unwrap()),
    });
    let res = handle(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset1".into(),
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: info.sender.clone(),
                amount: Uint128::from(10u128.pow(12))
            })
            .unwrap(),
            send: vec![]
        })]
    );

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "convert_token_reverse"),
            attr("from_amount", "1"),
            attr("to_amount", "1000000000000"),
        ]
    );

    //check if sender not from asset2
    let info = mock_info("addr", &[]);
    let convert_msg = Cw20HookMsg::ConvertReverse {
        from: AssetInfo::Token {
            contract_addr: "asset1".into(),
        },
    };
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        amount: Uint128::from(1u64),
        sender: info.sender.clone(),
        msg: Some(to_binary(&convert_msg).unwrap()),
    });
    let res = handle(deps.as_mut(), mock_env(), info.clone(), msg.clone());

    match res {
        Err(StdError::GenericErr { msg }) => assert_eq!(msg, "invalid cw20 hook message"),
        _ => panic!("Must return invalid cw20 hook message"),
    };

    //pair2
    let msg = HandleMsg::UpdatePair {
        from: TokenInfo {
            info: AssetInfo::Token {
                contract_addr: "asset1".into(),
            },
            decimals: 6,
        },
        to: TokenInfo {
            info: AssetInfo::NativeToken {
                denom: ORAI_DENOM.into(),
            },
            decimals: 18,
        },
    };
    let info = mock_info("addr", &[]);
    handle(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

    //convert_reverse
    let msg = HandleMsg::ConvertReverse {
        from_asset: AssetInfo::Token {
            contract_addr: "asset1".into(),
        },
    };

    //convert 10^12 ORAI to asset1
    let info = mock_info("addr", &[coin(1000000000000u128, ORAI_DENOM)]);
    let res = handle(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset1".into(),
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: info.sender.clone(),
                amount: Uint128::from(1u128)
            })
            .unwrap(),
            send: vec![]
        })]
    );

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "convert_token_reverse"),
            attr("denom", ORAI_DENOM),
            attr("from_amount", "1000000000000"),
            attr("to_amount", "1"),
        ]
    );

    //check if not send Orai to convert to asset1
    let msg = HandleMsg::ConvertReverse {
        from_asset: AssetInfo::Token {
            contract_addr: "asset1".into(),
        },
    };

    //convert 10^12 ORAI to asset1
    let info = mock_info("addr", &[coin(1000000000000u128, ATOM_DENOM)]);
    let res = handle(deps.as_mut(), mock_env(), info.clone(), msg.clone());

    match res {
        Err(StdError::GenericErr { msg }) => assert_eq!(msg, "invalid cw20 hook message"),
        _ => panic!("Must return invalid cw20 hook message"),
    };
}

#[test]
fn test_remove_pair() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {};
    let info = mock_info("addr", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();

    let msg = HandleMsg::UpdatePair {
        from: TokenInfo {
            info: AssetInfo::Token {
                contract_addr: "asset1".into(),
            },
            decimals: 16,
        },
        to: TokenInfo {
            info: AssetInfo::Token {
                contract_addr: "asset2".into(),
            },
            decimals: 16,
        },
    };
    let info = mock_info("addr", &[]);
    let res = handle(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::ConvertInfo {
            asset_info: AssetInfo::Token {
                contract_addr: "asset1".into(),
            },
        },
    )
    .unwrap();

    let msg = HandleMsg::UnregisterPair {
        from: TokenInfo {
            info: AssetInfo::Token {
                contract_addr: "asset1".into(),
            },
            decimals: 16,
        },
    };
    let info = mock_info("addr", &[]);
    let res = handle(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::ConvertInfo {
            asset_info: AssetInfo::Token {
                contract_addr: "asset1".into(),
            },
        },
    );

    match res {
        Err(StdError::NotFound { kind }) => assert_eq!(kind, "oraiswap::converter::TokenRatio"),
        _ => panic!("Must return not found"),
    };
}

#[test]
fn test_withdraw_tokens() {
    let mut deps = mock_dependencies(&[
        coin(10000000000u128, ORAI_DENOM),
        coin(20000000000u128, ATOM_DENOM),
    ]);

    let msg = InitMsg {};
    let info = mock_info(
        "addr",
        &[
            coin(10000000000u128, ORAI_DENOM),
            coin(20000000000u128, ATOM_DENOM),
        ],
    );

    // we can just call .unwrap() to assert this was a success
    let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

    //test proper withdraw tokens
    let msg = HandleMsg::WithdrawTokens {
        asset_infos: vec![
            AssetInfo::NativeToken {
                denom: ORAI_DENOM.into(),
            },
            AssetInfo::NativeToken {
                denom: ATOM_DENOM.into(),
            },
        ],
    };

    let info = mock_info("addr", &[]);
    let res = handle(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Bank(BankMsg::Send {
                from_address: mock_env().contract.address,
                to_address: info.sender.clone(),
                amount: vec![coin(10000000000u128, ORAI_DENOM),],
            }),
            CosmosMsg::Bank(BankMsg::Send {
                from_address: mock_env().contract.address,
                to_address: info.sender,
                amount: vec![coin(20000000000u128, ATOM_DENOM),],
            })
        ]
    );
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "withdraw_tokens"),
            attr("amount", 10000000000u128.to_string()),
            attr("amount", 20000000000u128.to_string())
        ]
    );

    //test unauthorized withdraw tokens
    let msg = HandleMsg::WithdrawTokens {
        asset_infos: vec![
            AssetInfo::NativeToken {
                denom: ORAI_DENOM.into(),
            },
            AssetInfo::NativeToken {
                denom: ATOM_DENOM.into(),
            },
        ],
    };

    let info = mock_info("addr1", &[]);
    let res = handle(deps.as_mut(), mock_env(), info.clone(), msg.clone());

    match res {
        Err(StdError::GenericErr { msg }) => assert_eq!(msg, "unauthorized"),
        _ => panic!("Must return unauthorized"),
    };
}
