use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, StdResult, Storage, Uint128};
use cosmwasm_storage::{singleton, singleton_read, Bucket, ReadonlyBucket, Singleton};

static KEY_CONFIG: &[u8] = b"config";
static KEY_LAST_DISTRIBUTED: &[u8] = b"last_distributed";
pub static PREFIX_REWARD_PER_SEC: &[u8] = b"reward_per_sec";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: CanonicalAddr,
    pub staking_contract: CanonicalAddr,
    pub token_code_id: u64, // used to create asset token
    pub base_denom: String,
    pub genesis_time: u64,
    pub distribution_schedule: Vec<(u64, u64, Uint128)>, // [[start_time, end_time, distribution_amount], [], ...]
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}

pub fn store_last_distributed(storage: &mut dyn Storage, last_distributed: u64) -> StdResult<()> {
    let mut store: Singleton<u64> = singleton(storage, KEY_LAST_DISTRIBUTED);
    store.save(&last_distributed)
}

pub fn read_last_distributed(storage: &dyn Storage) -> StdResult<u64> {
    singleton_read(storage, KEY_LAST_DISTRIBUTED).load()
}

pub fn store_pool_reward_per_sec(
    storage: &mut dyn Storage,
    asset_key: &[u8],
    reward_per_sec: &u128,
) -> StdResult<()> {
    Bucket::new(storage, PREFIX_REWARD_PER_SEC).save(asset_key, reward_per_sec)
}

pub fn read_pool_reward_per_sec(storage: &dyn Storage, asset_key: &[u8]) -> StdResult<u128> {
    ReadonlyBucket::new(storage, PREFIX_REWARD_PER_SEC).load(asset_key)
}
