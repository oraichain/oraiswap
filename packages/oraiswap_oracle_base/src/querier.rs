use cosmwasm_std::{Coin, QuerierWrapper, StdResult};

use crate::query::{
    ContractInfoResponse, ExchangeRatesResponse, OraiQuery, OraiQueryWrapper, SwapResponse,
    TaxCapResponse, TaxRateResponse,
};
use crate::route::OraiRoute;

/// This is a helper wrapper to easily use our custom queries
pub struct OraiQuerier<'a> {
    querier: &'a QuerierWrapper<'a>,
}

/// Instead of using custom query to query blockchain, we wrap on a smart contract
/// then call the method to get data
impl<'a> OraiQuerier<'a> {
    pub fn new(querier: &'a QuerierWrapper<'a>) -> Self {
        OraiQuerier { querier }
    }

    pub fn query_swap<T: Into<String>>(
        &self,
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

        self.querier.custom_query(&request)
    }

    pub fn query_tax_cap<T: Into<String>>(&self, denom: T) -> StdResult<TaxCapResponse> {
        let request = OraiQueryWrapper {
            route: OraiRoute::Treasury,
            query_data: OraiQuery::TaxCap {
                denom: denom.into(),
            },
        }
        .into();

        self.querier.custom_query(&request)
    }

    pub fn query_tax_rate(&self) -> StdResult<TaxRateResponse> {
        let request = OraiQueryWrapper {
            route: OraiRoute::Treasury,
            query_data: OraiQuery::TaxRate {},
        }
        .into();

        self.querier.custom_query(&request)
    }

    pub fn query_exchange_rates<T: Into<String>>(
        &self,
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

        self.querier.custom_query(&request)
    }

    pub fn query_contract_info<T: Into<String>>(
        &self,
        contract_address: T,
    ) -> StdResult<ContractInfoResponse> {
        let request = OraiQueryWrapper {
            route: OraiRoute::Wasm,
            query_data: OraiQuery::ContractInfo {
                contract_address: contract_address.into(),
            },
        }
        .into();

        self.querier.custom_query(&request)
    }
}
