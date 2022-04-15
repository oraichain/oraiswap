use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, Decimal, HumanAddr, Uint128};

use crate::asset::AssetInfo;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub token_code_id: u64,
    pub base_denom: String,
    pub staking_contract: HumanAddr,
    pub distribution_schedule: Vec<(u64, u64, Uint128)>, // [[start_time, end_time, distribution_amount], [], ...]
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    ///////////////////
    /// Owner Operations
    ///////////////////
    UpdateConfig {
        owner: Option<HumanAddr>,
        token_code_id: Option<u64>,
        distribution_schedule: Option<Vec<(u64, u64, Uint128)>>, // [[start_time, end_time, distribution_amount], [], ...]
    },

    UpdateRewardPerSec {
        owner: Option<HumanAddr>,
        reward_per_sec: Vec<(AssetInfo, u128)>,
    },

    Distribute {
        asset_info: AssetInfo,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    DistributionInfo {},
    RewardWeights {
        staking_contract_addr: HumanAddr,
        asset_info: AssetInfo,
    },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: HumanAddr,
    pub token_code_id: u64,
    pub base_denom: String,
    pub genesis_time: u64,
    pub distribution_schedule: Vec<(u64, u64, Uint128)>,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DistributionInfoResponse {
    pub last_distributed: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {
    pub tefi_oracle_contract: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Params {
    /// Auction discount rate applied to asset mint
    pub auction_discount: Decimal,
    /// Minium collateral ratio applied to asset mint
    pub min_collateral_ratio: Decimal,
    /// Distribution weight (default is 30, which is 1/10 of MIR distribution weight)
    pub weight: Option<u32>,
    /// For pre-IPO assets, time period after asset creation in which minting is enabled
    pub mint_period: Option<u64>,
    /// For pre-IPO assets, collateral ratio for the asset after ipo
    pub min_collateral_ratio_after_ipo: Option<Decimal>,
    /// For pre-IPO assets, fixed price during minting period
    pub pre_ipo_price: Option<Decimal>,
    /// For pre-IPO assets, address authorized to trigger the ipo event
    pub ipo_trigger_addr: Option<String>,
}
