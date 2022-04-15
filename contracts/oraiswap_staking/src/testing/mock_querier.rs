use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Coin, ContractResult, Decimal, Empty, HumanAddr, OwnedDeps,
    Querier, QuerierResult, QueryRequest, SystemError, SystemResult, Uint128, WasmQuery,
};
use cosmwasm_storage::to_length_prefixed;
use oraiswap::oracle::{
    ExchangeRateResponse, OracleExchangeQuery, OracleQuery, OracleTreasuryQuery, TaxCapResponse,
    TaxRateResponse,
};
use oraiswap::{
    asset::Asset, asset::AssetInfo, asset::PairInfo, asset::ORAI_DENOM, oracle::ExchangeRateItem,
    pair::PoolResponse,
};

use serde::Deserialize;

pub struct WasmMockQuerier {
    base: MockQuerier<Empty>,
    pair_addr: HumanAddr,
    pool_assets: [Asset; 2],
    oracle_price: Decimal,
    token_balance: Uint128,
    tax: (Decimal, Uint128),
}

pub fn mock_dependencies_with_querier(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let custom_querier: WasmMockQuerier = WasmMockQuerier::new(MockQuerier::new(&[(
        &MOCK_CONTRACT_ADDR.into(),
        contract_balance,
    )]));

    OwnedDeps {
        api: MockApi::default(),
        storage: MockStorage::default(),
        querier: custom_querier,
    }
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<Empty> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };
        self.handle_query(&request)
    }
}

// use mock format so we do not have to try multiple other formats
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MockQueryMsg {
    Pair { asset_infos: [AssetInfo; 2] },
    Pool {},
    Balance { address: String },
    TokenInfo {},
}

impl WasmMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(WasmQuery::Smart { msg, contract_addr }) => match from_binary(msg) {
                // maybe querywrapper like custom query from smart contract
                Ok(OracleQuery::Treasury(query_data)) => match query_data {
                    OracleTreasuryQuery::TaxRate {} => {
                        let res = TaxRateResponse { rate: self.tax.0 };
                        SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
                    }
                    OracleTreasuryQuery::TaxCap { .. } => {
                        let res = TaxCapResponse { cap: self.tax.1 };
                        SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
                    }
                },
                // query exchange rate
                Ok(OracleQuery::Exchange(query_data)) => match query_data {
                    OracleExchangeQuery::ExchangeRate {
                        base_denom,
                        quote_denom,
                    } => {
                        let res = ExchangeRateResponse {
                            base_denom: base_denom.unwrap_or(ORAI_DENOM.to_string()),
                            item: ExchangeRateItem {
                                quote_denom,
                                exchange_rate: self.oracle_price,
                            },
                        };
                        SystemResult::Ok(ContractResult::Ok(to_binary(&res).unwrap()))
                    }
                    _ => panic!("DO NOT ENTER HERE"),
                },

                // try with MockQueryMsg
                _ => match from_binary(msg).unwrap() {
                    MockQueryMsg::Pair { asset_infos } => {
                        SystemResult::Ok(ContractResult::from(to_binary(&PairInfo {
                            asset_infos,
                            oracle_addr: "oracle0000".into(),
                            contract_addr: self.pair_addr.clone(),
                            liquidity_token: "lptoken".into(),
                            commission_rate: "1".into(),
                        })))
                    }
                    MockQueryMsg::Pool {} => {
                        SystemResult::Ok(ContractResult::from(to_binary(&PoolResponse {
                            assets: self.pool_assets.clone(),
                            total_share: Uint128::zero(),
                        })))
                    }
                    MockQueryMsg::Balance { address: _ } => {
                        SystemResult::Ok(ContractResult::from(to_binary(&cw20::BalanceResponse {
                            balance: self.token_balance,
                        })))
                    }
                    MockQueryMsg::TokenInfo {} => SystemResult::Ok(ContractResult::from(
                        to_binary(&cw20::TokenInfoResponse {
                            symbol: contract_addr.to_string(),
                            // fake 1 million token
                            total_supply: Uint128(1_000_000_000u128),
                            decimals: 6,
                            name: "Mock Token".to_string(),
                        }),
                    )),
                },
            },

            QueryRequest::Wasm(WasmQuery::Raw {
                contract_addr: _,
                key,
            }) => {
                let key: &[u8] = key.as_slice();
                let prefix_balance = to_length_prefixed(b"balance").to_vec();
                if key[..prefix_balance.len()].to_vec() == prefix_balance {
                    SystemResult::Ok(ContractResult::from(to_binary(&self.token_balance)))
                } else {
                    panic!("DO NOT ENTER HERE")
                }
            }
            _ => self.base.handle_query(request),
        }
    }
}

impl WasmMockQuerier {
    pub fn new(base: MockQuerier<Empty>) -> Self {
        WasmMockQuerier {
            base,
            pair_addr: "".into(),
            pool_assets: [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                    amount: Uint128::zero(),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: "asset".into(),
                    },
                    amount: Uint128::zero(),
                },
            ],
            oracle_price: Decimal::zero(),
            token_balance: Uint128::zero(),
            tax: (Decimal::percent(1), Uint128(1000000)),
        }
    }

    pub fn with_pair_info(&mut self, pair_addr: HumanAddr) {
        self.pair_addr = pair_addr;
    }

    pub fn with_pool_assets(&mut self, pool_assets: [Asset; 2]) {
        self.pool_assets = pool_assets;
    }

    pub fn with_token_balance(&mut self, token_balance: Uint128) {
        self.token_balance = token_balance;
    }
}
