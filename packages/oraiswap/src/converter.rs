use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Decimal, HumanAddr};

use crate::asset::AssetInfo;
use cw20::Cw20ReceiveMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenInfo {
    pub info: AssetInfo,
    pub decimals: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenRatio {
    pub info: AssetInfo,
    pub ratio: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Receive(Cw20ReceiveMsg),

    ///////////////////
    /// Owner Operations
    ///////////////////
    UpdateConfig {
        owner: HumanAddr,
    },
    Convert {},
    UpdatePair {
        from: TokenInfo,
        to: TokenInfo,
    },
    UnregisterPair {
        from: TokenInfo,
    },
    ConvertReverse {
        from_asset: AssetInfo,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    ConvertInfo { asset_info: AssetInfo },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    Convert {},
    ConvertReverse { from: AssetInfo },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConvertInfoResponse {
    pub token_ratio: TokenRatio,
}
