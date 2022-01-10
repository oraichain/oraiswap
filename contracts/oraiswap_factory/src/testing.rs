use crate::contract::{handle, init, query};
use crate::mock_querier::mock_dependencies;

use crate::state::{pair_key, TmpPairInfo, TMP_PAIR_INFO};

use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{attr, from_binary, to_binary, HumanAddr, StdError, WasmMsg};
use oraiswap::asset::{AssetInfo, PairInfo};
use oraiswap::factory::{ConfigResponse, HandleMsg, InitMsg, QueryMsg};
use oraiswap::pair::InitMsg as PairInitMsg;

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        pair_code_id: 321u64,
        token_code_id: 123u64,
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(123u64, config_res.token_code_id);
    assert_eq!(321u64, config_res.pair_code_id);
    assert_eq!("addr0000".to_string(), config_res.owner);
}

#[test]
fn update_config() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        pair_code_id: 321u64,
        token_code_id: 123u64,
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

    // update owner
    let info = mock_info("addr0000", &[]);
    let msg = HandleMsg::UpdateConfig {
        owner: Some("addr0001".to_string()),
        pair_code_id: None,
        token_code_id: None,
    };

    let res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(123u64, config_res.token_code_id);
    assert_eq!(321u64, config_res.pair_code_id);
    assert_eq!("addr0001".to_string(), config_res.owner);

    // update left items
    let env = mock_env();
    let info = mock_info("addr0001", &[]);
    let msg = HandleMsg::UpdateConfig {
        owner: None,
        pair_code_id: Some(100u64),
        token_code_id: Some(200u64),
    };

    let res = handle(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(200u64, config_res.token_code_id);
    assert_eq!(100u64, config_res.pair_code_id);
    assert_eq!("addr0001".to_string(), config_res.owner);

    // Unauthorized err
    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let msg = HandleMsg::UpdateConfig {
        owner: None,
        pair_code_id: None,
        token_code_id: None,
    };

    let res = handle(deps.as_mut(), env, info, msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "unauthorized"),
        _ => panic!("Must return unauthorized error"),
    }
}

#[test]
fn create_pair() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        pair_code_id: 321u64,
        token_code_id: 123u64,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(deps.as_mut(), env, info, msg).unwrap();

    let asset_infos = [
        AssetInfo::Token {
            contract_addr: "asset0000".to_string(),
        },
        AssetInfo::Token {
            contract_addr: "asset0001".to_string(),
        },
    ];

    let msg = HandleMsg::CreatePair {
        asset_infos: asset_infos.clone(),
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    let res = handle(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "create_pair"),
            attr("pair", "asset0000-asset0001")
        ]
    );
    assert_eq!(
        res.messages,
        vec![WasmMsg::Instantiate {
            msg: to_binary(&PairInitMsg {
                asset_infos: asset_infos.clone(),
                token_code_id: 123u64,
            })
            .unwrap(),
            code_id: 321u64,
            send: vec![],
            label: None,
        }
        .into()]
    );

    let raw_infos = [
        asset_infos[0].to_raw(deps.as_ref().api).unwrap(),
        asset_infos[1].to_raw(deps.as_ref().api).unwrap(),
    ];

    let pair_key = pair_key(&raw_infos);

    assert_eq!(
        TMP_PAIR_INFO.load(&deps.storage, &pair_key).unwrap(),
        TmpPairInfo {
            asset_infos: raw_infos.clone(),
            creator: HumanAddr("addr0000".to_string()),
        }
    );
}

#[test]
fn update_pair() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        pair_code_id: 321u64,
        token_code_id: 123u64,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(deps.as_mut(), env, info, msg).unwrap();

    let asset_infos = [
        AssetInfo::Token {
            contract_addr: "asset0000".to_string(),
        },
        AssetInfo::Token {
            contract_addr: "asset0001".to_string(),
        },
    ];

    let raw_infos = [
        asset_infos[0].to_raw(deps.as_ref().api).unwrap(),
        asset_infos[1].to_raw(deps.as_ref().api).unwrap(),
    ];

    let pair_key = pair_key(&raw_infos);

    let msg = HandleMsg::CreatePair {
        asset_infos: asset_infos.clone(),
    };

    let _res = handle(deps.as_mut(), mock_env(), mock_info("addr0000", &[]), msg).unwrap();

    assert_eq!(
        TMP_PAIR_INFO.load(&deps.storage, &pair_key).unwrap(),
        TmpPairInfo {
            asset_infos: raw_infos.clone(),
            creator: HumanAddr("addr0000".to_string()),
        }
    );

    // register oraiswap pair querier, it is like deploy smart contract, let's assume pair0000 has liquidity_token address liquidity0000
    deps.querier.with_oraiswap_pairs(&[(
        &"pair0000".to_string(),
        &PairInfo {
            asset_infos: asset_infos.clone(),
            contract_addr: "pair0000".to_string(),
            liquidity_token: "liquidity0000".to_string(),
        },
    )]);

    // later update pair with newly created address
    let update_msg = HandleMsg::UpdatePair {
        pair_key,
        contract_addr: HumanAddr("pair0000".to_string()),
    };
    let _res = handle(
        deps.as_mut(),
        mock_env(),
        mock_info("addr0000", &[]),
        update_msg,
    )
    .unwrap();

    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Pair {
            asset_infos: asset_infos.clone(),
        },
    )
    .unwrap();

    let pair_res: PairInfo = from_binary(&query_res).unwrap();
    assert_eq!(
        pair_res,
        PairInfo {
            liquidity_token: "liquidity0000".to_string(),
            contract_addr: "pair0000".to_string(),
            asset_infos,
        }
    );
}
