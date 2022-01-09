use cosmwasm_std::{
    Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo, StdError, StdResult,
};

use cw2::set_contract_version;
use cw20_base::{
    contract::{create_accounts, handle as cw20_handle, query as cw20_query},
    msg::{HandleMsg, QueryMsg},
    state::{token_info, MinterData, TokenInfo},
    ContractError,
};

use oraiswap::token::InitMsg;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw20-base";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn init(
    mut deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    // check valid token info
    msg.validate()?;

    // create initial accounts
    let total_supply = create_accounts(&mut deps, &msg.initial_balances)?;

    if let Some(limit) = msg.get_cap() {
        if total_supply > limit {
            return Err(StdError::generic_err("Initial supply greater than cap"));
        }
    }

    let mint = match msg.mint {
        Some(m) => Some(MinterData {
            minter: deps.api.canonical_address(&m.minter)?,
            cap: m.cap,
        }),
        None => None,
    };

    // store token info
    let data = TokenInfo {
        name: msg.name,
        symbol: msg.symbol,
        decimals: msg.decimals,
        total_supply,
        mint,
    };

    token_info(deps.storage).save(&data)?;
    Ok(InitResponse::default())
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    cw20_handle(deps, env, info, msg)
}

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    cw20_query(deps, env, msg)
}
