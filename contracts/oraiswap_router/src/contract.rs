use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, QueryRequest, Response,
    Response, StdError, StdResult, Uint128, WasmQuery,
};
use oraiswap::error::ContractError;

use crate::operations::{handle_swap_operation, handle_swap_operations};
use crate::state::{Config, CONFIG};

use cw20::Cw20ReceiveMsg;
use oraiswap::asset::{Asset, AssetInfo, PairInfo};
use oraiswap::oracle::OracleContract;
use oraiswap::pair::{QueryMsg as PairQueryMsg, SimulationResponse};
use oraiswap::querier::{query_pair_config, query_pair_info};
use oraiswap::router::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg,
    SimulateSwapOperationsResponse, SwapOperation,
};

pub fn init(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    CONFIG.save(
        deps.storage,
        &Config {
            factory_addr: deps.api.addr_canonicalize(&msg.factory_addr)?,
        },
    )?;

    Ok(Response::default())
}

pub fn handle(
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
        } => handle_swap_operations(deps, env, info.sender, operations, minimum_receive, to),
        ExecuteMsg::ExecuteSwapOperation { operation, to } => {
            handle_swap_operation(deps, env, info, operation, to)
        }

        ExecuteMsg::AssertMinimumReceive {
            asset_info,
            prev_balance,
            minimum_receive,
            receiver,
        } => assert_minium_receive(
            deps.as_ref(),
            asset_info,
            prev_balance,
            minimum_receive,
            receiver.into(),
        ),
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    // throw empty data as well when decoding
    match from_binary(&cw20_msg.msg.unwrap_or_default())? {
        Cw20HookMsg::ExecuteSwapOperations {
            operations,
            minimum_receive,
            to,
        } => handle_swap_operations(
            deps,
            env,
            cw20_msg.sender,
            operations,
            minimum_receive,
            to.map(Addr),
        ),
    }
}

fn assert_minium_receive(
    deps: Deps,
    asset_info: AssetInfo,
    prev_balance: Uint128,
    minium_receive: Uint128,
    receiver: Addr,
) -> Result<Response, ContractError> {
    let receiver_balance = asset_info.query_pool(&deps.querier, receiver)?;
    let swap_amount = Asset::checked_sub(receiver_balance, prev_balance)?;

    if swap_amount < minium_receive {
        return Err(ContractError::SwapAssertionFailure {
            minium_receive,
            swap_amount,
        });
    }

    Ok(Response::default())
}

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
    };

    Ok(resp)
}

fn simulate_swap_operations(
    deps: Deps,
    offer_amount: Uint128,
    operations: Vec<SwapOperation>,
) -> StdResult<SimulateSwapOperationsResponse> {
    let config: Config = CONFIG.load(deps.storage)?;
    let factory_addr = deps.api.addr_humanize(&config.factory_addr)?;
    let pair_config = query_pair_config(&deps.querier, factory_addr.clone())?;
    let oracle_contract = OracleContract(pair_config.oracle_addr);

    let operations_len = operations.len();
    if operations_len == 0 {
        return Err(StdError::generic_err(
            ContractError::NoSwapOperation {}.to_string(),
        ));
    }

    let mut offer_amount = offer_amount;
    for operation in operations.into_iter() {
        match operation {
            SwapOperation::OraiSwap {
                offer_asset_info,
                ask_asset_info,
            } => {
                let pair_info: PairInfo = query_pair_info(
                    &deps.querier,
                    factory_addr.clone(),
                    &[offer_asset_info.clone(), ask_asset_info.clone()],
                )?;

                let return_asset = Asset {
                    info: offer_asset_info.clone(),
                    amount: offer_amount,
                };

                // Deduct tax before querying simulation, with native token only
                offer_amount = Asset::checked_sub(
                    offer_amount,
                    return_asset.compute_tax(&oracle_contract, &deps.querier)?,
                )?;

                let mut res: SimulationResponse =
                    deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                        contract_addr: pair_info.contract_addr.clone(),
                        msg: to_binary(&PairQueryMsg::Simulation {
                            offer_asset: Asset {
                                info: offer_asset_info,
                                amount: offer_amount,
                            },
                        })?,
                    }))?;

                let return_asset = Asset {
                    info: ask_asset_info,
                    amount: res.return_amount,
                };

                // Deduct tax after querying simulation, with native token only
                res.return_amount = Asset::checked_sub(
                    res.return_amount,
                    return_asset.compute_tax(&oracle_contract, &deps.querier)?,
                )?;

                offer_amount = res.return_amount;
            }
        }
    }

    Ok(SimulateSwapOperationsResponse {
        amount: offer_amount,
    })
}
