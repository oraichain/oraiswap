use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::{Addr, Decimal};

use crate::asset::AssetInfo;
use cw20::Cw20ReceiveMsg;

#[cw_serde]
pub struct TokenInfo {
    pub info: AssetInfo,
    pub decimals: u8,
}

#[cw_serde]
pub struct TokenRatio {
    pub info: AssetInfo,
    pub ratio: Decimal,
}

#[cw_serde]
pub struct InstantiateMsg {}
#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),

    ///////////////////
    /// Owner Operations
    ///////////////////
    UpdateConfig {
        owner: Addr,
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
    WithdrawTokens {
        asset_infos: Vec<AssetInfo>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(ConvertInfoResponse)]
    ConvertInfo { asset_info: AssetInfo },
}

#[cw_serde]
pub enum Cw20HookMsg {
    Convert {},
    ConvertReverse { from: AssetInfo },
}

// We define a custom struct for each query response
#[cw_serde]
pub struct ConfigResponse {
    pub owner: Addr,
}

#[cw_serde]
pub struct ConvertInfoResponse {
    pub token_ratio: TokenRatio,
}
