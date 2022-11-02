use cosmwasm_std::{
    attr, to_binary, Addr, Binary, CanonicalAddr, Deps, DepsMut, Env, HandleResponse, InitResponse,
    MessageInfo, MigrateResponse, StdResult, WasmMsg,
};
use oraiswap::error::ContractError;
use oraiswap::hook::InitHook;

use crate::querier::query_liquidity_token;
use crate::state::{pair_key, read_pairs, Config, CONFIG, PAIRS};

use oraiswap::asset::{AssetInfo, PairInfo, PairInfoRaw};
use oraiswap::factory::{ConfigResponse, HandleMsg, InitMsg, MigrateMsg, PairsResponse, QueryMsg};
use oraiswap::pair::{InitMsg as PairInitMsg, DEFAULT_COMMISSION_RATE};

pub fn init(deps: DepsMut, _env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    let config = Config {
        oracle_addr: deps.api.canonical_address(&msg.oracle_addr)?,
        owner: deps.api.canonical_address(&info.sender)?,
        token_code_id: msg.token_code_id,
        pair_code_id: msg.pair_code_id,
        commission_rate: msg
            .commission_rate
            .unwrap_or(DEFAULT_COMMISSION_RATE.to_string()),
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(InitResponse::default())
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::UpdateConfig {
            owner,
            token_code_id,
            pair_code_id,
        } => handle_update_config(deps, env, info, owner, token_code_id, pair_code_id),
        HandleMsg::CreatePair {
            asset_infos,
            auto_register,
        } => handle_create_pair(deps, env, info, asset_infos, auto_register),
        HandleMsg::Register { asset_infos } => handle_register_pair(deps, env, info, asset_infos),
    }
}

// Only owner can execute it
pub fn handle_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    owner: Option<String>,
    token_code_id: Option<u64>,
    pair_code_id: Option<u64>,
) -> Result<HandleResponse, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    // permission check
    if deps.api.canonical_address(&info.sender)? != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(owner) = owner {
        config.owner = deps.api.canonical_address(&Addr(owner))?;
    }

    if let Some(token_code_id) = token_code_id {
        config.token_code_id = token_code_id;
    }

    if let Some(pair_code_id) = pair_code_id {
        config.pair_code_id = pair_code_id;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![attr("action", "update_config")],
        data: None,
    })
}

// Anyone can execute it to create swap pair
pub fn handle_create_pair(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    asset_infos: [AssetInfo; 2],
    auto_register: bool,
) -> Result<HandleResponse, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;
    let raw_infos = [
        asset_infos[0].to_raw(deps.api)?,
        asset_infos[1].to_raw(deps.api)?,
    ];

    let pair_key = pair_key(&raw_infos);

    // can not update pair once updated
    if let Ok(Some(_)) = PAIRS.may_load(deps.storage, &pair_key) {
        return Err(ContractError::PairExisted {});
    }

    PAIRS.save(
        deps.storage,
        &pair_key,
        &PairInfoRaw {
            oracle_addr: config.oracle_addr.clone(),
            liquidity_token: CanonicalAddr::default(),
            contract_addr: CanonicalAddr::default(),
            asset_infos: raw_infos,
            commission_rate: config.commission_rate.clone(),
        },
    )?;

    let init_hook = if auto_register {
        Some(InitHook {
            contract_addr: env.contract.address,
            msg: to_binary(&HandleMsg::Register {
                asset_infos: asset_infos.clone(),
            })?,
        })
    } else {
        None
    };

    Ok(HandleResponse {
        // instantiate pair with hook to call register after work
        messages: vec![WasmMsg::Instantiate {
            code_id: config.pair_code_id,
            send: vec![],
            label: None,
            msg: to_binary(&PairInitMsg {
                oracle_addr: deps.api.addr_humanize(&config.oracle_addr)?,
                asset_infos: asset_infos.clone(),
                token_code_id: config.token_code_id,
                commission_rate: Some(config.commission_rate),
                init_hook,
            })?,
        }
        .into()],
        attributes: vec![
            attr("action", "create_pair"),
            attr("pair", &format!("{}-{}", asset_infos[0], asset_infos[1])),
        ],
        data: None,
    })
}

/// This just stores the result for future query, update pair after success instantiate contract
/// call rpc get_address from code_id after calling this
pub fn handle_register_pair(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    asset_infos: [AssetInfo; 2],
) -> Result<HandleResponse, ContractError> {
    let raw_infos = [
        asset_infos[0].to_raw(deps.api)?,
        asset_infos[1].to_raw(deps.api)?,
    ];
    let pair_key = pair_key(&raw_infos);

    let mut pair_info = PAIRS.load(deps.storage, &pair_key)?;

    // make sure creator can update their pairs
    if pair_info.contract_addr != CanonicalAddr::default() {
        return Err(ContractError::PairRegistered {});
    }

    // the contract must follow the standard interface
    pair_info.liquidity_token = query_liquidity_token(deps.querier, info.sender.clone())?;
    pair_info.contract_addr = deps.api.canonical_address(&info.sender)?;

    PAIRS.save(deps.storage, &pair_key, &pair_info)?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("pair_contract_address", info.sender.to_string()),
            attr(
                "liquidity_token_address",
                deps.api.addr_humanize(&pair_info.liquidity_token)?.as_str(),
            ),
        ],
        data: None,
    })
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Pair { asset_infos } => to_binary(&query_pair(deps, asset_infos)?),
        QueryMsg::Pairs { start_after, limit } => {
            to_binary(&query_pairs(deps, start_after, limit)?)
        }
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state: Config = CONFIG.load(deps.storage)?;
    let resp = ConfigResponse {
        oracle_addr: deps.api.addr_humanize(&state.oracle_addr)?,
        owner: deps.api.addr_humanize(&state.owner)?,
        token_code_id: state.token_code_id,
        pair_code_id: state.pair_code_id,
    };

    Ok(resp)
}

pub fn query_pair(deps: Deps, asset_infos: [AssetInfo; 2]) -> StdResult<PairInfo> {
    let pair_key = pair_key(&[
        asset_infos[0].to_raw(deps.api)?,
        asset_infos[1].to_raw(deps.api)?,
    ]);
    let pair_info: PairInfoRaw = PAIRS.load(deps.storage, &pair_key)?;
    pair_info.to_normal(deps.api)
}

pub fn query_pairs(
    deps: Deps,
    start_after: Option<[AssetInfo; 2]>,
    limit: Option<u32>,
) -> StdResult<PairsResponse> {
    let start_after = if let Some(start_after) = start_after {
        Some([
            start_after[0].to_raw(deps.api)?,
            start_after[1].to_raw(deps.api)?,
        ])
    } else {
        None
    };

    let pairs: Vec<PairInfo> = read_pairs(deps.storage, deps.api, start_after, limit)?;
    let resp = PairsResponse { pairs };

    Ok(resp)
}

pub fn migrate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: MigrateMsg,
) -> StdResult<MigrateResponse> {
    Ok(MigrateResponse::default())
}
