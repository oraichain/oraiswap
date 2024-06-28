use cosmwasm_schema::cw_serde;

use cosmwasm_std::CanonicalAddr;
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    pub factory_addr: CanonicalAddr,
    pub factory_addr_v2: CanonicalAddr,
    pub oraiswap_v3: CanonicalAddr,
}

// put the length bytes at the first for compatibility with legacy singleton store
pub const CONFIG: Item<Config> = Item::new("\u{0}\u{6}config");

#[cfg(test)]
mod test {
    use super::*;

    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::{Api, StdResult, Storage};
    use cosmwasm_storage::{singleton, singleton_read};
    const KEY_CONFIG: &[u8] = b"config";

    pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
        singleton(storage, KEY_CONFIG).save(config)
    }
    pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
        singleton_read(storage, KEY_CONFIG).load()
    }

    #[test]
    fn legacy_compatibility() {
        let mut deps = mock_dependencies();
        store_config(
            &mut deps.storage,
            &Config {
                factory_addr: deps.api.addr_canonicalize("addr0000").unwrap(),
                factory_addr_v2: deps.api.addr_canonicalize("addr0000_v2").unwrap(),
            },
        )
        .unwrap();

        assert_eq!(
            CONFIG.load(&deps.storage).unwrap(),
            read_config(&deps.storage).unwrap()
        );
    }
}
