pub mod asset;
pub mod converter;
pub mod error;
pub mod factory;
pub mod math;
pub mod mixed_router;
pub mod oracle;
pub mod orderbook;
pub mod pair;
pub mod querier;
pub mod response;
pub mod rewarder;
pub mod router;
pub mod smartrouter;
pub mod staking;

#[cfg(not(target_arch = "wasm32"))]
pub use cw_multi_test;

// for other to use, but not compile to wasm
#[cfg(not(target_arch = "wasm32"))]
pub mod testing;
