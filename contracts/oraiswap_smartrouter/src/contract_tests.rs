use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{from_binary, Addr, Coin, DepsMut};

use crate::contract;
use oraiswap::smartrouter::{ExecuteMsg, GetConfigResponse, InstantiateMsg, QueryMsg};

static CREATOR_ADDRESS: &str = "creator";

// test helper
#[allow(unused_assignments)]
fn initialize_contract(deps: DepsMut) -> Addr {
    let msg = InstantiateMsg {
        owner: String::from(CREATOR_ADDRESS),
        router_addr: "router_addr".to_string(),
    };
    let info = mock_info(CREATOR_ADDRESS, &[]);

    // instantiate with enough funds provided should succeed
    contract::instantiate(deps, mock_env(), info.clone(), msg).unwrap();

    info.sender
}

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();

    let owner = initialize_contract(deps.as_mut());

    // it worked, let's query the state
    let res: GetConfigResponse =
        from_binary(&contract::query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap())
            .unwrap();
    assert_eq!(owner, res.owner);
}

#[test]
fn proper_update_state() {
    let mut deps = mock_dependencies();

    let owner = initialize_contract(deps.as_mut());

    // it worked, let's query the state
    let res: GetConfigResponse =
        from_binary(&contract::query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap())
            .unwrap();
    assert_eq!(owner, res.owner);

    let good_addr = "new_owner".to_string();

    let other_info = mock_info("other_sender", &vec![] as &Vec<Coin>);
    let owner_info = mock_info(owner.as_str(), &vec![] as &Vec<Coin>);

    // valid addr, bad sender
    let msg = ExecuteMsg::UpdateConfig {
        new_owner: Some(good_addr.clone()),
        new_router: None,
    };
    contract::execute(deps.as_mut(), mock_env(), other_info, msg).unwrap_err();

    // and transfer ownership
    let msg = ExecuteMsg::UpdateConfig {
        new_owner: Some(good_addr.clone()),
        new_router: Some("new_router".to_string()),
    };
    contract::execute(deps.as_mut(), mock_env(), owner_info, msg).unwrap();

    let res: GetConfigResponse =
        from_binary(&contract::query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap())
            .unwrap();
    assert_eq!(good_addr, res.owner);
    assert_eq!(res.router, "new_router");
}
