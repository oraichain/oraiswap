use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Coin, ContractResult, Decimal, Empty, OwnedDeps, Querier,
    QuerierResult, QueryRequest, SystemError, SystemResult, Uint128, WasmQuery,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg};
use oracle_base::{
    OracleMarketQuery, OracleQuery, OracleTreasuryQuery, SwapResponse, TaxCapResponse,
    TaxRateResponse,
};
use oraiswap::asset::{AssetInfo, PairInfo};
use oraiswap::factory::{ConfigResponse, QueryMsg as FactoryQueryMsg};
use oraiswap::pair::{QueryMsg as PairQueryMsg, SimulationResponse};

/// mock_dependencies is a drop-in replacement for cosmwasm_std::testing::mock_dependencies
/// this uses our CustomQuerier.
pub fn mock_dependencies(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let custom_querier: WasmMockQuerier = WasmMockQuerier::new(MockQuerier::new(&[(
        &MOCK_CONTRACT_ADDR.into(),
        contract_balance,
    )]));

    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: custom_querier,
    }
}

pub struct WasmMockQuerier {
    base: MockQuerier<Empty>,
    token_querier: TokenQuerier,
    tax_querier: TaxQuerier,
    oraiswap_factory_querier: OraiswapFactoryQuerier,
}

#[derive(Clone, Default)]
pub struct TokenQuerier {
    // this lets us iterate over all pairs that match the first string
    balances: HashMap<String, HashMap<String, Uint128>>,
}

impl TokenQuerier {
    pub fn new(balances: &[(&String, &[(&String, &Uint128)])]) -> Self {
        TokenQuerier {
            balances: balances_to_map(balances),
        }
    }
}

pub(crate) fn balances_to_map(
    balances: &[(&String, &[(&String, &Uint128)])],
) -> HashMap<String, HashMap<String, Uint128>> {
    let mut balances_map: HashMap<String, HashMap<String, Uint128>> = HashMap::new();
    for (contract_addr, balances) in balances.iter() {
        let mut contract_balances_map: HashMap<String, Uint128> = HashMap::new();
        for (addr, balance) in balances.iter() {
            contract_balances_map.insert(addr.to_string(), **balance);
        }

        balances_map.insert(contract_addr.to_string(), contract_balances_map);
    }
    balances_map
}

#[derive(Clone, Default)]
pub struct TaxQuerier {
    rate: Decimal,
    // this lets us iterate over all pairs that match the first string
    caps: HashMap<String, Uint128>,
}

impl TaxQuerier {
    pub fn new(rate: Decimal, caps: &[(&String, &Uint128)]) -> Self {
        TaxQuerier {
            rate,
            caps: caps_to_map(caps),
        }
    }
}

pub(crate) fn caps_to_map(caps: &[(&String, &Uint128)]) -> HashMap<String, Uint128> {
    let mut owner_map: HashMap<String, Uint128> = HashMap::new();
    for (denom, cap) in caps.iter() {
        owner_map.insert(denom.to_string(), **cap);
    }
    owner_map
}

#[derive(Clone, Default)]
pub struct OraiswapFactoryQuerier {
    pairs: HashMap<String, String>,
}

impl OraiswapFactoryQuerier {
    pub fn new(pairs: &[(&String, &String)]) -> Self {
        OraiswapFactoryQuerier {
            pairs: pairs_to_map(pairs),
        }
    }
}

pub(crate) fn pairs_to_map(pairs: &[(&String, &String)]) -> HashMap<String, String> {
    let mut pairs_map: HashMap<String, String> = HashMap::new();
    for (key, pair) in pairs.iter() {
        pairs_map.insert(key.to_string(), pair.to_string());
    }
    pairs_map
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MockQueryMsg {
    Price {},
}

impl WasmMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => match from_binary(msg) {
                Ok(OracleQuery::Treasury(query_data)) => match query_data {
                    OracleTreasuryQuery::TaxRate {} => {
                        let res = TaxRateResponse {
                            rate: self.tax_querier.rate,
                        };
                        SystemResult::Ok(ContractResult::from(to_binary(&res)))
                    }
                    OracleTreasuryQuery::TaxCap { denom } => {
                        let cap = self
                            .tax_querier
                            .caps
                            .get(&denom)
                            .copied()
                            .unwrap_or_default();
                        let res = TaxCapResponse { cap };
                        SystemResult::Ok(ContractResult::from(to_binary(&res)))
                    }
                },
                Ok(OracleQuery::Market(query_data)) => match query_data {
                    OracleMarketQuery::Swap {
                        offer_coin,
                        ask_denom: _,
                    } => {
                        let res = SwapResponse {
                            receive: offer_coin.clone(),
                        };
                        SystemResult::Ok(ContractResult::from(to_binary(&res)))
                    }
                },

                // process other cases
                _ => match from_binary(msg) {
                    Ok(PairQueryMsg::Simulation { offer_asset }) => {
                        SystemResult::Ok(ContractResult::from(to_binary(&SimulationResponse {
                            return_amount: offer_asset.amount,
                            commission_amount: Uint128::zero(),
                            spread_amount: Uint128::zero(),
                        })))
                    }

                    // process FactoryQueryMsg cases
                    _ => match from_binary(msg) {
                        // fake config response when call factory contract
                        // then by calling oracle contract it will go to OracleQueryWrapper
                        Ok(FactoryQueryMsg::Config {}) => SystemResult::Ok(ContractResult::Ok(
                            to_binary(&ConfigResponse {
                                owner: "owner0000".into(),
                                oracle_addr: "oracle0000".into(),
                                pair_code_id: 321u64,
                                token_code_id: 123u64,
                            })
                            .unwrap(),
                        )),

                        Ok(FactoryQueryMsg::Pair { asset_infos }) => {
                            let key =
                                asset_infos[0].to_string() + asset_infos[1].to_string().as_str();
                            match self.oraiswap_factory_querier.pairs.get(&key) {
                                Some(v) => {
                                    SystemResult::Ok(ContractResult::from(to_binary(&PairInfo {
                                        creator: "creator0000".into(),
                                        oracle_addr: "oracle0000".into(),
                                        contract_addr: v.to_owned().into(),
                                        liquidity_token: "liquidity".into(),
                                        asset_infos: [
                                            AssetInfo::NativeToken {
                                                denom: "uusd".to_string(),
                                            },
                                            AssetInfo::NativeToken {
                                                denom: "uusd".to_string(),
                                            },
                                        ],
                                    })))
                                }
                                None => SystemResult::Err(SystemError::InvalidRequest {
                                    error: "No pair info exists".to_string(),
                                    request: msg.as_slice().into(),
                                }),
                            }
                        }

                        _ => match from_binary(msg).unwrap() {
                            Cw20QueryMsg::Balance { address } => {
                                let balances: &HashMap<String, Uint128> =
                                    match self.token_querier.balances.get(contract_addr.as_str()) {
                                        Some(balances) => balances,
                                        None => {
                                            return SystemResult::Err(SystemError::InvalidRequest {
                                                error: format!(
                                                    "No balance info exists for the contract {}",
                                                    contract_addr
                                                ),
                                                request: msg.as_slice().into(),
                                            })
                                        }
                                    };

                                let balance = match balances.get(address.as_str()) {
                                    Some(v) => *v,
                                    None => {
                                        return SystemResult::Ok(ContractResult::Ok(
                                            to_binary(&Cw20BalanceResponse {
                                                balance: Uint128::zero(),
                                            })
                                            .unwrap(),
                                        ));
                                    }
                                };

                                SystemResult::Ok(ContractResult::Ok(
                                    to_binary(&Cw20BalanceResponse { balance }).unwrap(),
                                ))
                            }
                            _ => panic!("DO NOT ENTER HERE"),
                        },
                    },
                },
            },
            _ => self.base.handle_query(request),
        }
    }
}

impl WasmMockQuerier {
    pub fn new(base: MockQuerier<Empty>) -> Self {
        WasmMockQuerier {
            base,
            token_querier: TokenQuerier::default(),
            tax_querier: TaxQuerier::default(),
            oraiswap_factory_querier: OraiswapFactoryQuerier::default(),
        }
    }

    pub fn with_balance(&mut self, balances: &[(String, &[Coin])]) {
        for (addr, balance) in balances {
            self.base.update_balance(addr.as_str(), balance.to_vec());
        }
    }

    pub fn with_token_balances(&mut self, balances: &[(&String, &[(&String, &Uint128)])]) {
        self.token_querier = TokenQuerier::new(balances);
    }

    pub fn with_tax(&mut self, rate: Decimal, caps: &[(&String, &Uint128)]) {
        self.tax_querier = TaxQuerier::new(rate, caps);
    }

    pub fn with_oraiswap_pairs(&mut self, pairs: &[(&String, &String)]) {
        self.oraiswap_factory_querier = OraiswapFactoryQuerier::new(pairs);
    }
}
