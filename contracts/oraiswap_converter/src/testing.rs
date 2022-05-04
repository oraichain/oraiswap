
use cosmwasm_std::{testing::{mock_env, mock_info}, StdError};
use oraiswap::{mock_querier::mock_dependencies, converter::{InitMsg, HandleMsg, TokenInfo, QueryMsg}, asset::AssetInfo};

use crate::contract::{init, query, handle};
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
    // let convert_info: ConvertInfoResponse = from_binary(&res).unwrap();
    // print!("{:?}", convert_info);

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