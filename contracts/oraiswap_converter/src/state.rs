use cosmwasm_schema::cw_serde;

use cosmwasm_std::{CanonicalAddr, StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read, Bucket, ReadonlyBucket};

static KEY_CONFIG: &[u8] = b"config";
static KEY_TOKEN_RATIO: &[u8] = b"token_ratio";

use oraiswap::converter::TokenRatio;

#[cw_serde]
pub struct Config {
    pub owner: CanonicalAddr,
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}

pub fn store_token_ratio(
    storage: &mut dyn Storage,
    asset_key: &[u8],
    token_ratio: &TokenRatio,
) -> StdResult<()> {
    Bucket::new(storage, KEY_TOKEN_RATIO).save(asset_key, token_ratio)
}

pub fn read_token_ratio(storage: &dyn Storage, asset_key: &[u8]) -> StdResult<TokenRatio> {
    ReadonlyBucket::new(storage, KEY_TOKEN_RATIO).load(asset_key)
}

// pub fn token_ratio_store<'a>(storage: &'a mut dyn Storage) -> Bucket<'a, TokenRatio> {
//     Bucket::new(storage, KEY_TOKEN_RATIO)
// }

pub fn token_ratio_remove<'a>(storage: &'a mut dyn Storage, asset_key: &[u8]) {
    Bucket::<'a, TokenRatio>::new(storage, KEY_TOKEN_RATIO).remove(asset_key)
}
