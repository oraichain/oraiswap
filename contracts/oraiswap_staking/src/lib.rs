pub mod contract;
mod migration;
mod rewards;
mod staking;
mod state;

#[cfg(all(target_arch = "wasm32", not(feature = "library")))]
cosmwasm_std::create_entry_points_with_migration!(contract);

#[cfg(test)]
mod testing;
