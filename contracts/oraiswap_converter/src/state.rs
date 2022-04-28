use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, StdResult, Storage, Uint128};
use cosmwasm_storage::{singleton, singleton_read, Bucket, ReadonlyBucket};

static KEY_CONFIG: &[u8] = b"config";
static KEY_CONVERT_INFO: &[u8] = b"convert_info";

use oraiswap::asset::{Asset, AssetInfo};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: CanonicalAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConvertInfo {
    pub to_token: AssetInfo,
    pub from_to_ratio: u128, //fromAmount * fromToRatio = toAmount
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}

pub fn store_convert_info(
    storage: &mut dyn Storage,
    asset_key: &[u8],
    convert_info: &ConvertInfo,
) -> StdResult<()> {
    Bucket::new(storage, KEY_CONVERT_INFO).save(asset_key, convert_info)
}

pub fn read_convert_info(storage: &dyn Storage, asset_key: &[u8]) -> StdResult<ConvertInfo> {
    ReadonlyBucket::new(storage, KEY_CONVERT_INFO).load(asset_key)
}
