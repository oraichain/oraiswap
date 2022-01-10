use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::query::{
    ContractInfoResponse, ExchangeRatesResponse, OraiQuery, OraiQueryWrapper, SwapResponse,
    TaxCapResponse, TaxRateResponse,
};
use crate::route::OraiRoute;
use crate::OraiMsgWrapper;

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

    pub fn call(&self, msg: OraiMsgWrapper) -> StdResult<CosmosMsg> {
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
        req: OraiQueryWrapper,
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
        let request = OraiQueryWrapper {
            route: OraiRoute::Market,
            query_data: OraiQuery::Swap {
                offer_coin,
                ask_denom: ask_denom.into(),
            },
        }
        .into();

        self.query(querier, request)
    }

    pub fn query_tax_cap<T: Into<String>>(
        &self,
        querier: &QuerierWrapper,
        denom: T,
    ) -> StdResult<TaxCapResponse> {
        let request = OraiQueryWrapper {
            route: OraiRoute::Treasury,
            query_data: OraiQuery::TaxCap {
                denom: denom.into(),
            },
        }
        .into();

        self.query(querier, request)
    }

    pub fn query_tax_rate(&self, querier: &QuerierWrapper) -> StdResult<TaxRateResponse> {
        let request = OraiQueryWrapper {
            route: OraiRoute::Treasury,
            query_data: OraiQuery::TaxRate {},
        }
        .into();

        self.query(querier, request)
    }

    pub fn query_exchange_rates<T: Into<String>>(
        &self,
        querier: &QuerierWrapper,
        base_denom: T,
        quote_denoms: Vec<T>,
    ) -> StdResult<ExchangeRatesResponse> {
        let request = OraiQueryWrapper {
            route: OraiRoute::Oracle,
            query_data: OraiQuery::ExchangeRates {
                base_denom: base_denom.into(),
                quote_denoms: quote_denoms.into_iter().map(|x| x.into()).collect(),
            },
        }
        .into();

        self.query(querier, request)
    }

    pub fn query_contract_info<T: Into<String>>(
        &self,
        querier: &QuerierWrapper,
        contract_address: T,
    ) -> StdResult<ContractInfoResponse> {
        let request = OraiQueryWrapper {
            route: OraiRoute::Wasm,
            query_data: OraiQuery::ContractInfo {
                contract_address: contract_address.into(),
            },
        }
        .into();

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
