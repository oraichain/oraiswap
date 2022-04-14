use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_slice, to_binary, Api, Coin, ContractResult, Empty, OwnedDeps, Querier, QuerierResult,
    QueryRequest, SystemError, SystemResult, WasmQuery,
};
use cosmwasm_storage::to_length_prefixed;
use oraiswap::asset::{AssetInfoRaw, PairInfo, PairInfoRaw, ORAI_DENOM};
use std::collections::HashMap;

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
        api: MockApi::default(),
        storage: MockStorage::default(),
        querier: custom_querier,
    }
}

pub struct WasmMockQuerier {
    base: MockQuerier<Empty>,
    oraiswap_pair_querier: OraiswapPairQuerier,
}

#[derive(Clone, Default)]
pub struct OraiswapPairQuerier {
    pairs: HashMap<String, PairInfo>,
}

impl OraiswapPairQuerier {
    pub fn new(pairs: &[(&String, &PairInfo)]) -> Self {
        OraiswapPairQuerier {
            pairs: pairs_to_map(pairs),
        }
    }
}

pub(crate) fn pairs_to_map(pairs: &[(&String, &PairInfo)]) -> HashMap<String, PairInfo> {
    let mut pairs_map: HashMap<String, PairInfo> = HashMap::new();
    for (key, pair) in pairs.iter() {
        pairs_map.insert(key.to_string(), (*pair).clone());
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

impl WasmMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(WasmQuery::Raw { contract_addr, key }) => {
                let key: &[u8] = key.as_slice();

                let prefix_pair_info = to_length_prefixed(b"pair_info");
                if key.eq(&prefix_pair_info) {
                    let pair_info: PairInfo =
                        match self.oraiswap_pair_querier.pairs.get(contract_addr.as_str()) {
                            Some(v) => v.clone(),
                            None => {
                                return SystemResult::Err(SystemError::InvalidRequest {
                                    error: format!("PairInfo is not found for {}", contract_addr),
                                    request: key.into(),
                                })
                            }
                        };

                    let api: MockApi = MockApi::default();
                    let pair_info_raw = PairInfoRaw {
                        oracle_addr: api.canonical_address(&pair_info.oracle_addr).unwrap(),
                        contract_addr: api.canonical_address(&pair_info.contract_addr).unwrap(),
                        liquidity_token: api.canonical_address(&pair_info.liquidity_token).unwrap(),
                        asset_infos: [
                            AssetInfoRaw::NativeToken {
                                denom: ORAI_DENOM.to_string(),
                            },
                            AssetInfoRaw::NativeToken {
                                denom: ORAI_DENOM.to_string(),
                            },
                        ],
                        commission_rate: pair_info.commission_rate,
                    };
                    SystemResult::Ok(ContractResult::from(to_binary(&pair_info_raw)))
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
            oraiswap_pair_querier: OraiswapPairQuerier::default(),
        }
    }

    // configure the oraiswap pair, with contract_address => pair info
    pub fn with_oraiswap_pairs(&mut self, pairs: &[(&String, &PairInfo)]) {
        self.oraiswap_pair_querier = OraiswapPairQuerier::new(pairs);
    }
}
