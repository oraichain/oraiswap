use cosmwasm_std::{Addr, Deps};
use oraiswap::router::SwapOperation;

use crate::{state::CONFIG, ContractError};

pub fn check_is_contract_owner(deps: Deps, sender: Addr) -> Result<(), ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.owner != sender {
        Err(ContractError::Unauthorized {})
    } else {
        Ok(())
    }
}

pub fn validate_pool_route(
    _deps: Deps,
    _input_denom: String,
    _output_denom: String,
    _pool_route: Vec<SwapOperation>,
) -> Result<(), ContractError> {
    // FIXME: try simulating
    Ok(())
}