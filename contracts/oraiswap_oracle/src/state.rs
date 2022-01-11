use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, StdResult, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use oracle_base::ContractInfoResponse;

// put the length bytes at the first for compatibility with legacy singleton store
pub const CONTRACT_INFO: Item<ContractInfoResponse> = Item::new("\u{0}\u{13}contract_info");
