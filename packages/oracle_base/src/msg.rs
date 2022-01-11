use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{to_binary, Coin, CosmosMsg, HumanAddr, WasmMsg};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OracleMarketMsg {
    Swap {
        offer_coin: Coin,
        ask_denom: String,
    },
    SwapSend {
        to_address: HumanAddr,
        offer_coin: Coin,
        ask_denom: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OracleMsg {
    Market(OracleMarketMsg),
}

// create_swap_msg returns wrapped swap msg
pub fn create_swap_msg(oracle_addr: HumanAddr, offer_coin: Coin, ask_denom: String) -> CosmosMsg {
    CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: oracle_addr,
        msg: to_binary(&OracleMsg::Market(OracleMarketMsg::Swap {
            offer_coin,
            ask_denom,
        }))
        .unwrap(),
        send: vec![],
    })
}

// create_swap_send_msg returns wrapped swap send msg
pub fn create_swap_send_msg(
    oracle_addr: HumanAddr,
    to_address: HumanAddr,
    offer_coin: Coin,
    ask_denom: String,
) -> CosmosMsg {
    CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: oracle_addr,
        msg: to_binary(&OracleMsg::Market(OracleMarketMsg::SwapSend {
            to_address,
            offer_coin,
            ask_denom,
        }))
        .unwrap(),
        send: vec![],
    })
}
