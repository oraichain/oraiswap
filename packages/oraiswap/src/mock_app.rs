use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};
use cosmwasm_std::{
    AllBalanceResponse, BalanceResponse, BankQuery, Coin, Decimal, HumanAddr, QuerierWrapper,
    QueryRequest, StdResult, Uint128,
};
use std::collections::HashMap;

use crate::asset::{AssetInfo, PairInfo};

use crate::pair::DEFAULT_COMMISSION_RATE;
use cw_multi_test::{next_block, App, Contract, SimpleBank};

pub const ATOM_DENOM: &str = "ibc/1777D03C5392415FE659F0E8ECB2CE553C6550542A68E4707D5D46949116790B";
const APP_OWNER: &str = "admin";

pub struct MockApp {
    app: App,
    cw20_id: u64,
    token_map: HashMap<String, HumanAddr>, // map token name to address
    pub oracle_addr: Option<HumanAddr>,
    pub factory_addr: Option<HumanAddr>,
}

impl MockApp {
    pub fn new() -> Self {
        let env = mock_env();
        let api = MockApi::default();
        let bank = SimpleBank {};

        let app = App::new(Box::new(api), env.block, bank, || {
            Box::new(MockStorage::new())
        });

        MockApp {
            app,
            cw20_id: 0,
            factory_addr: None,
            token_map: HashMap::new(),
            oracle_addr: None,
        }
    }

    pub fn set_cw20_contract(&mut self, code: Box<dyn Contract>) {
        self.cw20_id = self.app.store_code(code);
        self.app.update_block(next_block);
    }

    pub fn set_oracle_contract(&mut self, code: Box<dyn Contract>) {
        let code_id = self.app.store_code(code);
        let contract_addr = self
            .app
            .instantiate_contract(
                code_id,
                APP_OWNER,
                &crate::oracle::InitMsg {
                    name: None,
                    version: None,
                    admin: None,
                    min_rate: None,
                    max_rate: None,
                },
                &[],
                "oracle",
            )
            .unwrap();
        self.app.update_block(next_block);
        self.oracle_addr = Some(contract_addr);
    }

    pub fn set_factory_and_pair_contract(
        &mut self,
        factory_code: Box<dyn Contract>,
        pair_code: Box<dyn Contract>,
    ) {
        let factory_id = self.app.store_code(factory_code);
        let pair_code_id = self.app.store_code(pair_code);

        let factory_addr = self
            .app
            .instantiate_contract(
                factory_id,
                APP_OWNER,
                &crate::factory::InitMsg {
                    pair_code_id,
                    token_code_id: self.cw20_id,
                    oracle_addr: self.oracle_addr.as_ref().cloned().unwrap(),
                    commission_rate: Some(DEFAULT_COMMISSION_RATE.to_string()),
                },
                &[],
                "factory",
            )
            .unwrap();
        self.app.update_block(next_block);
        self.factory_addr = Some(factory_addr);
    }

    // configure the oraiswap pair
    pub fn set_pairs(&mut self, asset_infos_list: &[[AssetInfo; 2]]) {
        // self.oraiswap_factory_querier = OraiswapFactoryQuerier::new(pairs);
        if let Some(contract_addr) = self.factory_addr.as_ref() {
            for asset_infos in asset_infos_list.iter() {
                self.app
                    .execute_contract(
                        APP_OWNER,
                        contract_addr,
                        &crate::factory::HandleMsg::CreatePair {
                            asset_infos: asset_infos.clone(),
                        },
                        &[],
                    )
                    .unwrap();
            }
            self.app.update_block(next_block);
        }
    }

    pub fn set_pair(&mut self, asset_infos: [AssetInfo; 2]) {
        if let Some(contract_addr) = self.factory_addr.as_ref() {
            println!("factory_addr {}", contract_addr);
            self.app
                .execute_contract(
                    APP_OWNER,
                    contract_addr,
                    &crate::factory::HandleMsg::CreatePair { asset_infos },
                    &[],
                )
                .unwrap();
            self.app.update_block(next_block);
        }
    }

    pub fn register_pair(&mut self, asset_infos: [AssetInfo; 2]) {
        // self.oraiswap_factory_querier = OraiswapFactoryQuerier::new(pairs);
        if let Some(contract_addr) = self.factory_addr.as_ref() {
            self.app
                .execute_contract(
                    APP_OWNER,
                    contract_addr,
                    &crate::factory::HandleMsg::Register { asset_infos },
                    &[],
                )
                .unwrap();
            self.app.update_block(next_block);
        }
    }

    pub fn query_pair(&self, asset_infos: [AssetInfo; 2]) -> StdResult<PairInfo> {
        if let Some(contract_addr) = self.factory_addr.as_ref() {
            return self.app.wrap().query_wasm_smart(
                contract_addr,
                &crate::factory::QueryMsg::Pair { asset_infos },
            );
        }
        Err(cosmwasm_std::StdError::NotFound {
            kind: "Pair".into(),
        })
    }

    pub fn set_tax(&mut self, rate: Decimal, caps: &[(&String, &Uint128)]) {
        if let Some(contract_addr) = self.oracle_addr.as_ref() {
            // update rate
            self.app
                .execute_contract(
                    APP_OWNER,
                    contract_addr,
                    &crate::oracle::OracleMsg::Treasury(
                        crate::oracle::OracleTreasuryMsg::UpdateTaxRate { rate },
                    ),
                    &[],
                )
                .unwrap();

            // update caps
            for (denom, &cap) in caps.iter() {
                self.app
                    .execute_contract(
                        APP_OWNER,
                        contract_addr,
                        &crate::oracle::OracleMsg::Treasury(
                            crate::oracle::OracleTreasuryMsg::UpdateTaxCap {
                                denom: denom.to_string(),
                                cap: cap.clone(),
                            },
                        ),
                        &[],
                    )
                    .unwrap();
            }

            self.app.update_block(next_block);
        }
    }

    pub fn query_balance(&self, account_addr: HumanAddr, denom: String) -> StdResult<Uint128> {
        // load price form the oracle
        let balance: BalanceResponse =
            self.app
                .wrap()
                .query(&QueryRequest::Bank(BankQuery::Balance {
                    address: account_addr,
                    denom,
                }))?;
        Ok(balance.amount.amount)
    }

    pub fn query_all_balances(&self, account_addr: HumanAddr) -> StdResult<Vec<Coin>> {
        // load price form the oracle
        let all_balances: AllBalanceResponse =
            self.app
                .wrap()
                .query(&QueryRequest::Bank(BankQuery::AllBalances {
                    address: account_addr,
                }))?;
        Ok(all_balances.amount)
    }

    pub fn set_balance(&mut self, addr: HumanAddr, balance: &[Coin]) {
        // init balance for client
        self.app.set_bank_balance(addr, balance.to_vec()).unwrap();
        self.app.update_block(next_block);
    }

    pub fn as_querier(&self) -> QuerierWrapper {
        self.app.wrap()
    }

    pub fn get_token_addr(&self, token: &str) -> Option<HumanAddr> {
        self.token_map.get(token).cloned()
    }

    pub fn create_token(&mut self, token: &str) -> HumanAddr {
        let addr = self
            .app
            .instantiate_contract(
                self.cw20_id,
                APP_OWNER,
                &cw20_base::msg::InitMsg {
                    name: token.to_string(),
                    symbol: token.to_string(),
                    decimals: 6,
                    initial_balances: vec![],
                    mint: Some(cw20::MinterResponse {
                        minter: HumanAddr(APP_OWNER.to_string()),
                        cap: None,
                    }),
                },
                &[],
                "cw20",
            )
            .unwrap();
        self.app.update_block(next_block);
        addr
    }

    // configure the mint whitelist mock querier
    pub fn set_token_balances(&mut self, balances: &[(&String, &[(&String, &Uint128)])]) {
        for (token, balances) in balances.iter() {
            let contract_addr = match self.token_map.get(*token) {
                None => {
                    let addr = self.create_token(&token);
                    self.token_map.insert(token.to_string(), addr.clone());
                    addr
                }
                Some(addr) => addr.clone(),
            };

            // mint for each recipient
            for (recipient, &amount) in balances.iter() {
                self.app
                    .execute_contract(
                        APP_OWNER,
                        &contract_addr,
                        &cw20_base::msg::HandleMsg::Mint {
                            recipient: HumanAddr(recipient.to_string()),
                            amount,
                        },
                        &[],
                    )
                    .unwrap();
            }
            self.app.update_block(next_block)
        }
    }
}
