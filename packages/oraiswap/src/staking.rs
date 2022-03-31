use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::asset::{Asset, AssetInfo};
use cosmwasm_std::{Decimal, HumanAddr, Uint128};
use cw20::Cw20ReceiveMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    // default is sender
    pub owner: Option<HumanAddr>,
    pub reward_addr: HumanAddr,
    // this for minting short token
    pub minter: Option<HumanAddr>,
    pub oracle_addr: HumanAddr,
    pub factory_addr: HumanAddr,
    pub base_denom: Option<String>,
    // this for update short token reward weight
    pub premium_min_update_interval: Option<u64>,
    pub short_reward_bound: Option<(Decimal, Decimal)>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Receive(Cw20ReceiveMsg),

    ////////////////////////
    /// Owner operations ///
    ////////////////////////
    UpdateConfig {
        owner: Option<HumanAddr>,
        premium_min_update_interval: Option<u64>,
        short_reward_bound: Option<(Decimal, Decimal)>,
    },
    RegisterAsset {
        asset_info: AssetInfo, // can be ow20 token or native token
        staking_token: HumanAddr,
    },
    DeprecateStakingToken {
        asset_info: AssetInfo,
        new_staking_token: HumanAddr,
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
    /// Provides liquidity and automatically stakes the LP tokens
    AutoStake {
        assets: [Asset; 2],
        slippage_tolerance: Option<Decimal>,
    },
    /// Hook to stake the minted LP tokens
    AutoStakeHook {
        asset_info: AssetInfo,
        staking_token: HumanAddr,
        staker_addr: HumanAddr,
        prev_staking_token_amount: Uint128,
    },

    //////////////////////////////////
    /// Permission-less operations ///
    //////////////////////////////////
    AdjustPremium {
        asset_tokens: Vec<HumanAddr>, // only support ow20 token
    },

    ////////////////////////////////
    /// Mint contract operations ///
    ////////////////////////////////
    IncreaseShortToken {
        asset_token: HumanAddr, // short token and premium only support ow20 token, it is from limit order
        staker_addr: HumanAddr,
        amount: Uint128,
    },
    DecreaseShortToken {
        asset_token: HumanAddr,
        staker_addr: HumanAddr,
        amount: Uint128,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    // this call from LP token contract
    Bond { asset_info: AssetInfo },
    // reward tokens are ow20 only, and used by admin or factory contract to deposit newly minted ORAIX tokens, which
    // will be used as rewards for the specified asset's staking pool.
    DepositReward { rewards: Vec<(HumanAddr, Uint128)> },
}

/// We currently take no arguments for migrations
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {
    pub asset_info_to_deprecate: AssetInfo,
    pub new_staking_token: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    PoolInfo {
        asset_info: AssetInfo,
    },
    RewardInfo {
        staker_addr: HumanAddr,
        asset_info: Option<AssetInfo>,
    },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: HumanAddr,
    pub reward_addr: HumanAddr,
    pub minter: HumanAddr,
    pub oracle_addr: HumanAddr,
    pub factory_addr: HumanAddr,
    pub base_denom: String,
    pub premium_min_update_interval: u64,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolInfoResponse {
    pub asset_info: AssetInfo,
    pub staking_token: HumanAddr,
    pub total_bond_amount: Uint128,
    pub total_short_amount: Uint128,
    pub reward_index: Decimal,
    pub short_reward_index: Decimal,
    pub pending_reward: Uint128,
    pub short_pending_reward: Uint128,
    pub premium_rate: Decimal,
    pub short_reward_weight: Decimal,
    pub premium_updated_time: u64,
    pub migration_index_snapshot: Option<Decimal>,
    pub migration_deprecated_staking_token: Option<HumanAddr>,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RewardInfoResponse {
    pub staker_addr: HumanAddr,
    pub reward_infos: Vec<RewardInfoResponseItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RewardInfoResponseItem {
    pub asset_info: AssetInfo,
    pub bond_amount: Uint128,
    pub pending_reward: Uint128,
    pub is_short: bool,
    // returns true if the position should be closed to keep receiving rewards
    // with the new lp token
    pub should_migrate: Option<bool>,
}
