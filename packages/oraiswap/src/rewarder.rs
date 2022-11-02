use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};

use crate::asset::AssetInfo;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub staking_contract: Addr,
    pub distribution_interval: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    ///////////////////
    /// Owner Operations
    ///////////////////
    UpdateConfig {
        owner: Option<Addr>,
        staking_contract: Option<Addr>,
        distribution_interval: Option<u64>,
    },

    // distribute for a list of pools
    Distribute {
        asset_infos: Vec<AssetInfo>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    DistributionInfo { asset_info: AssetInfo },
    RewardAmountPerSec { asset_info: AssetInfo },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: Addr,
    pub staking_contract: Addr,
    pub distribution_interval: u64,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DistributionInfoResponse {
    pub last_distributed: u64,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RewardAmountPerSecondResponse {
    pub reward_amount: Uint128,
}
