use cosmwasm_schema::cw_serde;

use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
pub enum Direction {
    AddToAmm,
    RemoveFromAmm,
}

#[cw_serde]
pub struct InstantiateMsg {
    pub oracle_hub_contract: String, // address of the oracle hub we are using
}

#[cw_serde]
pub enum ExecuteMsg {
    AppendPrice {
        key: String,
        price: Uint128,
        timestamp: u64,
    },
    AppendMultiplePrice {
        key: String,
        prices: Vec<Uint128>,
        timestamps: Vec<u64>,
    },
    UpdateOwner {
        owner: String,
    },
}

#[cw_serde]
pub enum QueryMsg {
    Config {},
    GetOwner {},
    GetPrice {
        key: String,
    },
    GetPreviousPrice {
        key: String,
        num_round_back: Uint128,
    },
    GetTwapPrice {
        key: String,
        interval: u64,
    },
}

#[cw_serde]
pub struct ConfigResponse {}

#[cw_serde]
pub struct OwnerResponse {
    pub owner: Addr,
}
