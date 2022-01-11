use cw_storage_plus::Item;
use oraiswap::asset::PairInfoRaw;

pub const PAIR_INFO: Item<PairInfoRaw> = Item::new("pair_info");

#[cfg(test)]
mod test {
    use super::*;

    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::{Api, StdResult, Storage};
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
                creator: deps.api.canonical_address(&"creator0000".into()).unwrap(),
                oracle_addr: deps.api.canonical_address(&"oracle0000".into()).unwrap(),
                asset_infos: [
                    AssetInfoRaw::NativeToken {
                        denom: "uusd".to_string(),
                    },
                    AssetInfoRaw::Token {
                        contract_addr: deps.api.canonical_address(&"token0000".into()).unwrap(),
                    },
                ],
                contract_addr: deps.api.canonical_address(&"pair0000".into()).unwrap(),
                liquidity_token: deps.api.canonical_address(&"liquidity0000".into()).unwrap(),
            },
        )
        .unwrap();

        assert_eq!(
            PAIR_INFO.load(&deps.storage).unwrap(),
            read_pair_info(&deps.storage).unwrap()
        );
    }
}
