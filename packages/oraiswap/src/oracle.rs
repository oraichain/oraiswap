use cosmwasm_schema::{cw_serde, QueryResponses};
use serde::de::DeserializeOwned;

use cosmwasm_std::{
    to_binary, Addr, Api, CanonicalAddr, Coin, CosmosMsg, Decimal, QuerierWrapper, StdResult,
    Uint128, WasmMsg, WasmQuery,
};

#[cw_serde]
pub struct InstantiateMsg {
    /// name of the NFT contract, can use default
    pub name: Option<String>,
    pub version: Option<String>,
    pub admin: Option<Addr>,
    pub min_rate: Option<Decimal>,
    pub max_rate: Option<Decimal>,
}

#[cw_serde]
pub enum OracleContractMsg {
    UpdateAdmin { admin: Addr },
}

#[cw_serde]
pub enum OracleExchangeMsg {
    UpdateExchangeRate {
        denom: String,
        exchange_rate: Decimal,
    },
    DeleteExchangeRate {
        denom: String,
    },
}

#[cw_serde]
pub enum OracleTreasuryMsg {
    UpdateTaxCap { denom: String, cap: Uint128 },
    // RateMax: 1%
    UpdateTaxRate { rate: Decimal },
}

#[cw_serde]
pub enum OracleMsg {
    Contract(OracleContractMsg),
    Exchange(OracleExchangeMsg),
    Treasury(OracleTreasuryMsg),
}

/// OracleQuery is defines available query datas
#[cw_serde]
pub enum OracleQuery {
    Treasury(OracleTreasuryQuery),
    Exchange(OracleExchangeQuery),
    Contract(OracleContractQuery),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum OracleTreasuryQuery {
    #[returns(TaxRateResponse)]
    TaxRate {},
    #[returns(TaxCapResponse)]
    TaxCap { denom: String },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum OracleExchangeQuery {
    #[returns(ExchangeRateResponse)]
    ExchangeRate {
        base_denom: Option<String>,
        quote_denom: String,
    },
    #[returns(ExchangeRatesResponse)]
    ExchangeRates {
        base_denom: Option<String>,
        quote_denoms: Vec<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum OracleContractQuery {
    #[returns(ContractInfoResponse)]
    ContractInfo {},
    #[returns(Coin)]
    RewardPool { denom: String },
}

/// TaxRateResponse is data format returned from TreasuryRequest::TaxRate query
#[cw_serde]
pub struct TaxRateResponse {
    pub rate: Decimal,
}

/// TaxCapResponse is data format returned from TreasuryRequest::TaxCap query
#[cw_serde]
pub struct TaxCapResponse {
    pub cap: Uint128,
}

/// ExchangeRateItem is data format returned from OracleRequest::ExchangeRates query
#[cw_serde]
pub struct ExchangeRateItem {
    pub quote_denom: String,
    pub exchange_rate: Decimal,
}

/// ExchangeRatesResponse is data format returned from OracleRequest::ExchangeRates query
#[cw_serde]
pub struct ExchangeRatesResponse {
    pub base_denom: String,
    pub items: Vec<ExchangeRateItem>,
}

/// ExchangeRateResponse is data format returned from OracleRequest::ExchangeRate query
#[cw_serde]
pub struct ExchangeRateResponse {
    pub base_denom: String,
    pub item: ExchangeRateItem,
}

/// ContractInfo is data format stored
#[cw_serde]
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
#[cw_serde]
pub struct ContractInfoResponse {
    pub name: String,
    pub version: String,
    pub creator: Addr,
    // admin can update the parameter, may be multisig
    pub admin: Addr,
    pub min_rate: Decimal,
    pub max_rate: Decimal,
    // pub min_stability_spread: Decimal,
    // pub base_pool: Uint128,
}

/// We currently take no arguments for migrations
#[cw_serde]
pub struct MigrateMsg {}

/// OracleContract is a wrapper around Addr that provides a lot of helpers
/// for working with this.
///
/// If you wish to persist this, convert to Cw721CanonicalContract via .canonical()
#[cw_serde]
pub struct OracleContract(pub Addr);

impl OracleContract {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }

    /// Convert this address to a form fit for storage
    pub fn canonical<A: Api>(&self, api: &A) -> StdResult<OracleCanonicalContract> {
        let canon = api.addr_canonicalize(&self.0.as_str())?;
        Ok(OracleCanonicalContract(canon))
    }

    pub fn call(&self, msg: OracleMsg) -> StdResult<CosmosMsg> {
        let msg = to_binary(&msg)?;
        Ok(WasmMsg::Execute {
            contract_addr: self.to_string(),
            msg,
            funds: vec![],
        }
        .into())
    }

    pub fn query<T: DeserializeOwned>(
        &self,
        querier: &QuerierWrapper,
        req: OracleQuery,
    ) -> StdResult<T> {
        let query = WasmQuery::Smart {
            contract_addr: self.to_string(),
            msg: to_binary(&req)?,
        }
        .into();
        querier.query(&query)
    }

    /*** queries ***/

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

    // this is for CEX
    pub fn query_exchange_rate<T: Into<String>>(
        &self,
        querier: &QuerierWrapper,
        base_denom: T,
        quote_denom: T,
    ) -> StdResult<ExchangeRateResponse> {
        let request = OracleQuery::Exchange(OracleExchangeQuery::ExchangeRate {
            base_denom: Some(base_denom.into()),
            quote_denom: quote_denom.into(),
        });

        self.query(querier, request)
    }

    pub fn query_exchange_rates<T: Into<String>>(
        &self,
        querier: &QuerierWrapper,
        base_denom: T,
        quote_denoms: Vec<T>,
    ) -> StdResult<ExchangeRatesResponse> {
        let request = OracleQuery::Exchange(OracleExchangeQuery::ExchangeRates {
            base_denom: Some(base_denom.into()),
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
#[cw_serde]
pub struct OracleCanonicalContract(pub CanonicalAddr);

impl OracleCanonicalContract {
    /// Convert this address to a form fit for usage in messages and queries
    pub fn human<A: Api>(&self, api: &A) -> StdResult<OracleContract> {
        let human = api.addr_humanize(&self.0)?;
        Ok(OracleContract(human))
    }
}
