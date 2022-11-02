use cosmwasm_std::{Addr, Binary};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// this is to trigger an action after execution
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitHook {
    pub msg: Binary,
    pub contract_addr: Addr,
}
