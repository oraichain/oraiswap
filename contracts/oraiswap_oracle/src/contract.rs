use cosmwasm_std::entry_point;

use cosmwasm_std::{
    to_binary, Addr, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Uint128,
};

use oraiswap::asset::{DECIMAL_FRACTION, ORAI_DENOM};
use oraiswap::oracle::{
    ContractInfo, ContractInfoResponse, ExchangeRateItem, ExchangeRateResponse,
    ExchangeRatesResponse, MigrateMsg, OracleMsg, OracleQuery, RewardPoolResponse, TaxCapResponse,
    TaxRateResponse,
};

use oraiswap::error::ContractError;
use oraiswap::oracle::InstantiateMsg;

// use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::{CONTRACT_INFO, EXCHANGE_RATES, TAX_CAP, TAX_RATE};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:oraiswap_oracle";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// whitelist of denom?
// base on denom address as ow20 can call burn
#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    msg_info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let creator = deps.api.addr_canonicalize(msg_info.sender.as_str())?;
    let info = ContractInfo {
        name: msg.name.unwrap_or(CONTRACT_NAME.to_string()),
        version: msg.version.unwrap_or(CONTRACT_VERSION.to_string()),
        creator: creator.clone(),
        // admin should be multisig
        admin: if let Some(admin) = msg.admin {
            deps.api.addr_canonicalize(admin.as_str())?
        } else {
            creator
        },
        min_rate: msg
            .min_rate
            .unwrap_or(Decimal::from_ratio(5u128, 10000u128)), // 0.05%
        max_rate: msg.max_rate.unwrap_or(Decimal::percent(1)), // 1%
    };
    CONTRACT_INFO.save(deps.storage, &info)?;

    // defaul is orai/orai 1:1 (no tax), this is for swap Orai native to Orai token
    EXCHANGE_RATES.save(deps.storage, ORAI_DENOM.as_bytes(), &Decimal::one())?;

    // return default
    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: OracleMsg,
) -> Result<Response, ContractError> {
    match msg {
        OracleMsg::UpdateExchangeRate {
            denom,
            exchange_rate,
        } => handle_update_exchange_rate(deps, info, denom, exchange_rate),
        OracleMsg::DeleteExchangeRate { denom } => handle_delete_exchange_rate(deps, info, denom),
        OracleMsg::UpdateTaxCap { cap, denom } => handle_update_tax_cap(deps, info, denom, cap),
        OracleMsg::UpdateTaxRate { rate } => handle_update_tax_rate(deps, info, rate),
        OracleMsg::UpdateAdmin { admin } => handle_update_admin(deps, info, admin),
    }
}

pub fn handle_update_tax_cap(
    deps: DepsMut,
    info: MessageInfo,
    denom: String,
    cap: Uint128,
) -> Result<Response, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    let sender_addr = deps.api.addr_canonicalize(info.sender.as_str())?;

    // check authorized
    if contract_info.admin.ne(&sender_addr) {
        return Err(ContractError::Unauthorized {});
    }

    // update tax cap
    TAX_CAP.save(deps.storage, denom.as_bytes(), &cap)?;

    // return nothing new
    Ok(Response::default())
}

pub fn handle_update_tax_rate(
    deps: DepsMut,
    info: MessageInfo,
    rate: Decimal,
) -> Result<Response, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    let sender_addr = deps.api.addr_canonicalize(info.sender.as_str())?;

    // check authorized, TODO: min and max tax_rate
    if contract_info.admin.ne(&sender_addr) {
        return Err(ContractError::Unauthorized {});
    }

    // update tax cap
    let rate = rate.clamp(contract_info.min_rate, contract_info.max_rate);
    TAX_RATE.save(deps.storage, &rate)?;

    // return nothing new
    Ok(Response::default())
}

pub fn handle_update_admin(
    deps: DepsMut,
    info: MessageInfo,
    admin: Addr,
) -> Result<Response, ContractError> {
    let mut contract_info = CONTRACT_INFO.load(deps.storage)?;
    let sender_addr = deps.api.addr_canonicalize(info.sender.as_str())?;

    // check authorized
    if contract_info.admin.ne(&sender_addr) {
        return Err(ContractError::Unauthorized {});
    }

    // update new admin
    contract_info.admin = deps.api.addr_canonicalize(admin.as_str())?;
    CONTRACT_INFO.save(deps.storage, &contract_info)?;

    // return nothing new
    Ok(Response::default())
}

pub fn handle_update_exchange_rate(
    deps: DepsMut,
    info: MessageInfo,
    denom: String,
    exchange_rate: Decimal,
) -> Result<Response, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    let sender_addr = deps.api.addr_canonicalize(info.sender.as_str())?;

    // check authorized
    if contract_info.admin.ne(&sender_addr) {
        return Err(ContractError::Unauthorized {});
    }

    EXCHANGE_RATES.save(deps.storage, denom.as_bytes(), &exchange_rate)?;

    Ok(Response::default())
}

pub fn handle_delete_exchange_rate(
    deps: DepsMut,
    info: MessageInfo,
    denom: String,
) -> Result<Response, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    let sender_addr = deps.api.addr_canonicalize(info.sender.as_str())?;

    // check authorized
    if contract_info.admin.ne(&sender_addr) {
        return Err(ContractError::Unauthorized {});
    }

    EXCHANGE_RATES.remove(deps.storage, denom.as_bytes());

    Ok(Response::default())
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: OracleQuery) -> StdResult<Binary> {
    match msg {
        OracleQuery::TaxRate {} => to_binary(&query_tax_rate(deps)?),
        OracleQuery::TaxCap { denom } => to_binary(&query_tax_cap(deps, denom)?),
        OracleQuery::ExchangeRate {
            base_denom,
            quote_denom,
        } => to_binary(&query_exchange_rate(
            deps,
            base_denom.unwrap_or(ORAI_DENOM.to_string()),
            quote_denom,
        )?),
        OracleQuery::ExchangeRates {
            base_denom,
            quote_denoms,
        } => to_binary(&query_exchange_rates(
            deps,
            base_denom.unwrap_or(ORAI_DENOM.to_string()),
            quote_denoms,
        )?),
        OracleQuery::ContractInfo {} => to_binary(&query_contract_info(deps)?),
        OracleQuery::RewardPool { denom } => to_binary(&query_reward_pool(deps, env, denom)?),
    }
}

pub fn query_tax_rate(deps: Deps) -> StdResult<TaxRateResponse> {
    if let Ok(Some(rate)) = TAX_RATE.may_load(deps.storage) {
        return Ok(TaxRateResponse { rate });
    }

    Err(StdError::NotFound {
        kind: "Tax rate not set".to_string(),
    })
}

pub fn query_tax_cap(deps: Deps, denom: String) -> StdResult<TaxCapResponse> {
    if let Ok(Some(cap)) = TAX_CAP.may_load(deps.storage, denom.as_bytes()) {
        return Ok(TaxCapResponse { cap });
    }

    Err(StdError::NotFound {
        kind: format!("Tax cap not found for denom: {}", denom),
    })
}

pub fn query_exchange_rate(
    deps: Deps,
    base_denom: String,
    quote_denom: String,
) -> StdResult<ExchangeRateResponse> {
    let base_rate = get_orai_exchange_rate(deps, &base_denom)?;
    let quote_rate = get_orai_exchange_rate(deps, &quote_denom)?;
    let fraction = Uint128::from(DECIMAL_FRACTION);
    // quote = ask, offer = base
    let exchange_rate = Decimal::from_ratio(fraction * quote_rate, fraction * base_rate);

    let res = ExchangeRateResponse {
        base_denom: base_denom.clone(),
        item: ExchangeRateItem {
            quote_denom,
            exchange_rate,
        },
    };

    Ok(res)
}

pub fn query_exchange_rates(
    deps: Deps,
    base_denom: String,
    quote_denoms: Vec<String>,
) -> StdResult<ExchangeRatesResponse> {
    let mut res = ExchangeRatesResponse {
        base_denom: base_denom.clone(),
        items: vec![],
    };

    let base_rate = get_orai_exchange_rate(deps, &base_denom)?;
    let fraction = Uint128::from(DECIMAL_FRACTION);

    for quote_denom in quote_denoms {
        let quote_rate = get_orai_exchange_rate(deps, &quote_denom)?;

        let exchange_rate = Decimal::from_ratio(fraction * quote_rate, fraction * base_rate);

        res.items.push(ExchangeRateItem {
            quote_denom,
            exchange_rate,
        });
    }

    Ok(res)
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfoResponse> {
    let info = CONTRACT_INFO.load(deps.storage)?;
    Ok(ContractInfoResponse {
        version: info.version,
        name: info.name,
        admin: deps.api.addr_humanize(&info.admin)?,
        creator: deps.api.addr_humanize(&info.creator)?,
        min_rate: info.min_rate,
        max_rate: info.max_rate,
    })
}

/// query_reward_pool: return native balance, currently only Orai denom
pub fn query_reward_pool(deps: Deps, env: Env, denom: String) -> StdResult<RewardPoolResponse> {
    deps.querier
        .query_balance(env.contract.address, &denom)
        .map(|balance| RewardPoolResponse { balance })
}

fn get_orai_exchange_rate(deps: Deps, denom: &str) -> StdResult<Decimal> {
    if denom == ORAI_DENOM {
        return Ok(Decimal::one());
    }

    EXCHANGE_RATES.load(deps.storage, denom.as_bytes())
}

#[entry_point]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}
