use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, Decimal, StdResult, Storage, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use oracle_base::ContractInfoResponse;

pub const CONTRACT_INFO: Item<ContractInfoResponse> = Item::new("contract_info");

pub const TAX_RATE: Item<Decimal> = Item::new("tax_rate");
pub const TAX_CAP: Map<&[u8], Uint128> = Map::new("tax_cap");

pub const EXCHANGE_RATES: Map<&[u8], Decimal> = Map::new("exchange_rates");
