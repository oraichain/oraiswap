use cosmwasm_schema::cw_serde;
use margined_common::asset::AssetInfo;

use cosmwasm_std::{Addr, Uint128};
#[cw_serde]
pub struct InstantiateMsg {
    pub engine: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateOwner { owner: String },
    AddVamm { vamm: String },
    RemoveVamm { vamm: String },
    Withdraw { token: AssetInfo, amount: Uint128 },
    ShutdownVamms {},
}

#[cw_serde]
pub enum QueryMsg {
    Config {},
    GetOwner {},
    IsVamm { vamm: String },
    GetAllVamm { limit: Option<u32> },
    GetAllVammStatus { limit: Option<u32> },
    GetVammStatus { vamm: String },
}

#[cw_serde]
pub struct ConfigResponse {
    pub engine: Addr,
}

#[cw_serde]
pub struct OwnerResponse {
    pub owner: Addr,
}

#[cw_serde]
pub struct VammResponse {
    pub is_vamm: bool,
}

#[cw_serde]
pub struct VammStatusResponse {
    pub vamm_status: bool,
}

#[cw_serde]
pub struct AllVammResponse {
    pub vamm_list: Vec<Addr>,
}

#[cw_serde]
pub struct AllVammStatusResponse {
    pub vamm_list_status: Vec<(Addr, bool)>,
}
