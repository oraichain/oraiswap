use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};
use oracle_base::{
    ExchangeRatesResponse, OracleContractQuery, OracleExchangeQuery, OracleMarketMsg,
    OracleMarketQuery, OracleMsg, OracleQuery, OracleTreasuryQuery, SwapResponse, TaxCapResponse,
    TaxRateResponse,
};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("artifacts/schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(OracleMsg), &out_dir);
    export_schema(&schema_for!(OracleMarketMsg), &out_dir);
    export_schema(&schema_for!(OracleQuery), &out_dir);
    export_schema(&schema_for!(OracleContractQuery), &out_dir);
    export_schema(&schema_for!(OracleExchangeQuery), &out_dir);
    export_schema(&schema_for!(OracleMarketQuery), &out_dir);
    export_schema(&schema_for!(OracleTreasuryQuery), &out_dir);
    export_schema(&schema_for!(SwapResponse), &out_dir);
    export_schema(&schema_for!(TaxCapResponse), &out_dir);
    export_schema(&schema_for!(TaxRateResponse), &out_dir);
    export_schema(&schema_for!(ExchangeRatesResponse), &out_dir);
}
