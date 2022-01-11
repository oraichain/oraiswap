use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::query::{
    ContractInfoResponse, ExchangeRatesResponse, OracleContractQuery, OracleExchangeQuery,
    OracleMarketQuery, OracleQuery, OracleTreasuryQuery, SwapResponse, TaxCapResponse,
    TaxRateResponse,
};

use crate::OracleMsg;

use cosmwasm_std::{
    to_binary, Api, CanonicalAddr, Coin, CosmosMsg, HumanAddr, QuerierWrapper, StdResult, WasmMsg,
    WasmQuery,
};

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
