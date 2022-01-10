use cosmwasm_std::HumanAddr;
use cw_storage_plus::Item;
use oraiswap::asset::PairInfoRaw;

// put the length bytes at the first for compatibility with legacy singleton store
pub const PAIR_INFO: Item<PairInfoRaw> = Item::new("\u{0}\u{9}pair_info");
pub const ORACLE_INFO: Item<HumanAddr> = Item::new("oracle_info");

#[cfg(test)]
mod test {
    use super::*;

    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::{Api, HumanAddr, StdResult, Storage};
    use cosmwasm_storage::{singleton, singleton_read};
    use oraiswap::asset::AssetInfoRaw;
    const KEY_PAIR_INFO: &[u8] = b"pair_info";

    pub fn store_pair_info(storage: &mut dyn Storage, config: &PairInfoRaw) -> StdResult<()> {
        singleton(storage, KEY_PAIR_INFO).save(config)
    }
    pub fn read_pair_info(storage: &dyn Storage) -> StdResult<PairInfoRaw> {
        singleton_read(storage, KEY_PAIR_INFO).load()
    }

    #[test]
    fn legacy_compatibility() {
        let mut deps = mock_dependencies(&[]);
        store_pair_info(
            &mut deps.storage,
            &PairInfoRaw {
                creator: HumanAddr("creator0000".to_string()),
                asset_infos: [
                    AssetInfoRaw::NativeToken {
                        denom: "uusd".to_string(),
                    },
                    AssetInfoRaw::Token {
                        contract_addr: deps
                            .api
                            .canonical_address(&HumanAddr::from("token0000".to_string()))
                            .unwrap(),
                    },
                ],

                contract_addr: deps
                    .api
                    .canonical_address(&HumanAddr::from("pair0000".to_string()))
                    .unwrap(),

                liquidity_token: deps
                    .api
                    .canonical_address(&HumanAddr::from("liquidity0000".to_string()))
                    .unwrap(),
            },
        )
        .unwrap();

        assert_eq!(
            PAIR_INFO.load(&deps.storage).unwrap(),
            read_pair_info(&deps.storage).unwrap()
        );
    }
}
