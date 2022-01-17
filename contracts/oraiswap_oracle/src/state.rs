use cosmwasm_std::{Decimal, Uint128};
use cw_storage_plus::{Item, Map};
use oraiswap::oracle::ContractInfo;

// put the length bytes at the first for compatibility with legacy singleton store
pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("\u{0}\u{13}contract_info");
pub const TAX_RATE: Item<Decimal> = Item::new("\u{0}\u{8}tax_rate");

pub const TAX_CAP: Map<&[u8], Uint128> = Map::new("tax_cap");
/// Exchange rate of denom to Orai
/// (QUOTE_DENOM / ORAI)  / (BASE_DENOM / ORAI) = QUOTE_DENOM / BASE_DENOM
pub const EXCHANGE_RATES: Map<&[u8], Decimal> = Map::new("exchange_rates");
