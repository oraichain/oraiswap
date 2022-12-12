use cosmwasm_schema::write_api;

use cw20_ics20::msg::{ExecuteMsg, InitMsg, MigrateMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InitMsg,
        execute: ExecuteMsg,
        query: QueryMsg,
        migrate: MigrateMsg,
    }
}
