pub mod asset;
pub mod converter;
pub mod error;
pub mod factory;
pub mod ibc;
pub mod limit_order;
pub mod math;
pub mod oracle;
pub mod pair;
pub mod querier;
pub mod response;
pub mod rewarder;
pub mod router;
pub mod staking;

#[cfg(not(target_arch = "wasm32"))]
pub use cw_multi_test;

// for other to use, but not compile to wasm
#[cfg(not(target_arch = "wasm32"))]
pub mod testing;
