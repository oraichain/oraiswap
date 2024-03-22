use cosmwasm_schema::cw_serde;

use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use oraiswap::router::{RouterController, SwapOperation};

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub router_contract: RouterController,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const ROUTING_TABLE: Map<(&str, &str), Vec<Vec<SwapOperation>>> = Map::new("routing_table");
