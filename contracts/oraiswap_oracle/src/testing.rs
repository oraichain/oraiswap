use crate::contract::*;
use cosmwasm_std::testing::{
    mock_dependencies_with_balance, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
    MOCK_CONTRACT_ADDR,
};
use cosmwasm_std::{
    coins, from_binary, to_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal, OwnedDeps, StdError,
    Uint128, WasmMsg,
};

use oraiswap::asset::{Asset, AssetInfo, ORAI_DENOM};
use oraiswap::create_entry_points_testing;
use oraiswap::oracle::{
    ExchangeRateResponse, ExecuteMsg, InstantiateMsg, OracleContract, QueryMsg,
};
use oraiswap::testing::MockApp;

const OWNER: &str = "owner0000";

fn setup_contract() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies_with_balance(&coins(100000, ORAI_DENOM));

    let msg = InstantiateMsg {
        name: None,
        version: None,
        admin: None,
        min_rate: None,
        max_rate: None,
    };
    let info = mock_info(OWNER, &[]);
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    deps
}

#[test]
fn proper_initialization() {
    let mut deps = setup_contract();

    let msg = ExecuteMsg::UpdateExchangeRate {
        denom: "usdt".to_string(),
        exchange_rate: Decimal::percent(10), // 1 orai = 10 usdt
    };

    let _res = execute(deps.as_mut(), mock_env(), mock_info(OWNER, &[]), msg).unwrap();

    let msg = QueryMsg::ExchangeRate {
        base_denom: Some("usdt".to_string()),
        quote_denom: ORAI_DENOM.to_string(),
    };

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let exchange_rate_res: ExchangeRateResponse = from_binary(&res).unwrap();

    assert_eq!("10", exchange_rate_res.item.exchange_rate.to_string());
}

#[test]
fn tax_cap_notfound() {
    let deps = setup_contract();

    let msg = QueryMsg::TaxCap {
        denom: "airi".to_string(),
    };

    let res = query(deps.as_ref(), mock_env(), msg);
    match res {
        Err(StdError::NotFound { kind }) => {
            assert_eq!(kind, format!("Tax cap not found for denom: {}", "airi"))
        }
        _ => panic!("DO NOT ENTER HERE"),
    }
}

#[test]
fn test_asset() {
    let mut app = MockApp::new(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(123u128),
        }],
    )]);

    app.set_token_contract(Box::new(create_entry_points_testing!(oraiswap_token)));

    app.set_token_balances(&[(
        &"asset".to_string(),
        &[
            (&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(123u128)),
            (&"addr00000".to_string(), &Uint128::from(123u128)),
            (&"addr00001".to_string(), &Uint128::from(123u128)),
            (&"addr00002".to_string(), &Uint128::from(123u128)),
        ],
    )]);

    // set code implementation
    app.set_oracle_contract(Box::new(create_entry_points_testing!(crate)));

    app.set_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let token_asset = Asset {
        amount: Uint128::from(123123u128),
        info: AssetInfo::Token {
            contract_addr: Addr::unchecked("asset0000"),
        },
    };

    let native_token_asset = Asset {
        amount: Uint128::from(123123u128),
        info: AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
    };

    let orai_oracle = OracleContract(app.oracle_addr.clone());

    assert_eq!(
        token_asset
            .compute_tax(&orai_oracle, &app.as_querier())
            .unwrap(),
        Uint128::zero()
    );
    assert_eq!(
        native_token_asset
            .compute_tax(&orai_oracle, &app.as_querier())
            .unwrap(),
        Uint128::from(1220u128)
    );

    assert_eq!(
        native_token_asset
            .deduct_tax(&orai_oracle, &app.as_querier())
            .unwrap(),
        Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(121903u128),
        }
    );

    assert_eq!(
        token_asset
            .into_msg(
                Some(&orai_oracle),
                &app.as_querier(),
                Addr::unchecked("addr0000")
            )
            .unwrap(),
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset0000".into(),
            msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                recipient: "addr0000".into(),
                amount: Uint128::from(123123u128),
            })
            .unwrap(),
            funds: vec![],
        })
    );

    assert_eq!(
        native_token_asset
            .into_msg(
                Some(&orai_oracle),
                &app.as_querier(),
                Addr::unchecked("addr0000")
            )
            .unwrap(),
        CosmosMsg::Bank(BankMsg::Send {
            to_address: "addr0000".into(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(121903u128),
            }]
        })
    );
}
