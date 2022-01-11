use cosmwasm_std::{
    attr, to_binary, Api, Binary, BlockInfo, Deps, DepsMut, Env, HandleResponse, HumanAddr,
    InitResponse, MessageInfo, Order, StdError, StdResult, KV,
};

use oracle_base::{
    ContractInfoResponse, OracleMsgWrapper, OracleQuery, OracleQueryWrapper, OracleRoute,
};

use crate::check_size;
use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, MintMsg, MinterResponse, QueryMsg};
use crate::state::{
    decrement_tokens, increment_tokens, num_tokens, tokens, TokenInfo, CONTRACT_INFO,
};
use cw_storage_plus::Bound;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:oraiswap_oracle";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;
const MAX_CHARS_SIZE: usize = 1024;

pub fn init(
    deps: DepsMut,
    _env: Env,
    msg_info: MessageInfo,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let creator = msg_info.sender.to_string();
    let info = ContractInfoResponse {
        name: msg.name.unwrap_or(CONTRACT_NAME.to_string()),
        version: msg.version.unwrap_or(CONTRACT_VERSION.to_string()),
        creator,
        admin: msg.admin.unwrap_or(creator),
    };
    CONTRACT_INFO.save(deps.storage, &info)?;
    Ok(InitResponse::default())
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {}
}

pub fn query(deps: Deps, env: Env, msg: OracleQueryWrapper) -> StdResult<Binary> {
    let OracleQueryWrapper { route, query_data } = msg;
    match route {
        OracleRoute::Treasury => match query_data {
            OracleQuery::TaxRate {} => {
                let res = TaxRateResponse {
                    rate: self.tax_querier.rate,
                };
            }
            OracleQuery::TaxCap { denom } => {
                let res = TaxCapResponse { cap };
            }
            OracleQuery::Swap {
                offer_coin,
                ask_denom,
            } => todo!(),
            OracleQuery::ExchangeRates {
                base_denom,
                quote_denoms,
            } => todo!(),
            OracleQuery::ContractInfo { contract_address } => todo!(),
        },
        OracleRoute::Market => match query_data {
            OracleQuery::Swap {
                offer_coin,
                ask_denom: _,
            } => {
                let res = SwapResponse {
                    receive: offer_coin.clone(),
                };
            }
            OracleQuery::TaxRate {} => todo!(),
            OracleQuery::TaxCap { denom } => todo!(),
            OracleQuery::ExchangeRates {
                base_denom,
                quote_denoms,
            } => todo!(),
            OracleQuery::ContractInfo { contract_address } => todo!(),
        },
        OracleRoute::Oracle => todo!(),
        OracleRoute::Wasm => todo!(),
    }
}
