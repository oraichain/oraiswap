pub mod contract;
pub mod state;

mod operations;

#[cfg(all(target_arch = "wasm32", not(feature = "library")))]
cosmwasm_std::create_entry_points!(contract);

#[cfg(test)]
mod testing;
