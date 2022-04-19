use crate::contract::{init, query_config};
use cosmwasm_std::testing::{mock_env, mock_info};
use oraiswap::mock_querier::mock_dependencies;
use oraiswap::rewarder::{ConfigResponse, InitMsg};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        staking_contract: "staking".into(),
        distribution_interval: Some(600),
    };

    // we can just call .unwrap() to assert this was a success
    let _res = init(deps.as_mut(), mock_env(), mock_info("owner", &[]), msg).unwrap();

    // it worked, let's query the state
    let contract_info = query_config(deps.as_ref()).unwrap();

    assert_eq!(
        contract_info,
        ConfigResponse {
            owner: "owner".into(),
            staking_contract: "staking".into(),
            distribution_interval: 600,
        }
    );
}
