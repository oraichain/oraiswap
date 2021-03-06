use std::collections::HashMap;

use cosmwasm_std::{
    to_binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, HandleResponse, HumanAddr,
    MessageInfo, StdError, StdResult, Uint128, WasmMsg,
};
use oraiswap::error::ContractError;

use crate::state::{Config, CONFIG};

use cw20::Cw20HandleMsg;
use oraiswap::asset::{Asset, AssetInfo, PairInfo};
use oraiswap::oracle::OracleContract;
use oraiswap::pair::HandleMsg as PairHandleMsg;
use oraiswap::querier::{query_pair_config, query_pair_info, query_token_balance};
use oraiswap::router::{HandleMsg, SwapOperation};

/// Execute swap operation
/// swap all offer asset to ask asset
pub fn handle_swap_operation(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    operation: SwapOperation,
    to: Option<HumanAddr>,
) -> Result<HandleResponse, ContractError> {
    if env.contract.address != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let config: Config = CONFIG.load(deps.storage)?;
    let factory_addr = deps.api.human_address(&config.factory_addr)?;
    let pair_config = query_pair_config(&deps.querier, factory_addr.clone())?;
    let oracle_contract = OracleContract(pair_config.oracle_addr.clone());

    let messages: Vec<CosmosMsg> = match operation {
        SwapOperation::OraiSwap {
            offer_asset_info,
            ask_asset_info,
        } => {
            let pair_info: PairInfo = query_pair_info(
                &deps.querier,
                factory_addr,
                &[offer_asset_info.clone(), ask_asset_info],
            )?;

            let amount = match offer_asset_info.clone() {
                AssetInfo::NativeToken { denom } => {
                    deps.querier
                        .query_balance(env.contract.address, &denom)?
                        .amount
                }
                AssetInfo::Token { contract_addr } => {
                    query_token_balance(&deps.querier, contract_addr.into(), env.contract.address)?
                }
            };
            let offer_asset: Asset = Asset {
                info: offer_asset_info,
                amount,
            };

            // swap token in smart contract
            vec![asset_into_swap_msg(
                deps.as_ref(),
                &oracle_contract,
                pair_info.contract_addr,
                offer_asset,
                None,
                to,
            )?]
        }
    };

    Ok(HandleResponse {
        messages,
        attributes: vec![],
        data: None,
    })
}

pub fn handle_swap_operations(
    deps: DepsMut,
    env: Env,
    sender: HumanAddr,
    operations: Vec<SwapOperation>,
    minimum_receive: Option<Uint128>,
    to: Option<HumanAddr>,
) -> Result<HandleResponse, ContractError> {
    let operations_len = operations.len();
    if operations_len == 0 {
        return Err(ContractError::NoSwapOperation {});
    }

    // Assert the operations are properly set
    assert_operations(&operations)?;

    let to = to.unwrap_or(sender);
    let target_asset_info = operations.last().unwrap().get_target_asset_info();

    let mut operation_index = 0;
    let mut messages: Vec<CosmosMsg> = operations
        .into_iter()
        .map(|op| {
            operation_index += 1;
            Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.clone(),
                send: vec![],
                msg: to_binary(&HandleMsg::ExecuteSwapOperation {
                    operation: op,
                    to: if operation_index == operations_len {
                        Some(to.clone())
                    } else {
                        None
                    },
                })?,
            }))
        })
        .collect::<StdResult<Vec<CosmosMsg>>>()?;

    // Execute minimum amount assertion
    if let Some(minimum_receive) = minimum_receive {
        let receiver_balance = target_asset_info.query_pool(&deps.querier, to.clone())?;

        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.clone(),
            send: vec![],
            msg: to_binary(&HandleMsg::AssertMinimumReceive {
                asset_info: target_asset_info,
                prev_balance: receiver_balance,
                minimum_receive,
                receiver: to,
            })?,
        }))
    }

    Ok(HandleResponse {
        messages,
        attributes: vec![],
        data: None,
    })
}

fn asset_into_swap_msg(
    deps: Deps,
    oracle_contract: &OracleContract,
    pair_contract: HumanAddr,
    offer_asset: Asset,
    max_spread: Option<Decimal>,
    to: Option<HumanAddr>,
) -> StdResult<CosmosMsg> {
    match offer_asset.info.clone() {
        AssetInfo::NativeToken { denom } => {
            let return_asset = Asset {
                info: AssetInfo::NativeToken {
                    denom: denom.clone(),
                },
                amount: offer_asset.amount,
            };

            // deduct tax first
            let amount = Asset::checked_sub(
                offer_asset.amount,
                return_asset.compute_tax(oracle_contract, &deps.querier)?,
            )?;

            Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: pair_contract,
                send: vec![Coin { denom, amount }],
                msg: to_binary(&PairHandleMsg::Swap {
                    offer_asset: Asset {
                        amount,
                        ..offer_asset
                    },
                    belief_price: None,
                    max_spread,
                    to,
                })?,
            }))
        }
        AssetInfo::Token { contract_addr } => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            send: vec![],
            msg: to_binary(&Cw20HandleMsg::Send {
                contract: pair_contract,
                amount: offer_asset.amount,
                msg: to_binary(&PairHandleMsg::Swap {
                    offer_asset,
                    belief_price: None,
                    max_spread,
                    to,
                })
                .ok(),
            })?,
        })),
    }
}

pub fn assert_operations(operations: &[SwapOperation]) -> StdResult<()> {
    let mut ask_asset_map: HashMap<String, bool> = HashMap::new();
    for operation in operations.iter() {
        let (offer_asset, ask_asset) = match operation {
            SwapOperation::OraiSwap {
                offer_asset_info,
                ask_asset_info,
            } => (offer_asset_info.clone(), ask_asset_info.clone()),
        };

        ask_asset_map.remove(&offer_asset.to_string());
        ask_asset_map.insert(ask_asset.to_string(), true);
    }

    if ask_asset_map.keys().len() != 1 {
        return Err(StdError::generic_err(
            "invalid operations; multiple output token",
        ));
    }

    Ok(())
}
