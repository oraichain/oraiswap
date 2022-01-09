pub mod contract;
pub mod error;
pub mod msg;
pub mod state;

#[cfg(test)]
mod tests;

#[cfg(target_arch = "wasm32")]
cosmwasm_std::create_entry_points!(contract);

#[macro_export]
macro_rules! check_size {
    ($arg:ident, $len:expr) => {{
        if $arg.len() > $len {
            return Err(ContractError::InvalidArgument {
                reason: format!("`{}` exceeds {} chars", stringify!($arg), $len),
            });
        }
    }};
}
