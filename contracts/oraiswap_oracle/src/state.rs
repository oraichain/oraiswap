use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, Decimal, StdResult, Storage, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use oracle_base::ContractInfoResponse;

// put the length bytes at the first for compatibility with legacy singleton store
pub const CONTRACT_INFO: Item<ContractInfoResponse> = Item::new("\u{0}\u{13}contract_info");
pub const TAX_RATE: Item<Decimal> = Item::new("\u{0}\u{8}tax_rate");

pub const TAX_CAP: Map<&[u8], Uint128> = Map::new("tax_cap");
pub const EXCHANGE_RATES: Map<&[u8], Decimal> = Map::new("exchange_rates");
