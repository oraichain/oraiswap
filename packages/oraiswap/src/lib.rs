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
#[cfg(test)]
pub mod mock_app;

#[cfg(test)]
mod testing;

#[cfg(test)]
#[macro_export]
macro_rules! create_entry_points_testing {
    ($contract:ident) => {
        Box::new(cw_multi_test::ContractWrapper::new(
            $contract::contract::execute,
            $contract::contract::instantiate,
            $contract::contract::query,
        ))
    };
}
