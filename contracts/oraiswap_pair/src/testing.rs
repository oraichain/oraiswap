use cosmwasm_std::testing::MOCK_CONTRACT_ADDR;
use cosmwasm_std::{attr, to_binary, Addr, Coin, Decimal, StdError, Uint128};
use cw20::Cw20ReceiveMsg;
use oraiswap::asset::{Asset, AssetInfo, PairInfo, ORAI_DENOM};
use oraiswap::create_entry_points_testing;
use oraiswap::pair::{Cw20HookMsg, ExecuteMsg, InstantiateMsg};
use oraiswap::testing::{MockApp, ATOM_DENOM};

#[test]
fn provide_liquidity_both_native() {
    let mut app = MockApp::new(&[(
        &MOCK_CONTRACT_ADDR.to_string(),
        &[
            Coin {
                denom: ORAI_DENOM.to_string(),
                amount: Uint128::from(200u128),
            },
            Coin {
                denom: ATOM_DENOM.to_string(),
                amount: Uint128::from(200u128),
            },
        ],
    )]);

    app.set_oracle_contract(create_entry_points_testing!(oraiswap_oracle));

    app.set_token_contract(create_entry_points_testing!(oraiswap_token));

    app.set_token_balances(&[
        (
            &"liquidity".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::zero())],
        ),
        (&"asset".to_string(), &[]),
    ]);

    let msg = InstantiateMsg {
        oracle_addr: app.oracle_addr.clone(),
        asset_infos: [
            AssetInfo::NativeToken {
                denom: ORAI_DENOM.to_string(),
            },
            AssetInfo::NativeToken {
                denom: ATOM_DENOM.to_string(),
            },
        ],
        token_code_id: app.token_id,
        commission_rate: None,
    };

    // we can just call .unwrap() to assert this was a success
    let code_id = app.upload(create_entry_points_testing!(crate));
    let pair_addr = app
        .instantiate(code_id, Addr::unchecked("owner"), &msg, &[], "pair")
        .unwrap();

    // successfully provide liquidity for the exist pool
    let msg = ExecuteMsg::ProvideLiquidity {
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

    let res = app
        .execute(
            Addr::unchecked(MOCK_CONTRACT_ADDR),
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

    println!("{:?}", res);
}

// #[test]
// fn provide_liquidity() {
//     let mut app = MockApp::new();
//     app.set_token_contract(oraiswap_token::testutils::contract());
//     app.set_balance(
//         MOCK_CONTRACT_ADDR.into(),
//         &[Coin {
//             denom: ORAI_DENOM.to_string(),
//             amount: Uint128::from(200u128),
//         }],
//     );
//     app.set_oracle_contract(oraiswap_oracle::testutils::contract());

//     app.set_token_balances(&[
//         (
//             &"liquidity".to_string(),
//             &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(1000u128))],
//         ),
//         (
//             &"asset".to_string(),
//             &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(1000u128))],
//         ),
//     ]);

//     let asset_addr = app.get_token_addr("asset").unwrap();

//     let msg = InstantiateMsg {
//         oracle_addr: app.oracle_addr.clone(),
//         asset_infos: [
//             AssetInfo::NativeToken {
//                 denom: ORAI_DENOM.to_string(),
//             },
//             AssetInfo::Token {
//                 contract_addr: asset_addr.clone(),
//             },
//         ],
//         token_code_id: app.token_id,
//         commission_rate: None,
//         init_hook: None,
//     };

//     // we can just call .unwrap() to assert this was a success
//     let code_id = app.upload(crate::testutils::contract());
//     let pair_addr = app
//         .instantiate(code_id, "owner".into(), &msg, &[], "pair")
//         .unwrap();

//     // set allowance
//     app.execute(
//         MOCK_CONTRACT_ADDR.into(),
//         asset_addr.clone(),
//         &oraiswap_token::msg::ExecuteMsg::IncreaseAllowance {
//             spender: pair_addr.clone(),
//             amount: Uint128::from(100u128),
//             expires: None,
//         },
//         &[],
//     )
//     .unwrap();

//     // successfully provide liquidity for the exist pool
//     let msg = ExecuteMsg::ProvideLiquidity {
//         assets: [
//             Asset {
//                 info: AssetInfo::Token {
//                     contract_addr: asset_addr.clone(),
//                 },
//                 amount: Uint128::from(100u128),
//             },
//             Asset {
//                 info: AssetInfo::NativeToken {
//                     denom: ORAI_DENOM.to_string(),
//                 },
//                 amount: Uint128::from(100u128),
//             },
//         ],
//         slippage_tolerance: None,
//         receiver: None,
//     };

//     let _res = app
//         .execute(
//             MOCK_CONTRACT_ADDR.into(),
//             pair_addr.clone(),
//             &msg,
//             &[Coin {
//                 denom: ORAI_DENOM.to_string(),
//                 amount: Uint128::from(100u128),
//             }],
//         )
//         .unwrap();

//     // provide more liquidity 1:2, which is not proportional to 1:1,
//     // then it must accept 1:1 and treat left amount as donation
//     app.set_balance(
//         MOCK_CONTRACT_ADDR.into(),
//         &[Coin {
//             denom: ORAI_DENOM.to_string(),
//             amount: Uint128::from(
//                 200u128 + 200u128, /* user deposit must be pre-applied */
//             ),
//         }],
//     );

//     // set allowance one more 100
//     app.execute(
//         MOCK_CONTRACT_ADDR.into(),
//         asset_addr.clone(),
//         &oraiswap_token::msg::ExecuteMsg::IncreaseAllowance {
//             spender: pair_addr.clone(),
//             amount: Uint128::from(100u128),
//             expires: None,
//         },
//         &[],
//     )
//     .unwrap();

//     let msg = ExecuteMsg::ProvideLiquidity {
//         assets: [
//             Asset {
//                 info: AssetInfo::Token {
//                     contract_addr: asset_addr.clone(),
//                 },
//                 amount: Uint128::from(100u128),
//             },
//             Asset {
//                 info: AssetInfo::NativeToken {
//                     denom: ORAI_DENOM.to_string(),
//                 },
//                 amount: Uint128::from(200u128),
//             },
//         ],
//         slippage_tolerance: None,
//         receiver: Some("staking0000".into()), // try changing receiver
//     };

//     // only accept 100, then 50 share will be generated with 100 * (100 / 200)
//     let _res = app
//         .execute(
//             MOCK_CONTRACT_ADDR.into(),
//             pair_addr.clone(),
//             &msg,
//             &[Coin {
//                 denom: ORAI_DENOM.to_string(),
//                 amount: Uint128::from(200u128),
//             }],
//         )
//         .unwrap();

//     // check wrong argument
//     let msg = ExecuteMsg::ProvideLiquidity {
//         assets: [
//             Asset {
//                 info: AssetInfo::Token {
//                     contract_addr: asset_addr.clone(),
//                 },
//                 amount: Uint128::from(100u128),
//             },
//             Asset {
//                 info: AssetInfo::NativeToken {
//                     denom: ORAI_DENOM.to_string(),
//                 },
//                 amount: Uint128::from(50u128),
//             },
//         ],
//         slippage_tolerance: None,
//         receiver: None,
//     };

//     let res = app.execute(
//         MOCK_CONTRACT_ADDR.into(),
//         pair_addr.clone(),
//         &msg,
//         &[Coin {
//             denom: ORAI_DENOM.to_string(),
//             amount: Uint128::from(100u128),
//         }],
//     );
//     match res.err() {
//         Some(msg) => assert_eq!(
//             msg,
//             StdError::generic_err(
//                 "Native token balance mismatch between the argument and the transferred"
//             )
//             .to_string()
//         ),
//         None => panic!("Must return generic error"),
//     }
// }

// #[test]
// fn withdraw_liquidity() {
//     let mut app = MockApp::new();
//     app.set_balance(
//         "addr0000".into(),
//         &[Coin {
//             denom: ORAI_DENOM.to_string(),
//             amount: Uint128::from(1000u128),
//         }],
//     );

//     app.set_oracle_contract(oraiswap_oracle::testutils::contract());

//     app.set_tax(
//         Decimal::zero(),
//         &[(&ORAI_DENOM.to_string(), &Uint128::from(1000000u128))],
//     );

//     app.set_token_contract(oraiswap_token::testutils::contract());

//     app.set_token_balances(&[(
//         &"liquidity".to_string(),
//         &[(&"addr0000".to_string(), &Uint128::from(1000u128))],
//     )]);

//     let liquidity_addr = app.get_token_addr("liquidity").unwrap();

//     let msg = InstantiateMsg {
//         oracle_addr: app.oracle_addr.clone(),
//         asset_infos: [
//             AssetInfo::NativeToken {
//                 denom: ORAI_DENOM.to_string(),
//             },
//             AssetInfo::Token {
//                 contract_addr: liquidity_addr.clone(),
//             },
//         ],
//         token_code_id: app.token_id,
//         commission_rate: None,
//         init_hook: None,
//     };

//     let pair_id = app.upload(crate::testutils::contract());
//     // we can just call .unwrap() to assert this was a success
//     let pair_addr = app
//         .instantiate(pair_id, "addr0000".into(), &msg, &[], "pair")
//         .unwrap();

//     // set allowance one more 100
//     app.execute(
//         "addr0000".into(),
//         liquidity_addr.clone(),
//         &oraiswap_token::msg::ExecuteMsg::IncreaseAllowance {
//             spender: pair_addr.clone(),
//             amount: Uint128::from(1000u128),
//             expires: None,
//         },
//         &[],
//     )
//     .unwrap();

//     let msg = ExecuteMsg::ProvideLiquidity {
//         assets: [
//             Asset {
//                 info: AssetInfo::Token {
//                     contract_addr: liquidity_addr.clone(),
//                 },
//                 amount: Uint128::from(100u128),
//             },
//             Asset {
//                 info: AssetInfo::NativeToken {
//                     denom: ORAI_DENOM.to_string(),
//                 },
//                 amount: Uint128::from(100u128),
//             },
//         ],
//         slippage_tolerance: None,
//         // we send lq token to pair and later call it directly to test
//         receiver: Some(pair_addr.clone()),
//     };

//     // only accept 100, then 50 share will be generated with 100 * (100 / 200)
//     let _res = app
//         .execute(
//             "addr0000".into(),
//             pair_addr.clone(),
//             &msg,
//             &[Coin {
//                 denom: ORAI_DENOM.to_string(),
//                 amount: Uint128::from(100u128),
//             }],
//         )
//         .unwrap();

//     // withdraw liquidity
//     let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
//         sender: "addr0000".into(),
//         msg: to_binary(&Cw20HookMsg::WithdrawLiquidity {}).ok(),
//         amount: Uint128::from(100u128),
//     });

//     let pair_info: PairInfo = app
//         .query(pair_addr.clone(), &oraiswap::pair::QueryMsg::Pair {})
//         .unwrap();

//     let res = app
//         .execute(pair_info.liquidity_token, pair_addr.clone(), &msg, &[])
//         .unwrap();

//     let log_withdrawn_share = res.attributes.get(2).expect("no log");
//     let log_refund_assets = res.attributes.get(3).expect("no log");

//     assert_eq!(
//         log_withdrawn_share,
//         &attr("withdrawn_share", 100u128.to_string())
//     );
//     assert_eq!(
//         log_refund_assets,
//         &attr(
//             "refund_assets",
//             format!("200{}, 100{}", ORAI_DENOM, liquidity_addr)
//         )
//     );
// }
