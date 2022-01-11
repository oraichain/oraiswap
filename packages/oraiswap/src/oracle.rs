use cosmwasm_std::HumanAddr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    /// name of the NFT contract, can use default
    pub name: Option<String>,
    pub version: Option<String>,
    pub address: HumanAddr,
    pub creator: HumanAddr,
}
