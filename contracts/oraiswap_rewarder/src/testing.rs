use crate::contract::{handle, init, query, query_config};
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{to_binary, Uint128, WasmMsg};
use oraiswap::asset::{Asset, AssetInfo};
use oraiswap::mock_querier::mock_dependencies;
use oraiswap::rewarder::{
    ConfigResponse, DistributionInfoResponse, HandleMsg, InitMsg, QueryMsg, RewardPerSecondResponse,
};
use oraiswap::staking::HandleMsg as StakingHandleMsg;

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

#[test]
fn test_distribute() {
    let mut deps = mock_dependencies(&[]);

    let msg = InitMsg {
        staking_contract: "staking".into(),
        distribution_interval: Some(600),
    };

    // we can just call .unwrap() to assert this was a success
    let _res = init(deps.as_mut(), mock_env(), mock_info("owner", &[]), msg).unwrap();

    // update distribute reward for a pool
    let msg = HandleMsg::UpdateRewardPerSec {
        reward: Asset {
            info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
            amount: 100u128.into(),
        },
    };
    let _res = handle(deps.as_mut(), mock_env(), mock_info("owner", &[]), msg).unwrap();

    let rw_per_sec = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::RewardPerSec {
            asset_info: AssetInfo::Token {
                contract_addr: "asset".into(),
            },
        },
    );
    assert_eq!(
        rw_per_sec,
        to_binary(&RewardPerSecondResponse {
            reward: Asset {
                info: AssetInfo::Token {
                    contract_addr: "asset".into(),
                },
                amount: 100u128.into()
            }
        })
    );

    // now distribute reward
    let msg = HandleMsg::Distribute {};
    let mut new_env = mock_env();
    let elapsed_time: u64 = 1000;
    new_env.block.time += elapsed_time;
    let res = handle(deps.as_mut(), new_env, mock_info("owner", &[]), msg).unwrap();
    assert_eq!(
        res.messages,
        vec![WasmMsg::Execute {
            contract_addr: "staking".into(),
            msg: to_binary(&StakingHandleMsg::DepositReward {
                rewards: vec![Asset {
                    info: AssetInfo::Token {
                        contract_addr: "asset".into()
                    },
                    amount: Uint128(elapsed_time as u128 * 100u128),
                }]
            })
            .unwrap(),
            send: vec![],
        }
        .into()]
    );

    let last_distribute = query(deps.as_ref(), mock_env(), QueryMsg::DistributionInfo {});
    assert_eq!(
        last_distribute,
        to_binary(&DistributionInfoResponse {
            last_distributed: mock_env().block.time + elapsed_time
        })
    );
}
