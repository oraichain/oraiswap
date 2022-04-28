use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{HumanAddr, Uint128};

use crate::asset::{Asset, AssetInfo};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    ///////////////////
    /// Owner Operations
    ///////////////////
    UpdateConfig {
        owner: HumanAddr,
    },
    Convert {
        asset: Asset,
    },
    UpdateConvertInfoMsg {
        from: AssetInfo,
        to_token: AssetInfo,
        from_to_ratio: u128,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    ConvertInfo { asset_info: AssetInfo },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConvertInfoResponse {
    pub to_token: AssetInfo,
    pub from_to_ratio: u128,
}

// We define a custom struct for each query response
pub enum Cw20HookMsg {
    // this call from LP token contract
    Convert { asset_info: AssetInfo },
}
