use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};
use oracle_base::{
    ExchangeRatesResponse, OraiMsg, OraiMsgWrapper, OraiQuery, OraiQueryWrapper, OraiRoute,
    SwapResponse, TaxCapResponse, TaxRateResponse,
};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("artifacts/schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(OraiMsgWrapper), &out_dir);
    export_schema(&schema_for!(OraiMsg), &out_dir);
    export_schema(&schema_for!(OraiQueryWrapper), &out_dir);
    export_schema(&schema_for!(OraiQuery), &out_dir);
    export_schema(&schema_for!(OraiRoute), &out_dir);
    export_schema(&schema_for!(SwapResponse), &out_dir);
    export_schema(&schema_for!(TaxCapResponse), &out_dir);
    export_schema(&schema_for!(TaxRateResponse), &out_dir);
    export_schema(&schema_for!(ExchangeRatesResponse), &out_dir);
}
