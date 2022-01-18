use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    asset::{Asset, AssetInfo},
    hook::InitHook,
};

use cosmwasm_std::{Decimal, HumanAddr, Uint128};
use cw20::Cw20ReceiveMsg;

/// Default commission rate == 0.3%
/// in the future need to update ?
pub const DEFAULT_COMMISSION_RATE: &str = "0.003";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    /// Asset infos
    pub asset_infos: [AssetInfo; 2],
    /// Token contract code id for initialization
    pub token_code_id: u64,

    /// Oracle contract for query oracle information
    pub oracle_addr: HumanAddr,

    pub commission_rate: Option<String>,

    /// Hook for post initalization
    pub init_hook: Option<InitHook>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Receive(Cw20ReceiveMsg),
    /// Post initize step to allow user to set controlled contract address after creating it
    PostInitialize {},
    /// ProvideLiquidity a user provides pool liquidity
    ProvideLiquidity {
        assets: [Asset; 2],
        slippage_tolerance: Option<Decimal>,
        receiver: Option<HumanAddr>,
    },
    /// Swap an offer asset to the other
    Swap {
        offer_asset: Asset,
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
        to: Option<HumanAddr>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Sell a given amount of asset
    Swap {
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
        to: Option<String>,
    },
    WithdrawLiquidity {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Pair {},
    Pool {},
    Simulation { offer_asset: Asset },
    ReverseSimulation { ask_asset: Asset },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolResponse {
    pub assets: [Asset; 2],
    pub total_share: Uint128,
}

/// SimulationResponse returns swap simulation response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SimulationResponse {
    pub return_amount: Uint128,
    pub spread_amount: Uint128,
    pub commission_amount: Uint128,
}

/// ReverseSimulationResponse returns reverse swap simulation response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ReverseSimulationResponse {
    pub offer_amount: Uint128,
    pub spread_amount: Uint128,
    pub commission_amount: Uint128,
}

/// We currently take no arguments for migrations
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}