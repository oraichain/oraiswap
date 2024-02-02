use cosmwasm_schema::cw_serde;
use cosmwasm_std::{entry_point, Addr, StdResult};
use cosmwasm_std::{DepsMut, Env, Response};

use crate::contract::{distribute, read_staking_tokens};
use crate::state::{read_config, Config};

#[cw_serde]
pub enum SudoMsg {
    // default message for Sudo, hard-coded in the Juno module: https://github.com/CosmosContracts/juno/blob/main/x/clock/types/msgs.go#L13
    ClockEndBlock {},
}

// contract.rs
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut, env: Env, msg: SudoMsg) -> StdResult<Response> {
    match msg {
        SudoMsg::ClockEndBlock {} => {
            if env.block.height % 100 != 0 {
                return Ok(Response::new());
            }
            // Every 10 blocks this config value increases 1
            let config: Config = read_config(deps.storage)?;
            let staking_contract = deps.api.addr_humanize(&config.staking_contract)?;
            let staking_tokens: Vec<Addr> = read_staking_tokens(&deps.querier, staking_contract)?
                .into_iter()
                .map(|token| deps.api.addr_validate(&token))
                .collect::<StdResult<Vec<Addr>>>()?;
            distribute(deps, env, staking_tokens)
        }
    }
}
