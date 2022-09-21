use cosmwasm_std::{CanonicalAddr, Storage, Uint128};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};

const TAX_RECEIVER_KEY: &[u8] = b"tax_receiver";

pub const TAX_RATE: Uint128 = Uint128(5u128);

// meta is the token definition as well as the total_supply
pub fn tax_receiver(storage: &mut dyn Storage) -> Singleton<CanonicalAddr> {
    singleton(storage, TAX_RECEIVER_KEY)
}

pub fn tax_receiver_read(storage: &dyn Storage) -> ReadonlySingleton<CanonicalAddr> {
    singleton_read(storage, TAX_RECEIVER_KEY)
}
