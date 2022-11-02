use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::{Addr, Uint128};

use crate::asset::AssetInfo;

#[cw_serde]
pub struct InstantiateMsg {
    pub staking_contract: Addr,
    pub distribution_interval: Option<u64>,
}

#[cw_serde]
pub enum ExecuteMsg {
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

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(DistributionInfoResponse)]
    DistributionInfo { asset_info: AssetInfo },
    #[returns(RewardAmountPerSecondResponse)]
    RewardAmountPerSec { asset_info: AssetInfo },
}

// We define a custom struct for each query response
#[cw_serde]
pub struct ConfigResponse {
    pub owner: Addr,
    pub staking_contract: Addr,
    pub distribution_interval: u64,
}

// We define a custom struct for each query response
#[cw_serde]
pub struct DistributionInfoResponse {
    pub last_distributed: u64,
}

// We define a custom struct for each query response
#[cw_serde]
pub struct RewardAmountPerSecondResponse {
    pub reward_amount: Uint128,
}
