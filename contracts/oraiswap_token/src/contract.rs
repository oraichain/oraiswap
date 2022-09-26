use cosmwasm_std::{
    attr, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo, MigrateResponse,
    StdError, StdResult, WasmMsg,
};

use cw2::set_contract_version;
use cw20_base::{
    contract::{
        create_accounts, handle as cw20_handle, migrate as cw20_migrate, query as cw20_query,
    },
    msg::{HandleMsg as OriginalHandleMsg, MigrateMsg, QueryMsg},
    state::{token_info, token_info_read, MinterData, TokenInfo},
    ContractError,
};

use oraiswap::token::InitMsg;

use crate::msg::{HandleMsg, MinterDataMsg};

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

    // Check supply cap
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

    // do hook ?
    if let Some(hook) = msg.init_hook {
        Ok(InitResponse {
            messages: vec![WasmMsg::Execute {
                contract_addr: hook.contract_addr,
                msg: hook.msg,
                send: vec![],
            }
            .into()],
            attributes: vec![],
        })
    } else {
        Ok(InitResponse::default())
    }
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::ChangeMinter { new_minter } => handle_change_minter(deps, env, info, new_minter),
        HandleMsg::Mint { recipient, amount } => cw20_handle(
            deps,
            env,
            info,
            OriginalHandleMsg::Mint { recipient, amount },
        ),
        HandleMsg::Burn { amount } => {
            cw20_handle(deps, env, info, OriginalHandleMsg::Burn { amount })
        }
        HandleMsg::BurnFrom { owner, amount } => cw20_handle(
            deps,
            env,
            info,
            OriginalHandleMsg::BurnFrom { owner, amount },
        ),
        HandleMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => cw20_handle(
            deps,
            env,
            info,
            OriginalHandleMsg::DecreaseAllowance {
                spender,
                amount,
                expires,
            },
        ),
        HandleMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => cw20_handle(
            deps,
            env,
            info,
            OriginalHandleMsg::IncreaseAllowance {
                spender,
                amount,
                expires,
            },
        ),
        HandleMsg::Send {
            contract,
            amount,
            msg,
        } => cw20_handle(
            deps,
            env,
            info,
            OriginalHandleMsg::Send {
                contract,
                amount,
                msg,
            },
        ),
        HandleMsg::SendFrom {
            owner,
            contract,
            amount,
            msg,
        } => cw20_handle(
            deps,
            env,
            info,
            OriginalHandleMsg::SendFrom {
                owner,
                contract,
                amount,
                msg,
            },
        ),
        HandleMsg::Transfer { recipient, amount } => cw20_handle(
            deps,
            env,
            info,
            OriginalHandleMsg::Transfer { recipient, amount },
        ),
        HandleMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => cw20_handle(
            deps,
            env,
            info,
            OriginalHandleMsg::TransferFrom {
                owner,
                recipient,
                amount,
            },
        ),
    }
}

pub fn handle_change_minter(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_minter: MinterDataMsg,
) -> Result<HandleResponse, ContractError> {
    let mut config = token_info_read(deps.storage).load()?;
    if config.mint.is_none()
        || config.mint.as_ref().unwrap().minter != deps.api.canonical_address(&info.sender)?
    {
        return Err(ContractError::Unauthorized {});
    }

    config.mint = Some(MinterData {
        minter: deps.api.canonical_address(&new_minter.minter)?,
        cap: new_minter.cap,
    });

    token_info(deps.storage).save(&config)?;

    let res = HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "change_minter"),
            attr("new_minter", new_minter.minter),
        ],
        data: None,
    };
    Ok(res)
}

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    cw20_query(deps, env, msg)
}

pub fn migrate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: MigrateMsg,
) -> StdResult<MigrateResponse> {
    cw20_migrate(deps, env, info, msg)
}
