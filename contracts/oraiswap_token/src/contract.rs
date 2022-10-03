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

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        DepsMut, HumanAddr, Uint128,
    };
    use cw20::{Cw20CoinHuman, MinterResponse, TokenInfoResponse};
    use cw20_base::{
        contract::{query_balance, query_minter, query_token_info},
        ContractError,
    };
    use oraiswap::token::InitMsg;

    use crate::{
        contract::handle,
        contract::init,
        msg::{HandleMsg, MinterDataMsg},
    };

    // this will set up the init for other tests
    fn do_init_with_minter(
        deps: DepsMut,
        addr: &HumanAddr,
        amount: Uint128,
        minter: &HumanAddr,
        cap: Option<Uint128>,
    ) -> TokenInfoResponse {
        _do_init(
            deps,
            addr,
            amount,
            Some(MinterResponse {
                minter: minter.into(),
                cap,
            }),
        )
    }

    // this will set up the init for other tests
    fn _do_init(
        mut deps: DepsMut,
        addr: &HumanAddr,
        amount: Uint128,
        mint: Option<MinterResponse>,
    ) -> TokenInfoResponse {
        let init_msg = InitMsg {
            name: "Auto Gen".to_string(),
            symbol: "AUTO".to_string(),
            decimals: 3,
            initial_balances: vec![Cw20CoinHuman {
                address: addr.into(),
                amount,
            }],
            mint: mint.clone(),
            init_hook: None,
        };
        let info = mock_info(&HumanAddr("creator".to_string()), &[]);
        let env = mock_env();
        let res = init(deps.branch(), env, info, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        let meta = query_token_info(deps.as_ref()).unwrap();
        assert_eq!(
            meta,
            TokenInfoResponse {
                name: "Auto Gen".to_string(),
                symbol: "AUTO".to_string(),
                decimals: 3,
                total_supply: amount,
            }
        );
        assert_eq!(
            query_balance(deps.as_ref(), addr.into()).unwrap().balance,
            amount
        );
        assert_eq!(query_minter(deps.as_ref()).unwrap(), mint);
        meta
    }

    #[test]
    fn test_change_minter() {
        let mut deps = mock_dependencies(&[]);
        let minter = HumanAddr::from("minter");
        do_init_with_minter(
            deps.as_mut(),
            &HumanAddr::from("genesis"),
            Uint128(1234),
            &minter,
            None,
        );

        let msg = HandleMsg::ChangeMinter {
            new_minter: MinterDataMsg {
                minter: HumanAddr("new_minter".to_string()),
                cap: None,
            },
        };

        // unauthorized, only minter can change minter
        let info = mock_info(&HumanAddr::from("genesis"), &[]);
        let env = mock_env();
        let res = handle(deps.as_mut(), env.clone(), info, msg.clone());
        match res.unwrap_err() {
            ContractError::Unauthorized { .. } => {}
            e => panic!("expected unauthorized error, got {}", e),
        }

        // valid case. Minter can change minter
        let info = mock_info(&minter, &[]);
        handle(deps.as_mut(), env, info, msg.clone()).unwrap();

        // query new minter
        let new_minter = query_minter(deps.as_ref()).unwrap().unwrap();

        assert_eq!(new_minter.minter, HumanAddr("new_minter".to_string()));
    }
}
