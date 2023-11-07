use cosmwasm_schema::write_api;

use oraiswap::oracle::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, OracleContractQuery, OracleExchangeQuery,
    OracleTreasuryQuery, QueryMsg,
};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg<OracleTreasuryQuery,OracleExchangeQuery,OracleContractQuery>,
        migrate: MigrateMsg
    }
}
