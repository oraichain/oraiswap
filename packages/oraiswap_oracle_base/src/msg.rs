use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::route::OraiRoute;
use cosmwasm_std::{Coin, CosmosMsg};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// OraiMsgWrapper is an override of CosmosMsg::Custom to show this works and can be extended in the contract
pub struct OraiMsgWrapper {
    pub route: OraiRoute,
    pub msg_data: OraiMsg,
}

// this is a helper to be able to return these as CosmosMsg easier
impl From<OraiMsgWrapper> for CosmosMsg<OraiMsgWrapper> {
    fn from(original: OraiMsgWrapper) -> Self {
        CosmosMsg::Custom(original)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OraiMsg {
    Swap {
        offer_coin: Coin,
        ask_denom: String,
    },
    SwapSend {
        to_address: String,
        offer_coin: Coin,
        ask_denom: String,
    },
}

// create_swap_msg returns wrapped swap msg
pub fn create_swap_msg(offer_coin: Coin, ask_denom: String) -> CosmosMsg<OraiMsgWrapper> {
    OraiMsgWrapper {
        route: OraiRoute::Market,
        msg_data: OraiMsg::Swap {
            offer_coin,
            ask_denom,
        },
    }
    .into()
}

// create_swap_send_msg returns wrapped swap send msg
pub fn create_swap_send_msg(
    to_address: String,
    offer_coin: Coin,
    ask_denom: String,
) -> CosmosMsg<OraiMsgWrapper> {
    OraiMsgWrapper {
        route: OraiRoute::Market,
        msg_data: OraiMsg::SwapSend {
            to_address,
            offer_coin,
            ask_denom,
        },
    }
    .into()
}
