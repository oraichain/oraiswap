use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// OraiRoute is enum type to represent orai query route path
/// the data are stored in state storage of the smart contract
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OraiRoute {
    Market,
    Treasury,
    Oracle,
    Wasm,
}
