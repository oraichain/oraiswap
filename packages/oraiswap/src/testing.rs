use crate::asset::{Asset, AssetInfo, PairInfo};
use crate::mock_querier::mock_dependencies;
use crate::oracle::OracleContract;
use crate::querier::{
    query_all_balances, query_balance, query_pair_info, query_supply, query_token_balance,
};

use cosmwasm_std::testing::MOCK_CONTRACT_ADDR;
use cosmwasm_std::{to_binary, BankMsg, Coin, CosmosMsg, Decimal, Uint128, WasmMsg};
use cw20::Cw20HandleMsg;

#[test]
fn token_balance_querier() {
    let mut deps = mock_dependencies(&[]);

    deps.querier.with_token_balances(&[(
        &"liquidity0000".to_string(),
        &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(123u128))],
    )]);

    assert_eq!(
        Uint128::from(123u128),
        query_token_balance(
            &deps.as_ref().querier,
            "liquidity0000".into(),
            MOCK_CONTRACT_ADDR.into(),
        )
        .unwrap()
    );
}

#[test]
fn balance_querier() {
    let deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(200u128),
    }]);

    assert_eq!(
        query_balance(
            &deps.as_ref().querier,
            MOCK_CONTRACT_ADDR.into(),
            "uusd".to_string()
        )
        .unwrap(),
        Uint128::from(200u128)
    );
}

#[test]
fn all_balances_querier() {
    let deps = mock_dependencies(&[
        Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(200u128),
        },
        Coin {
            denom: "ukrw".to_string(),
            amount: Uint128::from(300u128),
        },
    ]);

    assert_eq!(
        query_all_balances(&deps.as_ref().querier, MOCK_CONTRACT_ADDR.into(),).unwrap(),
        vec![
            Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(200u128),
            },
            Coin {
                denom: "ukrw".to_string(),
                amount: Uint128::from(300u128),
            }
        ]
    );
}

#[test]
fn supply_querier() {
    let mut deps = mock_dependencies(&[]);

    deps.querier.with_token_balances(&[(
        &"liquidity0000".to_string(),
        &[
            (&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(123u128)),
            (&"addr00000".to_string(), &Uint128::from(123u128)),
            (&"addr00001".to_string(), &Uint128::from(123u128)),
            (&"addr00002".to_string(), &Uint128::from(123u128)),
        ],
    )]);

    assert_eq!(
        query_supply(&deps.as_ref().querier, "liquidity0000".into()).unwrap(),
        Uint128::from(492u128)
    )
}

#[test]
fn test_asset_info() {
    let token_info: AssetInfo = AssetInfo::Token {
        contract_addr: "asset0000".into(),
    };
    let native_token_info: AssetInfo = AssetInfo::NativeToken {
        denom: "uusd".to_string(),
    };

    assert!(!token_info.equal(&native_token_info));

    assert!(!token_info.equal(&AssetInfo::Token {
        contract_addr: "asset0001".into(),
    }));

    assert!(token_info.equal(&AssetInfo::Token {
        contract_addr: "asset0000".into(),
    }));

    assert!(native_token_info.is_native_token());
    assert!(!token_info.is_native_token());

    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(123u128),
    }]);
    deps.querier.with_token_balances(&[(
        &"asset0000".to_string(),
        &[
            (&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(123u128)),
            (&"addr00000".to_string(), &Uint128::from(123u128)),
            (&"addr00001".to_string(), &Uint128::from(123u128)),
            (&"addr00002".to_string(), &Uint128::from(123u128)),
        ],
    )]);

    assert_eq!(
        token_info
            .query_pool(&deps.as_ref().querier, MOCK_CONTRACT_ADDR.into())
            .unwrap(),
        Uint128::from(123u128)
    );
    assert_eq!(
        native_token_info
            .query_pool(&deps.as_ref().querier, MOCK_CONTRACT_ADDR.into())
            .unwrap(),
        Uint128::from(123u128)
    );
}

#[test]
fn test_asset() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(123u128),
    }]);

    deps.querier.with_token_balances(&[(
        &"asset0000".to_string(),
        &[
            (&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(123u128)),
            (&"addr00000".to_string(), &Uint128::from(123u128)),
            (&"addr00001".to_string(), &Uint128::from(123u128)),
            (&"addr00002".to_string(), &Uint128::from(123u128)),
        ],
    )]);

    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let token_asset = Asset {
        amount: Uint128::from(123123u128),
        info: AssetInfo::Token {
            contract_addr: "asset0000".into(),
        },
    };

    let native_token_asset = Asset {
        amount: Uint128::from(123123u128),
        info: AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
    };

    let orai_oracle = OracleContract(MOCK_CONTRACT_ADDR.into());

    assert_eq!(
        token_asset
            .compute_tax(&orai_oracle, &deps.as_ref().querier)
            .unwrap(),
        Uint128::zero()
    );
    assert_eq!(
        native_token_asset
            .compute_tax(&orai_oracle, &deps.as_ref().querier)
            .unwrap(),
        Uint128::from(1220u128)
    );

    assert_eq!(
        native_token_asset
            .deduct_tax(&orai_oracle, &deps.as_ref().querier)
            .unwrap(),
        Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(121903u128),
        }
    );

    assert_eq!(
        token_asset
            .into_msg(
                &orai_oracle,
                &deps.as_ref().querier,
                MOCK_CONTRACT_ADDR.into(),
                "addr0000".into()
            )
            .unwrap(),
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "asset0000".into(),
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: "addr0000".into(),
                amount: Uint128::from(123123u128),
            })
            .unwrap(),
            send: vec![],
        })
    );

    assert_eq!(
        native_token_asset
            .into_msg(
                &orai_oracle,
                &deps.as_ref().querier,
                MOCK_CONTRACT_ADDR.into(),
                "addr0000".into()
            )
            .unwrap(),
        CosmosMsg::Bank(BankMsg::Send {
            from_address: MOCK_CONTRACT_ADDR.into(),
            to_address: "addr0000".into(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::from(121903u128),
            }]
        })
    );
}

#[test]
fn query_oraiswap_pair_contract() {
    let mut deps = mock_dependencies(&[]);

    deps.querier.with_oraiswap_pairs(&[(
        &"asset0000uusd".to_string(),
        &PairInfo {
            asset_infos: [
                AssetInfo::Token {
                    contract_addr: "asset0000".into(),
                },
                AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
            ],

            oracle_addr: "oracle0000".into(),
            contract_addr: "pair0000".into(),
            liquidity_token: "liquidity0000".into(),
            commission_rate: "1".to_string(),
        },
    )]);

    let pair_info: PairInfo = query_pair_info(
        &deps.as_ref().querier,
        MOCK_CONTRACT_ADDR.into(),
        &[
            AssetInfo::Token {
                contract_addr: "asset0000".into(),
            },
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
        ],
    )
    .unwrap();

    assert_eq!(pair_info.contract_addr, "pair0000");
    assert_eq!(pair_info.liquidity_token, "liquidity0000");
}
