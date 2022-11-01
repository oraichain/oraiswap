use crate::asset::AssetInfo;

use crate::mock_app::MockApp;

use crate::querier::{query_supply, query_token_balance};

use cosmwasm_std::testing::MOCK_CONTRACT_ADDR;
use cosmwasm_std::{Coin, Uint128};
use cw_multi_test::{Contract, ContractWrapper};

fn contract_cw20() -> Box<dyn Contract> {
    let contract = ContractWrapper::new(
        cw20_base::contract::handle,
        cw20_base::contract::init,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

#[test]
fn token_balance_querier() {
    let mut app = MockApp::new();

    app.set_token_contract(contract_cw20());

    app.set_token_balances(&[(
        &"AIRI".to_string(),
        &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(123u128))],
    )]);

    assert_eq!(
        Uint128::from(123u128),
        query_token_balance(
            &app.as_querier(),
            app.get_token_addr("AIRI").unwrap(),
            MOCK_CONTRACT_ADDR.into(),
        )
        .unwrap()
    );
}

#[test]
fn balance_querier() {
    let mut app = MockApp::new();
    app.set_balance(
        MOCK_CONTRACT_ADDR.into(),
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(200u128),
        }],
    );

    assert_eq!(
        app.query_balance(MOCK_CONTRACT_ADDR.into(), "uusd".to_string())
            .unwrap(),
        Uint128::from(200u128)
    );
}

#[test]
fn all_balances_querier() {
    let mut app = MockApp::new();
    app.set_balance(
        MOCK_CONTRACT_ADDR.into(),
        &[
            Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(200u128),
            },
            Coin {
                denom: "ukrw".to_string(),
                amount: Uint128::from(300u128),
            },
        ],
    );

    let mut balance1 = app.query_all_balances(MOCK_CONTRACT_ADDR.into()).unwrap();
    balance1.sort_by(|a, b| a.denom.cmp(&b.denom));
    let mut balance2 = vec![
        Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(200u128),
        },
        Coin {
            denom: "ukrw".to_string(),
            amount: Uint128::from(300u128),
        },
    ];
    balance2.sort_by(|a, b| a.denom.cmp(&b.denom));
    assert_eq!(balance1, balance2);
}

#[test]
fn supply_querier() {
    let mut app = MockApp::new();
    app.set_token_contract(contract_cw20());
    app.set_token_balances(&[(
        &"LPA".to_string(),
        &[
            (&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(123u128)),
            (&"addr00000".to_string(), &Uint128::from(123u128)),
            (&"addr00001".to_string(), &Uint128::from(123u128)),
            (&"addr00002".to_string(), &Uint128::from(123u128)),
        ],
    )]);

    assert_eq!(
        query_supply(&app.as_querier(), app.get_token_addr("LPA").unwrap()).unwrap(),
        Uint128::from(492u128)
    )
}

#[test]
fn test_asset_info() {
    let mut app = MockApp::new();
    app.set_token_contract(contract_cw20());
    app.set_balance(
        MOCK_CONTRACT_ADDR.into(),
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(123u128),
        }],
    );
    app.set_token_balances(&[(
        &"ASSET".to_string(),
        &[
            (&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(123u128)),
            (&"addr00000".to_string(), &Uint128::from(123u128)),
            (&"addr00001".to_string(), &Uint128::from(123u128)),
            (&"addr00002".to_string(), &Uint128::from(123u128)),
        ],
    )]);

    let token_info: AssetInfo = AssetInfo::Token {
        contract_addr: app.get_token_addr("ASSET").unwrap(),
    };
    let native_token_info: AssetInfo = AssetInfo::NativeToken {
        denom: "uusd".to_string(),
    };

    assert!(!token_info.eq(&native_token_info));
    assert!(native_token_info.is_native_token());
    assert!(!token_info.is_native_token());

    assert_eq!(
        token_info
            .query_pool(&app.as_querier(), MOCK_CONTRACT_ADDR.into())
            .unwrap(),
        Uint128::from(123u128)
    );
    assert_eq!(
        native_token_info
            .query_pool(&app.as_querier(), MOCK_CONTRACT_ADDR.into())
            .unwrap(),
        Uint128::from(123u128)
    );
}
