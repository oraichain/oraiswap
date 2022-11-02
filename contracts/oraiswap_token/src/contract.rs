use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

use cw20::Cw20ExecuteMsg;
use cw20_base::ContractError;
use cw20_base::{
    contract::{
        execute as cw20_execute, instantiate as cw20_instantiate, migrate as cw20_migrate,
        query as cw20_query,
    },
    msg::{InstantiateMsg, MigrateMsg, QueryMsg},
};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    cw20_instantiate(deps, env, info, msg)
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw20ExecuteMsg,
) -> Result<Response, ContractError> {
    cw20_execute(deps, env, info, msg)
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    cw20_query(deps, env, msg)
}

#[entry_point]
pub fn migrate(deps: DepsMut, env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    cw20_migrate(deps, env, msg)
}

#[test]
pub fn test() {
    let contract = oraiswap::create_entry_points_testing!(crate);
    let mut app = oraiswap::mock_app::MockApp::new(&[]);
    let code_id = app.upload(contract);
    println!("contract code id {}", code_id);
}
