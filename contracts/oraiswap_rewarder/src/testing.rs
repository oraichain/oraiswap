use crate::contract::{instantiate, query_config};
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    Addr,
};
use oraiswap::rewarder::{ConfigResponse, InstantiateMsg};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();

    let msg = InstantiateMsg {
        staking_contract: Addr::unchecked("staking"),
        distribution_interval: Some(600),
    };

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), mock_info("owner", &[]), msg).unwrap();

    // it worked, let's query the state
    let contract_info = query_config(deps.as_ref()).unwrap();

    assert_eq!(
        contract_info,
        ConfigResponse {
            owner: Addr::unchecked("owner"),
            staking_contract: Addr::unchecked("staking"),
            distribution_interval: 600,
        }
    );
}
