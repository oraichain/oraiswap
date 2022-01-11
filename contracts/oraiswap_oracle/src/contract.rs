use cosmwasm_std::{
    to_binary, Binary, Coin, Deps, DepsMut, Env, HandleResponse, HumanAddr, InitResponse,
    MessageInfo, StdResult,
};

use oracle_base::{
    ContractInfoResponse, ExchangeRateItem, ExchangeRatesResponse, OracleContractQuery,
    OracleExchangeQuery, OracleMarketMsg, OracleMarketQuery, OracleMsg, OracleQuery,
    OracleTreasuryQuery, SwapResponse, TaxCapResponse, TaxRateResponse,
};

use oraiswap::oracle::InitMsg;

// use crate::msg::{HandleMsg, InitMsg};
use crate::state::{CONTRACT_INFO, EXCHANGE_RATES, TAX_CAP, TAX_RATE};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:oraiswap_oracle";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn init(
    deps: DepsMut,
    _env: Env,
    msg_info: MessageInfo,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let info = ContractInfoResponse {
        name: msg.name.unwrap_or(CONTRACT_NAME.to_string()),
        version: msg.version.unwrap_or(CONTRACT_VERSION.to_string()),
        creator: msg_info.sender.clone(),
    };
    CONTRACT_INFO.save(deps.storage, &info)?;
    Ok(InitResponse::default())
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: OracleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        OracleMsg::Market(handle_data) => match handle_data {
            OracleMarketMsg::Swap {
                offer_coin,
                ask_denom,
            } => handle_swap(deps, info, env.contract.address, offer_coin, ask_denom),
            OracleMarketMsg::SwapSend {
                to_address,
                offer_coin,
                ask_denom,
            } => handle_swap(deps, info, to_address, offer_coin, ask_denom),
        },
    }
}

// Only owner can execute it
pub fn handle_swap(
    deps: DepsMut,
    info: MessageInfo,
    to_address: HumanAddr,
    offer_coin: Coin,
    ask_denom: String,
) -> StdResult<HandleResponse> {
    // TODO: implemented from here https://github.com/terra-money/core/blob/main/x/market/keeper/msg_server.go
    Ok(HandleResponse::default())
}

pub fn query(deps: Deps, env: Env, msg: OracleQuery) -> StdResult<Binary> {
    match msg {
        OracleQuery::Treasury(query_data) => match query_data {
            OracleTreasuryQuery::TaxRate {} => to_binary(&query_tax_rate(deps)?),
            OracleTreasuryQuery::TaxCap { denom } => to_binary(&query_tax_cap(deps, denom)?),
        },
        OracleQuery::Market(query_data) => match query_data {
            OracleMarketQuery::Swap {
                offer_coin,
                ask_denom,
            } => to_binary(&query_swap(deps, offer_coin, ask_denom)?),
        },
        OracleQuery::Exchange(query_data) => match query_data {
            OracleExchangeQuery::ExchangeRates {
                base_denom,
                quote_denoms,
            } => to_binary(&query_exchange_rates(deps, base_denom, quote_denoms)?),
        },
        OracleQuery::Contract(query_data) => match query_data {
            OracleContractQuery::ContractInfo {} => to_binary(&query_contract_info(deps)?),
        },
    }
}

pub fn query_tax_rate(deps: Deps) -> StdResult<TaxRateResponse> {
    // TODO : implemented here https://github.com/terra-money/core/tree/main/x/treasury/spec
    let rate = TAX_RATE.load(deps.storage)?;
    Ok(TaxRateResponse { rate })
}

pub fn query_tax_cap(deps: Deps, denom: String) -> StdResult<TaxCapResponse> {
    // TODO : implemented here https://github.com/terra-money/core/tree/main/x/treasury/spec
    let cap = TAX_CAP.load(deps.storage, denom.as_bytes())?;
    Ok(TaxCapResponse { cap })
}

pub fn query_swap(deps: Deps, offer_coin: Coin, ask_denom: String) -> StdResult<SwapResponse> {
    // TODO: implemented here https://github.com/terra-money/core/blob/main/x/market/keeper/querier.go
    // with offer_coin, ask for denom, will return receive, based on swap rate
    Ok(SwapResponse {
        receive: offer_coin.clone(),
    })
}

pub fn query_exchange_rates(
    deps: Deps,
    base_denom: String,
    quote_denoms: Vec<String>,
) -> StdResult<ExchangeRatesResponse> {
    // TODO: implemented here https://github.com/terra-money/core/tree/main/x/oracle/spec

    let mut res = ExchangeRatesResponse {
        base_denom: base_denom.clone(),
        exchange_rates: vec![],
    };

    for quote_denom in quote_denoms {
        let key = [base_denom.as_bytes(), quote_denom.as_bytes()].concat();
        let exchange_rate = EXCHANGE_RATES.load(deps.storage, &key)?;
        res.exchange_rates.push(ExchangeRateItem {
            quote_denom,
            exchange_rate,
        });
    }

    Ok(res)
}

pub fn query_contract_info(deps: Deps) -> StdResult<ContractInfoResponse> {
    CONTRACT_INFO.load(deps.storage)
}
