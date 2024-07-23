#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    coin, from_binary, to_binary, Addr, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Response, StdError, StdResult, Uint128, WasmMsg,
};
use oraiswap::error::ContractError;
use oraiswap_v3::interface::QuoteResult;
use oraiswap_v3::msg::QueryMsg as SwapV3QueryMsg;
use oraiswap_v3::sqrt_price::SqrtPrice;
use oraiswap_v3::token_amount::TokenAmount;
use oraiswap_v3::{MAX_TICK, MIN_TICK};

use crate::operations::{execute_swap_operation, execute_swap_operations};
use crate::state::{Config, CONFIG};

use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use oraiswap::asset::{Asset, AssetInfo, PairInfo};
use oraiswap::mixed_router::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
    SimulateSwapOperationsResponse, SwapOperation,
};
use oraiswap::oracle::OracleContract;
use oraiswap::pair::{QueryMsg as PairQueryMsg, SimulationResponse};
use oraiswap::querier::{query_pair_config, query_pair_info};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    CONFIG.save(
        deps.storage,
        &Config {
            factory_addr: deps.api.addr_canonicalize(msg.factory_addr.as_str())?,
            factory_addr_v2: deps.api.addr_canonicalize(msg.factory_addr_v2.as_str())?,
            oraiswap_v3: deps.api.addr_canonicalize(msg.oraiswap_v3.as_str())?,
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::ExecuteSwapOperations {
            operations,
            minimum_receive,
            to,
        } => execute_swap_operations(deps, env, info.sender, operations, minimum_receive, to),
        ExecuteMsg::ExecuteSwapOperation {
            operation,
            to,
            sender,
        } => execute_swap_operation(deps, env, info, operation, to, sender),

        ExecuteMsg::AssertMinimumReceiveAndTransfer {
            asset_info,
            minimum_receive,
            receiver,
        } => assert_minium_receive_and_transfer(
            deps.as_ref(),
            env,
            asset_info,
            minimum_receive,
            receiver,
        ),
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let sender = deps.api.addr_validate(&cw20_msg.sender)?;

    // throw empty data as well when decoding
    match from_binary(&cw20_msg.msg)? {
        Cw20HookMsg::ExecuteSwapOperations {
            operations,
            minimum_receive,
            to,
        } => {
            let receiver = to.and_then(|addr| deps.api.addr_validate(addr.as_str()).ok());
            execute_swap_operations(deps, env, sender, operations, minimum_receive, receiver)
        }
    }
}

fn assert_minium_receive_and_transfer(
    deps: Deps,
    env: Env,
    asset_info: AssetInfo,
    minium_receive: Uint128,
    receiver: Addr,
) -> Result<Response, ContractError> {
    let curr_balance = asset_info.query_pool(&deps.querier, env.contract.address)?;

    if curr_balance < minium_receive {
        return Err(ContractError::SwapAssertionFailure {
            minium_receive,
            swap_amount: curr_balance,
        });
    }

    // transfer to user
    let msg = match asset_info {
        AssetInfo::NativeToken { denom } => CosmosMsg::Bank(BankMsg::Send {
            to_address: receiver.to_string(),
            amount: vec![coin(curr_balance.into(), denom)],
        }),
        AssetInfo::Token { contract_addr } => CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: receiver.to_string(),
                amount: curr_balance,
            })?,
            funds: vec![],
        }),
    };

    Ok(Response::new().add_message(msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::SimulateSwapOperations {
            offer_amount,
            operations,
        } => to_binary(&simulate_swap_operations(deps, offer_amount, operations)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = CONFIG.load(deps.storage)?;
    let resp = ConfigResponse {
        factory_addr: deps.api.addr_humanize(&state.factory_addr)?,
        factory_addr_v2: deps.api.addr_humanize(&state.factory_addr_v2)?,
    };

    Ok(resp)
}

fn simulate_swap_operations(
    deps: Deps,
    offer_amount: Uint128,
    operations: Vec<SwapOperation>,
) -> StdResult<SimulateSwapOperationsResponse> {
    let config: Config = CONFIG.load(deps.storage)?;
    let oraiswap_v3 = deps.api.addr_humanize(&config.oraiswap_v3)?;
    let factory_addr = deps.api.addr_humanize(&config.factory_addr)?;
    let factory_addr_v2 = deps.api.addr_humanize(&config.factory_addr_v2)?;
    let operations_len = operations.len();
    if operations_len == 0 {
        return Err(StdError::generic_err(
            ContractError::NoSwapOperation {}.to_string(),
        ));
    }

    let mut offer_amount = offer_amount;
    for operation in operations.into_iter() {
        let pair_config = query_pair_config(&deps.querier, factory_addr.clone())
            .or_else(|_| query_pair_config(&deps.querier, factory_addr_v2.clone()))?;
        let oracle_contract = OracleContract(pair_config.oracle_addr);
        match operation {
            SwapOperation::OraiSwap {
                offer_asset_info,
                ask_asset_info,
            } => {
                let pair_info = query_pair_info(
                    &deps.querier,
                    factory_addr.clone(),
                    &[offer_asset_info.clone(), ask_asset_info.clone()],
                )
                .or_else(|_| -> StdResult<PairInfo> {
                    query_pair_info(
                        &deps.querier,
                        factory_addr_v2.clone(),
                        &[offer_asset_info.clone(), ask_asset_info.clone()],
                    )
                })?;

                let return_asset = Asset {
                    info: offer_asset_info.clone(),
                    amount: offer_amount,
                };

                // Deduct tax before querying simulation, with native token only
                offer_amount = offer_amount
                    .checked_sub(return_asset.compute_tax(&oracle_contract, &deps.querier)?)?;

                let mut res: SimulationResponse = deps.querier.query_wasm_smart(
                    pair_info.contract_addr,
                    &PairQueryMsg::Simulation {
                        offer_asset: Asset {
                            info: offer_asset_info,
                            amount: offer_amount,
                        },
                    },
                )?;

                let return_asset = Asset {
                    info: ask_asset_info,
                    amount: res.return_amount,
                };

                // Deduct tax after querying simulation, with native token only
                res.return_amount = res
                    .return_amount
                    .checked_sub(return_asset.compute_tax(&oracle_contract, &deps.querier)?)?;

                offer_amount = res.return_amount;
            }
            SwapOperation::SwapV3 { pool_key, x_to_y } => {
                // only support swap by_amount_in

                let sqrt_price_limit = if x_to_y {
                    SqrtPrice::from_tick(MIN_TICK).unwrap()
                } else {
                    SqrtPrice::from_tick(MAX_TICK).unwrap()
                };

                let res: QuoteResult = deps.querier.query_wasm_smart(
                    oraiswap_v3.to_string(),
                    &SwapV3QueryMsg::Quote {
                        pool_key,
                        x_to_y,
                        amount: TokenAmount(offer_amount.into()),
                        by_amount_in: true,
                        sqrt_price_limit,
                    },
                )?;

                offer_amount = Uint128::from(res.amount_out.0)
            }
        }
    }

    Ok(SimulateSwapOperationsResponse {
        amount: offer_amount,
    })
}