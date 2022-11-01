use crate::contract::{handle, init, query};

use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{attr, from_binary, to_binary, WasmMsg};
use cw_multi_test::{Contract, ContractWrapper};
use oraiswap::asset::{AssetInfo, PairInfo};
use oraiswap::error::ContractError;
use oraiswap::factory::{ConfigResponse, HandleMsg, InitMsg, QueryMsg};
use oraiswap::hook::InitHook;
use oraiswap::mock_app::MockApp;
use oraiswap::pair::{InitMsg as PairInitMsg, DEFAULT_COMMISSION_RATE};

fn contract_token() -> Box<dyn Contract> {
    let contract = ContractWrapper::new(
        oraiswap_token::contract::handle,
        oraiswap_token::contract::init,
        oraiswap_token::contract::query,
    );
    Box::new(contract)
}

fn contract_pair() -> Box<dyn Contract> {
    let contract = ContractWrapper::new(
        oraiswap_pair::contract::handle,
        oraiswap_pair::contract::init,
        oraiswap_pair::contract::query,
    );
    Box::new(contract)
}

fn contract_oracle() -> Box<dyn Contract> {
    let contract = ContractWrapper::new(
        oraiswap_oracle::contract::handle,
        oraiswap_oracle::contract::init,
        oraiswap_oracle::contract::query,
    );
    Box::new(contract)
}

fn contract_factory() -> Box<dyn Contract> {
    let contract = ContractWrapper::new(handle, init, query);
    Box::new(contract)
}

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        oracle_addr: "oracle0000".into(),
        pair_code_id: 321u64,
        token_code_id: 123u64,
        commission_rate: None,
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config_res: ConfigResponse = from_binary(&query_res).unwrap();
    assert_eq!(123u64, config_res.token_code_id);
    assert_eq!(321u64, config_res.pair_code_id);
    assert_eq!("addr0000", config_res.owner.as_str());
}

#[test]
fn update_config() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        oracle_addr: "oracle0000".into(),
        pair_code_id: 321u64,
        token_code_id: 123u64,
        commission_rate: None,
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
    assert_eq!("addr0001", config_res.owner.as_str());

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
    assert_eq!("addr0001", config_res.owner.as_str());

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
        Err(err) => assert_eq!(err, ContractError::Unauthorized {}),
        _ => panic!("Must return unauthorized error"),
    }
}

#[test]
fn create_pair() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        oracle_addr: "oracle0000".into(),
        pair_code_id: 321u64,
        token_code_id: 123u64,
        commission_rate: None,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(deps.as_mut(), env, info, msg).unwrap();

    let asset_infos = [
        AssetInfo::Token {
            contract_addr: "asset0000".into(),
        },
        AssetInfo::Token {
            contract_addr: "asset0001".into(),
        },
    ];

    let msg = HandleMsg::CreatePair {
        asset_infos: asset_infos.clone(),
        auto_register: true,
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
                oracle_addr: "oracle0000".into(),
                asset_infos: asset_infos.clone(),
                token_code_id: 123u64,
                commission_rate: Some(DEFAULT_COMMISSION_RATE.to_string()),
                init_hook: Some(InitHook {
                    contract_addr: MOCK_CONTRACT_ADDR.into(),
                    msg: to_binary(&HandleMsg::Register {
                        asset_infos: asset_infos.clone(),
                    })
                    .unwrap(),
                }),
            })
            .unwrap(),
            code_id: 321u64,
            send: vec![],
            label: None,
        }
        .into()]
    );
}

#[test]
fn update_pair() {
    let mut app = MockApp::new();
    app.set_cw20_contract(contract_token());
    app.set_oracle_contract(contract_oracle());

    app.set_factory_and_pair_contract(contract_factory(), contract_pair());

    let contract_addr1 = app.create_token("assetA");
    let contract_addr2 = app.create_token("assetB");

    let asset_infos = [
        AssetInfo::Token {
            contract_addr: contract_addr1,
        },
        AssetInfo::Token {
            contract_addr: contract_addr2,
        },
    ];

    // create pair
    app.set_pair(asset_infos.clone());

    // should never change commission rate once deployed
    let pair_res = app.query_pair(asset_infos.clone()).unwrap();
    assert_eq!(
        pair_res,
        PairInfo {
            oracle_addr: app.oracle_addr,
            liquidity_token: "Contract #5".into(),
            contract_addr: "Contract #4".into(),
            asset_infos,
            commission_rate: DEFAULT_COMMISSION_RATE.into()
        }
    );
}
