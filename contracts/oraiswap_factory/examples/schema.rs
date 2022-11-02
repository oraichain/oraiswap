use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use cosmwasm_std::Addr;
use oraiswap::asset::{AssetInfo, PairInfo};
use oraiswap::factory::{ConfigResponse, HandleMsg, InitMsg, PairsResponse, QueryMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// make AssetInfo compartible with Pair query by creating a duplicate PairResponse following naming convention
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PairResponse {
    pub asset_infos: [AssetInfo; 2],
    pub contract_addr: Addr,
    pub liquidity_token: Addr,

    pub oracle_addr: Addr,
    pub commission_rate: String,
}

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("artifacts/schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InitMsg), &out_dir);
    export_schema(&schema_for!(HandleMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(PairInfo), &out_dir);
    export_schema(&schema_for!(PairResponse), &out_dir);
    export_schema(&schema_for!(PairsResponse), &out_dir);
    export_schema(&schema_for!(ConfigResponse), &out_dir);
}
