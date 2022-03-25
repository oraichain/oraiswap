use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use oraix_protocol::staking::{
    ConfigResponse, HandleMsg as StakingHandleMsg, InitMsg as StakingInitMsg,
    QueryMsg as StakingQueryMsg,
};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("artifacts/schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(StakingInitMsg), &out_dir);
    export_schema(&schema_for!(StakingHandleMsg), &out_dir);
    export_schema(&schema_for!(StakingQueryMsg), &out_dir);
    export_schema(&schema_for!(ConfigResponse), &out_dir);
}
