use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, Decimal, StdResult, Storage, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use oraiswap::{
    oracle::{ContractInfo, ContractInfoResponse},
    Decimal256, Uint256,
};

// put the length bytes at the first for compatibility with legacy singleton store
pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("\u{0}\u{13}contract_info");
pub const TAX_RATE: Item<Decimal> = Item::new("\u{0}\u{8}tax_rate");
pub const NATIVE_ORAI_POOL_DELTA: Item<Uint256> = Item::new("\u{0}\u{10}pool_delta");

pub const TAX_CAP: Map<&[u8], Uint128> = Map::new("tax_cap");
/// Exchange rate of denom to Orai
/// (QUOTE_DENOM / ORAI)  / (BASE_DENOM / ORAI) = QUOTE_DENOM / BASE_DENOM
pub const EXCHANGE_RATES: Map<&[u8], Decimal> = Map::new("exchange_rates");
pub const TOBIN_TAXES: Map<&[u8], Decimal> = Map::new("tobin_taxes");
