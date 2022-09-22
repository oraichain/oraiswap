use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Coin, ContractResult, Empty, HumanAddr, OwnedDeps, Querier,
    QuerierResult, QueryRequest, SystemError, SystemResult, WasmQuery,
};
use oraiswap::asset::{AssetInfo, PairInfo};

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
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => match from_binary(msg) {
                Ok(oraiswap::pair::QueryMsg::Pair {}) => {
                    let pair_info = PairInfo {
                        oracle_addr: HumanAddr("oracle_addr".to_string()),
                        contract_addr: contract_addr.clone(),
                        liquidity_token: HumanAddr("liquidity_token".to_string()),
                        asset_infos: [
                            AssetInfo::NativeToken {
                                denom: oraiswap::asset::ORAI_DENOM.to_string(),
                            },
                            AssetInfo::NativeToken {
                                denom: oraiswap::asset::ORAI_DENOM.to_string(),
                            },
                        ],
                        commission_rate: "0.3".to_string(),
                    };
                    SystemResult::Ok(ContractResult::from(to_binary(&pair_info)))
                }
                // process other cases
                _ => panic!("DO NOT ENTER HERE"),
            },
            _ => self.base.handle_query(request),
        }
    }
}

impl WasmMockQuerier {
    pub fn new(base: MockQuerier<Empty>) -> Self {
        WasmMockQuerier { base }
    }
}
