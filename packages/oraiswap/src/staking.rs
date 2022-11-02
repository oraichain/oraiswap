use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::asset::{Asset, AssetInfo};
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    // default is sender
    pub owner: Option<Addr>,
    pub rewarder: Addr,
    pub minter: Option<Addr>,
    pub oracle_addr: Addr,
    pub factory_addr: Addr,
    pub base_denom: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Receive(Cw20ReceiveMsg),

    ////////////////////////
    /// Owner operations ///
    ////////////////////////
    UpdateConfig {
        rewarder: Option<Addr>,
        owner: Option<Addr>,
    },
    RegisterAsset {
        asset_info: AssetInfo, // can be ow20 token or native token
        staking_token: Addr,
    },
    DeprecateStakingToken {
        asset_info: AssetInfo,
        new_staking_token: Addr,
    },
    // update rewards per second for an asset
    UpdateRewardsPerSec {
        asset_info: AssetInfo,
        assets: Vec<Asset>,
    },
    // reward tokens are in amount proportionaly, and used by minter contract to update amounts after checking the balance, which
    // will be used as rewards for the specified asset's staking pool.
    DepositReward {
        rewards: Vec<Asset>,
    },

    ////////////////////////
    /// User operations ///
    ////////////////////////
    Unbond {
        asset_info: AssetInfo,
        amount: Uint128,
    },
    /// Withdraw pending rewards
    Withdraw {
        // If the asset token is not given, then all rewards are withdrawn
        asset_info: Option<AssetInfo>,
    },
    // Withdraw for others in this pool, such as when rewards per second are changed for the pool
    WithdrawOthers {
        asset_info: Option<AssetInfo>,
        staker_addrs: Vec<Addr>,
    },

    /// Provides liquidity and automatically stakes the LP tokens
    AutoStake {
        assets: [Asset; 2],
        slippage_tolerance: Option<Decimal>,
    },
    /// Hook to stake the minted LP tokens
    AutoStakeHook {
        asset_info: AssetInfo,
        staking_token: Addr,
        staker_addr: Addr,
        prev_staking_token_amount: Uint128,
    },
    UpdateListStakers {
        asset_info: AssetInfo,
        stakers: Vec<Addr>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    // this call from LP token contract
    Bond { asset_info: AssetInfo },
}

/// We currently take no arguments for migrations
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {
    pub staker_addrs: Vec<Addr>,
    // pub amount_infos: Vec<AmountInfo>,
    // pub new_staking_token: Addr,
}

/// We currently take no arguments for migrations
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AmountInfo {
    pub asset_info: AssetInfo,
    pub amount: Uint128,
    // pub new_staking_token: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    PoolInfo {
        asset_info: AssetInfo,
    },
    RewardsPerSec {
        asset_info: AssetInfo,
    },
    RewardInfo {
        staker_addr: Addr,
        asset_info: Option<AssetInfo>,
    },
    // Query all staker belong to the pool
    RewardInfos {
        asset_info: AssetInfo,
        start_after: Option<Addr>,
        limit: Option<u32>,
        // so can convert or throw error
        order: Option<i32>,
    },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: Addr,
    pub rewarder: Addr,
    pub oracle_addr: Addr,
    pub factory_addr: Addr,
    pub base_denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RewardsPerSecResponse {
    pub assets: Vec<Asset>,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolInfoResponse {
    pub asset_info: AssetInfo,
    pub staking_token: Addr,
    pub total_bond_amount: Uint128,
    pub reward_index: Decimal,
    pub pending_reward: Uint128,
    pub migration_index_snapshot: Option<Decimal>,
    pub migration_deprecated_staking_token: Option<Addr>,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RewardInfoResponse {
    pub staker_addr: Addr,
    pub reward_infos: Vec<RewardInfoResponseItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RewardInfoResponseItem {
    pub asset_info: AssetInfo,
    pub bond_amount: Uint128,
    pub pending_reward: Uint128,
    pub pending_withdraw: Vec<Asset>,
    // returns true if the position should be closed to keep receiving rewards
    // with the new lp token
    pub should_migrate: Option<bool>,
}
