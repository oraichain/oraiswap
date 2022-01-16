use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use cosmwasm_std::{
    to_binary, Api, CanonicalAddr, Coin, CosmosMsg, Decimal, HumanAddr, QuerierWrapper, StdResult,
    Uint128, WasmMsg, WasmQuery,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    /// name of the NFT contract, can use default
    pub name: Option<String>,
    pub version: Option<String>,
    pub creator: HumanAddr,
    pub admin: Option<HumanAddr>,

    pub min_rate: Option<Decimal>,
    pub max_rate: Option<Decimal>,
    // pub min_stability_spread: Option<Decimal>,
    // pub base_pool: Option<Uint128>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OracleMarketMsg {
    Swap {
        offer_coin: Coin,
        ask_denom: String,
    },
    SwapSend {
        to_address: HumanAddr,
        offer_coin: Coin,
        ask_denom: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OracleContractMsg {
    UpdateAdmin { admin: HumanAddr },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OracleExchangeMsg {
    UpdateExchangeRate {
        denom: String,
        exchange_rate: Decimal,
    },
    DeleteExchangeRate {
        denom: String,
    },
    UpdateTobinTax {
        denom: String,
        tobin_tax: Decimal,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OracleTreasuryMsg {
    UpdateTaxCap { denom: String, cap: Uint128 },
    // RateMax: 1%
    UpdateTaxRate { rate: Decimal },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OracleMsg {
    Market(OracleMarketMsg),
    Contract(OracleContractMsg),
    Exchange(OracleExchangeMsg),
    Treasury(OracleTreasuryMsg),
}

/// OracleQuery is defines available query datas
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OracleQuery {
    Market(OracleMarketQuery),
    Treasury(OracleTreasuryQuery),
    Exchange(OracleExchangeQuery),
    Contract(OracleContractQuery),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OracleMarketQuery {
    Swap { offer_coin: Coin, ask_denom: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OracleTreasuryQuery {
    TaxRate {},
    TaxCap { denom: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OracleExchangeQuery {
    ExchangeRate {
        base_denom: String,
        quote_denom: String,
    },
    ExchangeRates {
        base_denom: String,
        quote_denoms: Vec<String>,
    },
    TobinTax {
        denom: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OracleContractQuery {
    ContractInfo {},
    RewardPool { denom: String },
}

/// SwapResponse is data format returned from SwapRequest::Simulate query
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SwapResponse {
    pub receive: Coin,
}

/// TaxRateResponse is data format returned from TreasuryRequest::TaxRate query
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TaxRateResponse {
    pub rate: Decimal,
}

/// TaxCapResponse is data format returned from TreasuryRequest::TaxCap query
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TaxCapResponse {
    pub cap: Uint128,
}

/// ExchangeRateItem is data format returned from OracleRequest::ExchangeRates query
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ExchangeRateItem {
    pub quote_denom: String,
    pub exchange_rate: Decimal,
}

/// ExchangeRatesResponse is data format returned from OracleRequest::ExchangeRates query
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ExchangeRatesResponse {
    pub base_denom: String,
    pub items: Vec<ExchangeRateItem>,
}

/// ExchangeRateResponse is data format returned from OracleRequest::ExchangeRate query
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ExchangeRateResponse {
    pub base_denom: String,
    pub item: ExchangeRateItem,
}

/// ContractInfo is data format stored
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ContractInfo {
    pub name: String,
    pub version: String,
    pub creator: CanonicalAddr,
    // admin can update the parameter, may be multisig
    pub admin: CanonicalAddr,
    // constraint
    pub min_rate: Decimal,
    pub max_rate: Decimal,
    // pub min_stability_spread: Decimal,
    // pub base_pool: Uint128,
}

/// ContractInfoResponse is data format returned from WasmRequest::ContractInfo query
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ContractInfoResponse {
    pub name: String,
    pub version: String,
    pub creator: HumanAddr,
    // admin can update the parameter, may be multisig
    pub admin: HumanAddr,
    pub min_rate: Decimal,
    pub max_rate: Decimal,
    // pub min_stability_spread: Decimal,
    // pub base_pool: Uint128,
}

// create_swap_msg returns wrapped swap msg
pub fn create_swap_msg(oracle_addr: HumanAddr, offer_coin: Coin, ask_denom: String) -> CosmosMsg {
    CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: oracle_addr,
        msg: to_binary(&OracleMsg::Market(OracleMarketMsg::Swap {
            offer_coin,
            ask_denom,
        }))
        .unwrap(),
        send: vec![],
    })
}

// create_swap_send_msg returns wrapped swap send msg
pub fn create_swap_send_msg(
    oracle_addr: HumanAddr,
    to_address: HumanAddr,
    offer_coin: Coin,
    ask_denom: String,
) -> CosmosMsg {
    CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: oracle_addr,
        msg: to_binary(&OracleMsg::Market(OracleMarketMsg::SwapSend {
            to_address,
            offer_coin,
            ask_denom,
        }))
        .unwrap(),
        send: vec![],
    })
}

/// OracleContract is a wrapper around HumanAddr that provides a lot of helpers
/// for working with this.
///
/// If you wish to persist this, convert to Cw721CanonicalContract via .canonical()
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OracleContract(pub HumanAddr);

impl OracleContract {
    pub fn addr(&self) -> HumanAddr {
        self.0.clone()
    }

    /// Convert this address to a form fit for storage
    pub fn canonical<A: Api>(&self, api: &A) -> StdResult<OracleCanonicalContract> {
        let canon = api.canonical_address(&self.0)?;
        Ok(OracleCanonicalContract(canon))
    }

    pub fn call(&self, msg: OracleMsg) -> StdResult<CosmosMsg> {
        let msg = to_binary(&msg)?;
        Ok(WasmMsg::Execute {
            contract_addr: self.addr(),
            msg,
            send: vec![],
        }
        .into())
    }

    pub fn query<T: DeserializeOwned>(
        &self,
        querier: &QuerierWrapper,
        req: OracleQuery,
    ) -> StdResult<T> {
        let query = WasmQuery::Smart {
            contract_addr: self.addr(),
            msg: to_binary(&req)?,
        }
        .into();
        querier.query(&query)
    }

    /*** queries ***/

    pub fn query_swap<T: Into<String>>(
        &self,
        querier: &QuerierWrapper,
        offer_coin: Coin,
        ask_denom: T,
    ) -> StdResult<SwapResponse> {
        let request = OracleQuery::Market(OracleMarketQuery::Swap {
            offer_coin,
            ask_denom: ask_denom.into(),
        });

        self.query(querier, request)
    }

    pub fn query_tax_cap<T: Into<String>>(
        &self,
        querier: &QuerierWrapper,
        denom: T,
    ) -> StdResult<TaxCapResponse> {
        let request = OracleQuery::Treasury(OracleTreasuryQuery::TaxCap {
            denom: denom.into(),
        });

        self.query(querier, request)
    }

    pub fn query_tax_rate(&self, querier: &QuerierWrapper) -> StdResult<TaxRateResponse> {
        let request = OracleQuery::Treasury(OracleTreasuryQuery::TaxRate {});

        self.query(querier, request)
    }

    pub fn query_exchange_rates<T: Into<String>>(
        &self,
        querier: &QuerierWrapper,
        base_denom: T,
        quote_denoms: Vec<T>,
    ) -> StdResult<ExchangeRatesResponse> {
        let request = OracleQuery::Exchange(OracleExchangeQuery::ExchangeRates {
            base_denom: base_denom.into(),
            quote_denoms: quote_denoms.into_iter().map(|x| x.into()).collect(),
        });

        self.query(querier, request)
    }

    pub fn query_contract_info<T: Into<String>>(
        &self,
        querier: &QuerierWrapper,
    ) -> StdResult<ContractInfoResponse> {
        let request = OracleQuery::Contract(OracleContractQuery::ContractInfo {});

        self.query(querier, request)
    }
}

/// This is a respresentation of OracleContract for storage.
/// Don't use it directly, just translate to the OracleContract when needed.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OracleCanonicalContract(pub CanonicalAddr);

impl OracleCanonicalContract {
    /// Convert this address to a form fit for usage in messages and queries
    pub fn human<A: Api>(&self, api: &A) -> StdResult<OracleContract> {
        let human = api.human_address(&self.0)?;
        Ok(OracleContract(human))
    }
}
