use cosmwasm_std::{
    Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo, MigrateResponse,
    StdError, StdResult, WasmMsg,
};

use cw2::set_contract_version;
use cw20_base::{
    contract::{
        create_accounts, handle as cw20_handle, migrate as cw20_migrate, query as cw20_query,
    },
    msg::{HandleMsg, MigrateMsg, QueryMsg},
    state::{token_info, MinterData, TokenInfo},
    ContractError,
};

use crate::{msg::InitMsg, state::tax_receiver, tax::handle_tax};

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

    // init tax receiver
    tax_receiver(deps.storage).save(&deps.api.canonical_address(&msg.tax_receiver)?)?;

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
    let msg_after_tax: HandleMsg = match msg {
        HandleMsg::Transfer { recipient, amount } => {
            let new_amount = handle_tax(deps.storage, amount)?;
            HandleMsg::Transfer {
                recipient,
                amount: new_amount,
            }
        }
        HandleMsg::Burn { amount } => {
            let new_amount = handle_tax(deps.storage, amount)?;
            HandleMsg::Burn { amount: new_amount }
        }
        HandleMsg::Send {
            contract,
            amount,
            msg,
        } => {
            let new_amount = handle_tax(deps.storage, amount)?;
            HandleMsg::Send {
                contract,
                amount: new_amount,
                msg,
            }
        }
        HandleMsg::Mint { recipient, amount } => {
            let new_amount = handle_tax(deps.storage, amount)?;
            HandleMsg::Mint {
                recipient,
                amount: new_amount,
            }
        }
        HandleMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => {
            let new_amount = handle_tax(deps.storage, amount)?;
            HandleMsg::IncreaseAllowance {
                spender,
                amount: new_amount,
                expires,
            }
        }
        HandleMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => {
            let new_amount = handle_tax(deps.storage, amount)?;
            HandleMsg::DecreaseAllowance {
                spender,
                amount: new_amount,
                expires,
            }
        }
        HandleMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => {
            let new_amount = handle_tax(deps.storage, amount)?;
            HandleMsg::TransferFrom {
                owner,
                recipient,
                amount: new_amount,
            }
        }
        HandleMsg::BurnFrom { owner, amount } => {
            let new_amount = handle_tax(deps.storage, amount)?;
            HandleMsg::BurnFrom {
                owner,
                amount: new_amount,
            }
        }
        HandleMsg::SendFrom {
            owner,
            contract,
            amount,
            msg,
        } => {
            let new_amount = handle_tax(deps.storage, amount)?;
            HandleMsg::SendFrom {
                owner,
                contract,
                amount: new_amount,
                msg,
            }
        }
    };

    cw20_handle(deps, env, info, msg_after_tax)
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
