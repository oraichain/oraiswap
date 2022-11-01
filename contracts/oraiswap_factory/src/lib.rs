pub mod contract;
pub mod state;

mod querier;

#[cfg(all(target_arch = "wasm32", not(feature = "library")))]
cosmwasm_std::create_entry_points_with_migration!(contract);

#[cfg(test)]
mod testing;

// for other to use, but not compile to wasm
#[cfg(not(target_arch = "wasm32"))]
pub mod testutils {
    oraiswap::create_entry_points_testing!(contract);
}
