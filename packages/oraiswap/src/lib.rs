pub mod asset;
pub mod converter;
pub mod error;
pub mod factory;
pub mod hook;
pub mod oracle;
pub mod pair;
pub mod querier;
pub mod rewarder;
pub mod router;
pub mod staking;
pub mod token;

mod math;
pub use crate::math::{Decimal256, Uint256};

// for other to use, but not compile to wasm
#[cfg(not(target_arch = "wasm32"))]
pub mod mock_app;

#[cfg(test)]
mod testing;
