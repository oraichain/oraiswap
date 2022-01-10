use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, StdResult, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use oracle_base::ContractInfoResponse;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenInfo {
    /// The owner of the newly minter NFT
    pub owner: CanonicalAddr,
    /// approvals are stored here, as we clear them all upon transfer and cannot accumulate much

    ///

    /// Identifies the asset to which this NFT represents
    pub name: String,
    /// Describes the asset to which this NFT represents
    pub description: String,
    /// A URI pointing to an image representing the asset
    pub image: String,
}

pub const CONTRACT_INFO: Item<ContractInfoResponse> = Item::new("nft_info");
pub const TOKEN_COUNT: Item<u64> = Item::new("num_tokens");

pub fn num_tokens(storage: &dyn Storage) -> StdResult<u64> {
    Ok(TOKEN_COUNT.may_load(storage)?.unwrap_or_default())
}

pub fn increment_tokens(storage: &mut dyn Storage) -> StdResult<u64> {
    let val = num_tokens(storage)? + 1;
    TOKEN_COUNT.save(storage, &val)?;
    Ok(val)
}

pub fn decrement_tokens(storage: &mut dyn Storage) -> StdResult<u64> {
    let val = num_tokens(storage)? - 1;
    TOKEN_COUNT.save(storage, &val)?;
    Ok(val)
}

pub struct TokenIndexes<'a> {
    pub owner: MultiIndex<'a, TokenInfo>,
}

impl<'a> IndexList<TokenInfo> for TokenIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<TokenInfo>> + '_> {
        let v: Vec<&dyn Index<TokenInfo>> = vec![&self.owner];
        Box::new(v.into_iter())
    }
}

pub fn tokens<'a>() -> IndexedMap<'a, &'a str, TokenInfo, TokenIndexes<'a>> {
    let indexes = TokenIndexes {
        owner: MultiIndex::new(|d| d.owner.to_vec(), "tokens", "tokens__owner"),
    };
    IndexedMap::new("tokens", indexes)
}
