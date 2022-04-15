use oraiswap::asset::Asset;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read, Bucket, ReadonlyBucket};

pub static PREFIX_REWARD_PER_SEC: &[u8] = b"reward_per_sec";
static KEY_CONFIG: &[u8] = b"config";
static KEY_LAST_DISTRIBUTED: &[u8] = b"last_distributed";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: CanonicalAddr,
    pub staking_contract: CanonicalAddr,
    pub distribution_interval: u64,
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}

pub fn store_last_distributed(storage: &mut dyn Storage, last_distributed: u64) -> StdResult<()> {
    singleton(storage, KEY_LAST_DISTRIBUTED).save(&last_distributed)
}

pub fn read_last_distributed(storage: &dyn Storage) -> StdResult<u64> {
    singleton_read(storage, KEY_LAST_DISTRIBUTED).load()
}

pub fn store_pool_reward_per_sec(
    storage: &mut dyn Storage,
    asset_key: &[u8],
    reward: &Asset,
) -> StdResult<()> {
    Bucket::new(storage, PREFIX_REWARD_PER_SEC).save(asset_key, reward)
}

pub fn read_pool_reward_per_sec(storage: &dyn Storage, asset_key: &[u8]) -> StdResult<Asset> {
    ReadonlyBucket::new(storage, PREFIX_REWARD_PER_SEC).load(asset_key)
}
