use crate::contract::{handle, init, query};
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{attr, from_binary, Decimal, StdError, Uint128};
use oraiswap::asset::{AssetInfo, ORAI_DENOM};
use oraiswap::staking::{ConfigResponse, HandleMsg, InitMsg, PoolInfoResponse, QueryMsg};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        owner: Some("owner".into()),
        rewarder: "reward".into(),
        minter: Some("mint".into()),
        oracle_addr: "oracle".into(),
        factory_addr: "factory".into(),
        base_denom: None,
    };

    let info = mock_info("addr", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

    // it worked, let's query the state
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(
        ConfigResponse {
            owner: "owner".into(),
            rewarder: "reward".into(),
            minter: "mint".into(),
            oracle_addr: "oracle".into(),
            factory_addr: "factory".into(),
            base_denom: ORAI_DENOM.to_string(),
        },
        config
    );
}

#[test]
fn update_config() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        owner: Some("owner".into()),
        rewarder: "reward".into(),
        minter: Some("mint".into()),
        oracle_addr: "oracle".into(),
        factory_addr: "factory".into(),
        base_denom: None,
    };

    let info = mock_info("addr", &[]);
    let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

    // update owner
    let info = mock_info("owner", &[]);
    let msg = HandleMsg::UpdateConfig {
        owner: Some("owner2".into()),
        rewarder: None,
    };

    let res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(
        ConfigResponse {
            owner: "owner2".into(),
            rewarder: "reward".into(),
            minter: "mint".into(),
            oracle_addr: "oracle".into(),
            factory_addr: "factory".into(),
            base_denom: ORAI_DENOM.to_string(),
        },
        config
    );

    // unauthorized err
    let info = mock_info("owner", &[]);
    let msg = HandleMsg::UpdateConfig {
        rewarder: None,
        owner: None,
    };

    let res = handle(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "unauthorized"),
        _ => panic!("Must return unauthorized error"),
    }
}

#[test]
fn test_register() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        owner: Some("owner".into()),
        rewarder: "reward".into(),
        minter: Some("mint".into()),
        oracle_addr: "oracle".into(),
        factory_addr: "factory".into(),
        base_denom: None,
    };

    let info = mock_info("addr", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = HandleMsg::RegisterAsset {
        asset_info: AssetInfo::Token {
            contract_addr: "asset".into(),
        },
        staking_token: "staking".into(),
    };

    // failed with unauthorized error
    let info = mock_info("addr", &[]);
    let res = handle(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    match res {
        StdError::GenericErr { msg, .. } => assert_eq!(msg, "unauthorized"),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let info = mock_info("owner", &[]);
    let res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "register_asset"),
            attr("asset_info", "asset"),
        ]
    );

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::PoolInfo {
            asset_info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
        },
    )
    .unwrap();
    let pool_info: PoolInfoResponse = from_binary(&res).unwrap();
    assert_eq!(
        pool_info,
        PoolInfoResponse {
            asset_info: AssetInfo::Token {
                contract_addr: "asset".into()
            },
            staking_token: "staking".into(),
            total_bond_amount: Uint128::zero(),
            reward_index: Decimal::zero(),
            pending_reward: Uint128::zero(),
            migration_deprecated_staking_token: None,
            migration_index_snapshot: None,
        }
    );
}
