use cosmwasm_schema::cw_serde;

use cosmwasm_std::{Addr, Uint128};
use margined_common::asset::AssetInfo;
#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateOwner {
        owner: String,
    },
    AddToken {
        token: String,
    },
    RemoveToken {
        token: String,
    },
    SendToken {
        token: String,
        amount: Uint128,
        recipient: String,
    },
}

#[cw_serde]
pub enum QueryMsg {
    Config {},
    GetOwner {},
    IsToken { token: String },
    GetTokenLength {},
    GetTokenList { limit: Option<u32> },
}

#[cw_serde]
pub struct ConfigResponse {}

#[cw_serde]
pub struct OwnerResponse {
    pub owner: Addr,
}

#[cw_serde]
pub struct TokenResponse {
    pub is_token: bool,
}

#[cw_serde]
pub struct AllTokenResponse {
    pub token_list: Vec<AssetInfo>,
}

#[cw_serde]
pub struct TokenLengthResponse {
    pub length: usize,
}
