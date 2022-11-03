use cosmwasm_schema::write_api;

use oraiswap::oracle::{InstantiateMsg, OracleMsg as ExecuteMsg, OracleQuery as QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg,
    }
}
