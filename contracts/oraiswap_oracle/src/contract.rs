use std::ops::Mul;

use cosmwasm_std::{
    to_binary, Binary, Coin, Decimal, Deps, DepsMut, Env, HandleResponse, HumanAddr, InitResponse,
    MessageInfo, StdResult, Uint128,
};

use oraiswap::asset::{DECIMAL_FRACTION, ORAI_DENOM};
use oraiswap::oracle::{
    ContractInfo, ContractInfoResponse, ExchangeRateItem, ExchangeRateResponse,
    ExchangeRatesResponse, OracleContractMsg, OracleContractQuery, OracleExchangeMsg,
    OracleExchangeQuery, OracleMsg, OracleQuery, OracleTreasuryMsg, OracleTreasuryQuery,
    TaxCapResponse, TaxRateResponse,
};

use oraiswap::error::ContractError;
use oraiswap::oracle::InitMsg;

// use crate::msg::{HandleMsg, InitMsg};
use crate::state::{CONTRACT_INFO, EXCHANGE_RATES, TAX_CAP, TAX_RATE};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:oraiswap_oracle";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// whitelist of denom?
// base on denom address as ow20 can call burn
pub fn init(
    deps: DepsMut,
    _env: Env,
    msg_info: MessageInfo,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let creator = deps.api.canonical_address(&msg_info.sender)?;
    let info = ContractInfo {
        name: msg.name.unwrap_or(CONTRACT_NAME.to_string()),
        version: msg.version.unwrap_or(CONTRACT_VERSION.to_string()),
        creator: creator.clone(),
        // admin should be multisig
        admin: if let Some(admin) = msg.admin {
            deps.api.canonical_address(&admin)?
        } else {
            creator
        },
        min_rate: msg
            .min_rate
            .unwrap_or(Decimal::from_ratio(5u128, 10000u128)), // 0.05%
        max_rate: msg.max_rate.unwrap_or(Decimal::percent(1)), // 1%
    };
    CONTRACT_INFO.save(deps.storage, &info)?;

    // defaul is orai/orai 1:1 (no tax)
    EXCHANGE_RATES.save(deps.storage, ORAI_DENOM.as_bytes(), &Decimal::one())?;

    // return default
    Ok(InitResponse::default())
}

pub fn handle(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: OracleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        OracleMsg::Exchange(handle_data) => match handle_data {
            OracleExchangeMsg::UpdateExchangeRate {
                denom,
                exchange_rate,
            } => handle_update_exchange_rate(deps, info, denom, exchange_rate),
            OracleExchangeMsg::DeleteExchangeRate { denom } => {
                handle_delete_exchange_rate(deps, info, denom)
            }
        },
        OracleMsg::Treasury(handle_data) => match handle_data {
            OracleTreasuryMsg::UpdateTaxCap { cap, denom } => {
                handle_update_tax_cap(deps, info, denom, cap)
            }
            OracleTreasuryMsg::UpdateTaxRate { rate } => handle_update_tax_rate(deps, info, rate),
        },
        OracleMsg::Contract(handle_data) => match handle_data {
            OracleContractMsg::UpdateAdmin { admin } => handle_update_admin(deps, info, admin),
        },
    }
}

pub fn handle_update_tax_cap(
    deps: DepsMut,
    info: MessageInfo,
    denom: String,
    cap: Uint128,
) -> Result<HandleResponse, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    let sender_addr = deps.api.canonical_address(&info.sender)?;

    // check authorized
    if contract_info.admin.ne(&sender_addr) {
        return Err(ContractError::Unauthorized {});
    }

    // update tax cap
    TAX_CAP.save(deps.storage, denom.as_bytes(), &cap)?;

    // return nothing new
    Ok(HandleResponse::default())
}

pub fn handle_update_tax_rate(
    deps: DepsMut,
    info: MessageInfo,
    rate: Decimal,
) -> Result<HandleResponse, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    let sender_addr = deps.api.canonical_address(&info.sender)?;

    // check authorized, TODO: min and max tax_rate
    if contract_info.admin.ne(&sender_addr) {
        return Err(ContractError::Unauthorized {});
    }

    // update tax cap
    let rate = rate.clamp(contract_info.min_rate, contract_info.max_rate);
    TAX_RATE.save(deps.storage, &rate)?;

    // return nothing new
    Ok(HandleResponse::default())
}

pub fn handle_update_admin(
    deps: DepsMut,
    info: MessageInfo,
    admin: HumanAddr,
) -> Result<HandleResponse, ContractError> {
    let mut contract_info = CONTRACT_INFO.load(deps.storage)?;
    let sender_addr = deps.api.canonical_address(&info.sender)?;

    // check authorized
    if contract_info.admin.ne(&sender_addr) {
        return Err(ContractError::Unauthorized {});
    }

    // update new admin
    contract_info.admin = deps.api.canonical_address(&admin)?;
    CONTRACT_INFO.save(deps.storage, &contract_info)?;

    // return nothing new
    Ok(HandleResponse::default())
}

pub fn handle_update_exchange_rate(
    deps: DepsMut,
    info: MessageInfo,
    denom: String,
    exchange_rate: Decimal,
) -> Result<HandleResponse, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    let sender_addr = deps.api.canonical_address(&info.sender)?;

    // check authorized
    if contract_info.admin.ne(&sender_addr) {
        return Err(ContractError::Unauthorized {});
    }

    EXCHANGE_RATES.save(deps.storage, denom.as_bytes(), &exchange_rate)?;

    Ok(HandleResponse::default())
}

pub fn handle_delete_exchange_rate(
    deps: DepsMut,
    info: MessageInfo,
    denom: String,
) -> Result<HandleResponse, ContractError> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    let sender_addr = deps.api.canonical_address(&info.sender)?;

    // check authorized
    if contract_info.admin.ne(&sender_addr) {
        return Err(ContractError::Unauthorized {});
    }

    EXCHANGE_RATES.remove(deps.storage, denom.as_bytes());

    Ok(HandleResponse::default())
}

pub fn query(deps: Deps, env: Env, msg: OracleQuery) -> StdResult<Binary> {
    match msg {
        OracleQuery::Treasury(query_data) => match query_data {
            OracleTreasuryQuery::TaxRate {} => to_binary(&query_tax_rate(deps)?),
            OracleTreasuryQuery::TaxCap { denom } => to_binary(&query_tax_cap(deps, denom)?),
        },
        OracleQuery::Exchange(query_data) => match query_data {
            OracleExchangeQuery::ExchangeRate {
                base_denom,
                quote_denom,
            } => to_binary(&query_exchange_rate(deps, base_denom, quote_denom)?),
            OracleExchangeQuery::ExchangeRates {
                base_denom,
                quote_denoms,
            } => to_binary(&query_exchange_rates(deps, base_denom, quote_denoms)?),
        },
        OracleQuery::Contract(query_data) => match query_data {
            OracleContractQuery::ContractInfo {} => to_binary(&query_contract_info(deps)?),
            OracleContractQuery::RewardPool { denom } => {
                to_binary(&query_contract_balance(deps, env, denom)?)
            }
        },
    }
}

pub fn query_tax_rate(deps: Deps) -> StdResult<TaxRateResponse> {
    let rate = TAX_RATE.load(deps.storage)?;
    Ok(TaxRateResponse { rate })
}

pub fn query_tax_cap(deps: Deps, denom: String) -> StdResult<TaxCapResponse> {
    let cap = TAX_CAP.load(deps.storage, denom.as_bytes())?;
    Ok(TaxCapResponse { cap })
}

pub fn query_exchange_rate(
    deps: Deps,
    base_denom: String,
    quote_denom: String,
) -> StdResult<ExchangeRateResponse> {
    let base_rate = get_orai_exchange_rate(deps, &base_denom)?;
    let quote_rate = get_orai_exchange_rate(deps, &quote_denom)?;

    // quote = ask, offer = base
    let exchange_rate = Decimal::from_ratio(
        quote_rate.mul(DECIMAL_FRACTION),
        base_rate.mul(DECIMAL_FRACTION),
    );

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

    for quote_denom in quote_denoms {
        let quote_rate = get_orai_exchange_rate(deps, &quote_denom)?;

        let exchange_rate = Decimal::from_ratio(
            quote_rate.mul(DECIMAL_FRACTION),
            base_rate.mul(DECIMAL_FRACTION),
        );

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
        admin: deps.api.human_address(&info.admin)?,
        creator: deps.api.human_address(&info.creator)?,
        min_rate: info.min_rate,
        max_rate: info.max_rate,
    })
}

pub fn query_contract_balance(deps: Deps, env: Env, denom: String) -> StdResult<Coin> {
    deps.querier.query_balance(env.contract.address, &denom)
}

fn get_orai_exchange_rate(deps: Deps, denom: &str) -> StdResult<Decimal> {
    if denom == ORAI_DENOM {
        return Ok(Decimal::one());
    }

    EXCHANGE_RATES.load(deps.storage, denom.as_bytes())
}
