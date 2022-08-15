use cosmwasm_std::{
    attr, from_binary, to_binary, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env, HandleResponse,
    HumanAddr, InitResponse, MessageInfo, StdError, StdResult,
};
use cw20::Cw20ReceiveMsg;

use crate::state::{read_config, read_token_ratio, store_config, store_token_ratio, Config};

use oraiswap::converter::{
    ConfigResponse, ConvertInfoResponse, Cw20HookMsg, HandleMsg, InitMsg, QueryMsg, TokenInfo,
    TokenRatio,
};

use oraiswap::asset::{Asset, AssetInfo};

pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, _msg: InitMsg) -> StdResult<InitResponse> {
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
        HandleMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        HandleMsg::UpdateConfig { owner } => update_config(deps, info, owner),
        HandleMsg::UpdatePair { from, to } => update_pair(deps, info, from, to),
        HandleMsg::Convert {} => convert(deps, env, info),
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

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> StdResult<HandleResponse> {
    match from_binary(&cw20_msg.msg.unwrap_or(Binary::default())) {
        Ok(Cw20HookMsg::Convert {}) => {
            // check permission
            let token_raw = deps.api.canonical_address(&info.sender)?;
            let token_ratio = read_token_ratio(deps.storage, token_raw.as_slice())?;
            let message = Asset {
                info: token_ratio.info,
                amount: cw20_msg.amount * token_ratio.ratio,
            }
            .into_msg(
                None,
                &deps.querier,
                env.contract.address.clone(),
                cw20_msg.sender,
            )?;

            Ok(HandleResponse {
                messages: vec![message],
                attributes: vec![attr("action", "convert_token")],
                data: None,
            })
        }
        Err(_) => Err(StdError::generic_err("invalid cw20 hook message")),
    }
}

pub fn update_pair(
    deps: DepsMut,
    info: MessageInfo,
    from: TokenInfo,
    to: TokenInfo,
) -> StdResult<HandleResponse> {
    let config: Config = read_config(deps.storage)?;
    if config.owner != deps.api.canonical_address(&info.sender)? {
        return Err(StdError::generic_err("unauthorized"));
    }

    let asset_key = from.info.to_vec(deps.api)?;

    let token_ratio = TokenRatio {
        info: to.info,
        ratio: Decimal::from_ratio(to.decimals, from.decimals),
    };

    store_token_ratio(deps.storage, &asset_key, &token_ratio)?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_pair")],
        data: None,
    })
}

pub fn convert(deps: DepsMut, env: Env, info: MessageInfo) -> StdResult<HandleResponse> {
    let mut messages: Vec<CosmosMsg> = vec![];

    for native_coin in info.sent_funds {
        let asset_key = native_coin.denom.as_bytes();
        let amount = native_coin.amount;
        let token_ratio = read_token_ratio(deps.storage, asset_key)?;

        let message = Asset {
            info: token_ratio.info,
            amount: amount * token_ratio.ratio,
        }
        .into_msg(
            None,
            &deps.querier,
            env.contract.address.clone(),
            info.sender.clone(),
        )?;

        messages.push(message);
    }

    Ok(HandleResponse {
        messages,
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
    let token_ratio = read_token_ratio(deps.storage, &asset_key)?;
    Ok(ConvertInfoResponse { token_ratio })
}
