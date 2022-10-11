use cosmwasm_std::{
    attr, to_binary, Attribute, Binary, Deps, DepsMut, Env, HandleResponse, HumanAddr,
    InitResponse, MessageInfo, MigrateResponse, StdError, StdResult, WasmMsg,
};

use cw2::set_contract_version;
use cw20_base::{
    contract::{
        create_accounts, handle as cw20_handle, migrate as cw20_migrate, query as cw20_query,
        query_token_info,
    },
    msg::{HandleMsg as OriginalHandleMsg, MigrateMsg, QueryMsg},
    state::{token_info, token_info_read, MinterData, TokenInfo},
    ContractError,
};

use crate::{
    msg::{HandleMsg, InitMsg, MinterDataMsg, NewTokenInfoResponse},
    state::{router_contract, router_contract_read, tax_receiver, tax_receiver_read},
    tax::handle_tax,
};

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
    router_contract(deps.storage).save(&deps.api.canonical_address(&msg.router_contract)?)?;
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
        HandleMsg::Send {
            amount,
            msg,
            contract,
        } => {
            let tax_receiver = tax_receiver_read(deps.storage).load()?;
            let router_contract = router_contract_read(deps.storage).load()?;

            // if call from this contract & send token to router contract, then caller is trying to sell / swap this token to another token => apply tax
            let new_amount = if deps.api.canonical_address(&contract)?.eq(&router_contract) {
                handle_tax(deps.storage, tax_receiver, amount)?
            } else {
                amount
            };

            cw20_handle(
                deps,
                env,
                info,
                OriginalHandleMsg::Send {
                    msg,
                    amount: new_amount,
                    contract,
                },
            )
        }
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
        HandleMsg::ChangeTaxInfo {
            new_tax_receiver,
            new_router_contract,
        } => handle_change_tax_info(deps, env, info, new_tax_receiver, new_router_contract),
    }
}

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::TokenInfo {} => to_binary(&new_query_token_info(deps)?),
        _ => cw20_query(deps, env, msg),
    }
}

pub fn migrate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: MigrateMsg,
) -> StdResult<MigrateResponse> {
    cw20_migrate(deps, env, info, msg)
}

pub fn handle_change_tax_info(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_tax_receiver: Option<HumanAddr>,
    new_router_contract: Option<HumanAddr>,
) -> Result<HandleResponse, ContractError> {
    let config = token_info_read(deps.storage).load()?;
    if config.mint.is_none()
        || config.mint.as_ref().unwrap().minter != deps.api.canonical_address(&info.sender)?
    {
        return Err(ContractError::Unauthorized {});
    }

    let mut attributes: Vec<Attribute> = vec![attr("action", "change_tax_info")];

    if let Some(new_tax_receiver) = new_tax_receiver {
        tax_receiver(deps.storage).save(&deps.api.canonical_address(&new_tax_receiver)?)?;
        attributes.push(attr("new_tax_receiver", new_tax_receiver));
    };

    if let Some(new_router_contract) = new_router_contract {
        router_contract(deps.storage).save(&deps.api.canonical_address(&new_router_contract)?)?;
        attributes.push(attr("new_router_contract", new_router_contract));
    };

    let res = HandleResponse {
        messages: vec![],
        attributes,
        data: None,
    };
    Ok(res)
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

pub fn new_query_token_info(deps: Deps) -> StdResult<NewTokenInfoResponse> {
    let token_info_response = query_token_info(deps)?;
    let tax_receiver = deps
        .api
        .human_address(&tax_receiver_read(deps.storage).load()?)?;
    let router_contract = deps
        .api
        .human_address(&router_contract_read(deps.storage).load()?)?;
    Ok(NewTokenInfoResponse {
        token_info_response,
        tax_receiver,
        router_contract,
    })
}
