use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, HumanAddr, InitResponse,
    MessageInfo, StdError, StdResult,
};

use crate::state::{
    read_config, read_convert_info, store_config, store_convert_info, Config, ConvertInfo,
};

use oraiswap::converter::ConvertInfoResponse;

use oraiswap::converter::{ConfigResponse, HandleMsg, QueryMsg};

use oraiswap::asset::{Asset, AssetInfo};

pub fn init(deps: DepsMut, info: MessageInfo) -> StdResult<InitResponse> {
    store_config(
        deps.storage,
        &Config {
            owner: deps.api.canonical_address(&info.sender)?,
        },
    )?;

    Ok(InitResponse::default())
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::UpdateConfig { owner } => update_config(deps, info, owner),
        HandleMsg::UpdateConvertInfoMsg {
            from,
            to_token,
            from_to_ratio,
        } => update_convert_info(deps, info, from, to_token, from_to_ratio),
        HandleMsg::Convert { asset } => convert(deps, env, info, asset),
    }
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: HumanAddr,
) -> StdResult<HandleResponse> {
    let mut config: Config = read_config(deps.storage)?;
    if config.owner != deps.api.canonical_address(&info.sender)? {
        return Err(StdError::generic_err("unauthorized"));
    }

    config.owner = deps.api.canonical_address(&owner)?;

    store_config(deps.storage, &config)?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_config")],
        data: None,
    })
}

pub fn update_convert_info(
    deps: DepsMut,
    info: MessageInfo,
    from: AssetInfo,
    to_token: AssetInfo,
    from_to_ratio: u128,
) -> StdResult<HandleResponse> {
    let config: Config = read_config(deps.storage)?;
    if config.owner != deps.api.canonical_address(&info.sender)? {
        return Err(StdError::generic_err("unauthorized"));
    }

    let asset_key = &from.to_vec(deps.api)?;
    store_convert_info(
        deps.storage,
        asset_key,
        &ConvertInfo {
            to_token,
            from_to_ratio,
        },
    )?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_convert_info")],
        data: None,
    })
}

pub fn convert(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset: Asset,
) -> StdResult<HandleResponse> {
    asset.assert_sent_native_token_balance(&info)?;

    let asset_key = &asset.info.to_vec(deps.api)?;
    let convert_info = read_convert_info(deps.storage, asset_key)?;

    let message = (Asset {
        info: convert_info.to_token,
        amount: (asset.amount.u128() * convert_info.from_to_ratio).into(),
    })
    .into_msg(
        None,
        &deps.querier,
        env.contract.address.clone(),
        info.sender,
    )?;
    Ok(HandleResponse {
        messages: vec![message],
        attributes: vec![attr("action", "convert_token")],
        data: None,
    })
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::ConvertInfo { asset_info } => to_binary(&query_convert_info(deps, asset_info)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = read_config(deps.storage)?;
    let resp = ConfigResponse {
        owner: deps.api.human_address(&state.owner)?,
    };

    Ok(resp)
}

pub fn query_convert_info(deps: Deps, asset_info: AssetInfo) -> StdResult<ConvertInfoResponse> {
    let asset_key = asset_info.to_vec(deps.api)?;
    let convert_info = read_convert_info(deps.storage, &asset_key)?;
    Ok(ConvertInfoResponse {
        to_token: convert_info.to_token,
        from_to_ratio: convert_info.from_to_ratio,
    })
}
