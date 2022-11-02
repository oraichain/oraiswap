use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};
use cosmwasm_std::{
    Addr, AllBalanceResponse, Attribute, BalanceResponse, BankQuery, Binary, Coin, Decimal,
    QuerierWrapper, QueryRequest, StdResult, Uint128,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;

use crate::asset::{AssetInfo, PairInfo};

use crate::pair::DEFAULT_COMMISSION_RATE;
use cw_multi_test::{next_block, App, Contract, SimpleBank};

pub const ATOM_DENOM: &str = "ibc/1777D03C5392415FE659F0E8ECB2CE553C6550542A68E4707D5D46949116790B";
const APP_OWNER: &str = "admin";

#[derive(Default, Clone, Debug)]
pub struct Response {
    pub attributes: Vec<Attribute>,
    pub data: Option<Binary>,
}

pub struct MockApp {
    app: App,
    pub token_id: u64,
    pub token_map: HashMap<String, Addr>, // map token name to address
    pub oracle_addr: Addr,
    pub factory_addr: Addr,
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
            token_id: 0,
            factory_addr: Addr::default(),
            token_map: HashMap::new(),
            oracle_addr: Addr::default(),
        }
    }

    pub fn set_token_contract(&mut self, code: Box<dyn Contract>) {
        self.token_id = self.upload(code);
    }

    pub fn upload(&mut self, code: Box<dyn Contract>) -> u64 {
        let code_id = self.app.store_code(code);
        self.app.update_block(next_block);
        code_id
    }

    pub fn instantiate<T: Serialize>(
        &mut self,
        code_id: u64,
        sender: Addr,
        init_msg: &T,
        send_funds: &[Coin],
        label: &str,
    ) -> Result<Addr, String> {
        let contract_addr = self
            .app
            .instantiate_contract(code_id, sender, init_msg, send_funds, label)?;
        // simulate bank transfer when run sent_funds
        self.set_balance(contract_addr.clone(), send_funds);
        self.app.update_block(next_block);
        Ok(contract_addr)
    }

    pub fn execute<T: Serialize>(
        &mut self,
        sender: Addr,
        contract_addr: Addr,
        msg: &T,
        send_funds: &[Coin],
    ) -> Result<Response, String> {
        // simulate bank transfer when run sent_funds by updating the balance
        let mut balance = self.query_all_balances(contract_addr.clone()).unwrap();
        for fund in send_funds {
            if let Some(current_fund) = balance.iter_mut().find(|v| v.denom.eq(&fund.denom)) {
                current_fund.amount += fund.amount;
            } else {
                balance.push(fund.clone());
            }
        }

        self.set_balance(contract_addr.clone(), &balance);

        let response = self
            .app
            .execute_contract(sender, contract_addr, msg, send_funds)?;

        self.app.update_block(next_block);
        Ok(Response {
            attributes: response.attributes,
            data: response.data,
        })
    }

    pub fn query<T: DeserializeOwned, U: Serialize>(
        &self,
        contract_addr: Addr,
        msg: &U,
    ) -> StdResult<T> {
        self.app.wrap().query_wasm_smart(contract_addr, msg)
    }

    pub fn set_oracle_contract(&mut self, code: Box<dyn Contract>) {
        let code_id = self.upload(code);
        self.oracle_addr = self
            .instantiate(
                code_id,
                APP_OWNER.into(),
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
    }

    pub fn set_factory_and_pair_contract(
        &mut self,
        factory_code: Box<dyn Contract>,
        pair_code: Box<dyn Contract>,
    ) {
        let factory_id = self.upload(factory_code);
        let pair_code_id = self.upload(pair_code);

        self.factory_addr = self
            .instantiate(
                factory_id,
                APP_OWNER.into(),
                &crate::factory::InitMsg {
                    pair_code_id,
                    token_code_id: self.token_id,
                    oracle_addr: self.oracle_addr.clone(),
                    commission_rate: Some(DEFAULT_COMMISSION_RATE.to_string()),
                },
                &[],
                "factory",
            )
            .unwrap();
    }

    // configure the oraiswap pair
    pub fn set_pairs(&mut self, asset_infos_list: &[[AssetInfo; 2]]) {
        for asset_infos in asset_infos_list.iter() {
            self.set_pair(asset_infos.clone());
        }
    }

    pub fn set_pair(&mut self, asset_infos: [AssetInfo; 2]) -> Option<Addr> {
        if !self.factory_addr.is_empty() {
            let crate::factory::ConfigResponse {
                token_code_id,
                pair_code_id,
                oracle_addr,
                ..
            } = self
                .as_querier()
                .query_wasm_smart(
                    self.factory_addr.clone(),
                    &crate::factory::QueryMsg::Config {},
                )
                .unwrap();

            let pair_addr = self
                .instantiate(
                    pair_code_id,
                    APP_OWNER.into(),
                    &crate::pair::InitMsg {
                        asset_infos: asset_infos.clone(),
                        token_code_id,
                        oracle_addr,
                        commission_rate: Some(DEFAULT_COMMISSION_RATE.to_string()),
                        init_hook: None,
                    },
                    &[],
                    "pair",
                )
                .unwrap();

            self.execute(
                pair_addr.clone(),
                self.factory_addr.clone(),
                &crate::factory::HandleMsg::CreatePair {
                    asset_infos: asset_infos.clone(),
                    auto_register: false,
                },
                &[],
            )
            .unwrap();

            // then register
            self.execute(
                pair_addr.clone(),
                self.factory_addr.clone(),
                &crate::factory::HandleMsg::Register { asset_infos },
                &[],
            )
            .unwrap();

            return Some(pair_addr);
        }

        None
    }

    pub fn query_pair(&self, asset_infos: [AssetInfo; 2]) -> StdResult<PairInfo> {
        if !self.factory_addr.is_empty() {
            return self.app.wrap().query_wasm_smart(
                self.factory_addr.clone(),
                &crate::factory::QueryMsg::Pair { asset_infos },
            );
        }
        Err(cosmwasm_std::StdError::NotFound {
            kind: "Pair".into(),
        })
    }

    pub fn set_tax(&mut self, rate: Decimal, caps: &[(&String, &Uint128)]) {
        if !self.oracle_addr.is_empty() {
            // update rate
            self.execute(
                APP_OWNER.into(),
                self.oracle_addr.clone(),
                &crate::oracle::OracleMsg::Treasury(
                    crate::oracle::OracleTreasuryMsg::UpdateTaxRate { rate },
                ),
                &[],
            )
            .unwrap();

            // update caps
            for (denom, &cap) in caps.iter() {
                self.execute(
                    APP_OWNER.into(),
                    self.oracle_addr.clone(),
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
        }
    }

    pub fn query_balance(&self, account_addr: Addr, denom: String) -> StdResult<Uint128> {
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

    pub fn query_all_balances(&self, account_addr: Addr) -> StdResult<Vec<Coin>> {
        // load price form the oracle
        let all_balances: AllBalanceResponse =
            self.app
                .wrap()
                .query(&QueryRequest::Bank(BankQuery::AllBalances {
                    address: account_addr,
                }))?;
        Ok(all_balances.amount)
    }

    pub fn register_token(&mut self, contract_addr: Addr) -> StdResult<String> {
        let res: cw20::TokenInfoResponse =
            self.query(contract_addr.clone(), &cw20::Cw20QueryMsg::TokenInfo {})?;
        self.token_map.insert(res.symbol.clone(), contract_addr);
        Ok(res.symbol)
    }

    pub fn query_token_balances(&self, account_addr: Addr) -> StdResult<Vec<Coin>> {
        let mut balances = vec![];
        for (denom, contract_addr) in self.token_map.iter() {
            let res: cw20::BalanceResponse = self.query(
                contract_addr.clone(),
                &cw20::Cw20QueryMsg::Balance {
                    address: account_addr.clone(),
                },
            )?;
            balances.push(Coin {
                denom: denom.clone(),
                amount: res.balance,
            });
        }
        Ok(balances)
    }

    pub fn set_balance(&mut self, addr: Addr, balance: &[Coin]) {
        // init balance for client
        self.app.set_bank_balance(addr, balance.to_vec()).unwrap();
        self.app.update_block(next_block);
    }

    pub fn as_querier(&self) -> QuerierWrapper {
        self.app.wrap()
    }

    pub fn get_token_addr(&self, token: &str) -> Option<Addr> {
        self.token_map.get(token).cloned()
    }

    pub fn create_token(&mut self, token: &str) -> Addr {
        let addr = self
            .instantiate(
                self.token_id,
                APP_OWNER.into(),
                &cw20_base::msg::InitMsg {
                    name: token.to_string(),
                    symbol: token.to_string(),
                    decimals: 6,
                    initial_balances: vec![],
                    mint: Some(cw20::MinterResponse {
                        minter: Addr(APP_OWNER.to_string()),
                        cap: None,
                    }),
                },
                &[],
                "cw20",
            )
            .unwrap();
        self.token_map.insert(token.to_string(), addr.clone());
        addr
    }

    pub fn set_token_balances_from(
        &mut self,
        sender: Addr,
        balances: &[(&String, &[(&String, &Uint128)])],
    ) {
        for (token, balances) in balances.iter() {
            let contract_addr = match self.token_map.get(*token) {
                None => self.create_token(&token),
                Some(addr) => addr.clone(),
            };

            // mint for each recipient
            for (recipient, &amount) in balances.iter() {
                if !amount.is_zero() {
                    self.execute(
                        sender.clone(),
                        contract_addr.clone(),
                        &cw20_base::msg::HandleMsg::Mint {
                            recipient: Addr(recipient.to_string()),
                            amount,
                        },
                        &[],
                    )
                    .unwrap();
                }
            }
        }
    }

    // configure the mint whitelist mock querier
    pub fn set_token_balances(&mut self, balances: &[(&String, &[(&String, &Uint128)])]) {
        self.set_token_balances_from(APP_OWNER.into(), balances)
    }
}
