#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, IbcMsg, IbcQuery,
    MessageInfo, Order, PortIdResponse, Response, StdResult, WasmMsg,
};

use cw2::set_contract_version;
use cw20::{Cw20Coin, Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw_storage_plus::Bound;

use crate::amount::Amount;
use crate::error::ContractError;
use crate::msg::{
    AllowMsg, AllowedInfo, AllowedResponse, AllowedTokenInfo, AllowedTokenResponse,
    ChannelResponse, ClaimTokensMsg, ConfigResponse, CreateLockupMsg, ExecuteMsg, ExitPoolMsg,
    ExternalTokenMsg, InstantiateMsg, JoinPoolMsg, ListAllowedResponse, ListChannelsResponse,
    ListExternalTokensResponse, LockTokensMsg, LockupResponse, QueryMsg, SwapMsg, TransferMsg,
    UnlockTokensMsg,
};
use crate::state::{
    find_external_token, increase_channel_balance, join_ibc_paths, AllowInfo, Config,
    ExternalTokenInfo, ADMIN, ALLOW_LIST, CHANNEL_INFO, CHANNEL_STATE, CONFIG, EXTERNAL_TOKENS,
    LOCKUP,
};
use cw_utils::{maybe_addr, nonpayable, one_coin};
use oraiswap::ibc::{
    ClaimPacket, ExitPoolPacket, Ics20Packet, JoinPoolPacket, LockPacket, OsmoPacket,
    SwapAmountInRoute, SwapPacket, UnlockPacket,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:ics20-swap-client";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let cfg = Config {
        default_timeout: msg.default_timeout,
        init_channel: false,
        default_remote_denom: None,
    };
    CONFIG.save(deps.storage, &cfg)?;

    let admin = deps.api.addr_validate(&msg.gov_contract)?;
    ADMIN.set(deps.branch(), Some(admin))?;

    // add all allows
    for allowed in msg.allowlist {
        let contract = deps.api.addr_validate(&allowed.contract)?;
        let info = AllowInfo {
            gas_limit: allowed.gas_limit,
        };
        ALLOW_LIST.save(deps.storage, &contract, &info)?;
    }

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
        ExecuteMsg::Transfer(msg) => {
            let coin = one_coin(&info)?;
            execute_transfer(deps, env, msg, Amount::Native(coin), info.sender)
        }
        ExecuteMsg::Swap(msg) => {
            let coin = one_coin(&info)?;
            execute_swap(deps, env, msg, Amount::Native(coin), info.sender)
        }
        ExecuteMsg::JoinPool(pool) => {
            let coin = one_coin(&info)?;
            execute_join_pool(deps, env, pool, Amount::Native(coin), info.sender)
        }
        ExecuteMsg::ExitPool(pool) => {
            let coin = one_coin(&info)?;
            execute_exit_pool(deps, env, pool, Amount::Native(coin), info.sender)
        }
        ExecuteMsg::CreateLockup(msg) => {
            nonpayable(&info)?;
            execute_create_lockup(deps, env, msg, info.sender)
        }
        ExecuteMsg::LockTokens(msg) => {
            let coin = one_coin(&info)?;
            execute_lock_tokens(deps, env, msg, Amount::Native(coin), info.sender)
        }
        ExecuteMsg::ClaimTokens(msg) => {
            nonpayable(&info)?;
            execute_claim_tokens(deps, env, msg, info.sender)
        }
        ExecuteMsg::UnlockTokens(msg) => {
            nonpayable(&info)?;
            execute_unlock_tokens(deps, env, msg, info.sender)
        }
        ExecuteMsg::Allow(allow) => execute_allow(deps, env, info, allow),
        ExecuteMsg::AllowExternalToken(token) => allow_external_token(deps, env, info, token),
        ExecuteMsg::UpdateAdmin { admin } => {
            let admin = deps.api.addr_validate(&admin)?;
            Ok(ADMIN.execute_update_admin(deps, info, Some(admin))?)
        }
    }
}

pub fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let msg: ExecuteMsg = from_binary(&wrapper.msg)?;
    let amount = Amount::Cw20(Cw20Coin {
        address: info.sender.to_string(),
        amount: wrapper.amount,
    });
    let api = deps.api;

    match msg {
        ExecuteMsg::Transfer(transfer) => execute_transfer(
            deps,
            env,
            transfer,
            amount,
            api.addr_validate(&wrapper.sender)?,
        ),
        ExecuteMsg::Swap(swap) => {
            execute_swap(deps, env, swap, amount, api.addr_validate(&wrapper.sender)?)
        }
        ExecuteMsg::JoinPool(pool) => {
            execute_join_pool(deps, env, pool, amount, api.addr_validate(&wrapper.sender)?)
        }
        ExecuteMsg::ExitPool(pool) => {
            execute_exit_pool(deps, env, pool, amount, api.addr_validate(&wrapper.sender)?)
        }
        ExecuteMsg::LockTokens(msg) => {
            execute_lock_tokens(deps, env, msg, amount, api.addr_validate(&wrapper.sender)?)
        }
        _ => Err(ContractError::UnknownRequest {}),
    }
}

pub fn execute_transfer(
    deps: DepsMut,
    env: Env,
    msg: TransferMsg,
    amount: Amount,
    sender: Addr,
) -> Result<Response, ContractError> {
    execute_transfer_with_action(deps, env, msg, amount, sender, None, "transfer")
}

pub fn execute_transfer_with_action(
    deps: DepsMut,
    env: Env,
    msg: TransferMsg,
    amount: Amount,
    sender: Addr,
    action: Option<OsmoPacket>,
    action_label: &str,
) -> Result<Response, ContractError> {
    if amount.is_empty() {
        return Err(ContractError::NoFunds {});
    }

    // ensure the requested channel is registered
    if !CHANNEL_INFO.has(deps.storage, &msg.channel) {
        return Err(ContractError::NoSuchChannel { id: msg.channel });
    }

    // if cw20 token, ensure it is whitelisted
    let mut denom = amount.denom();
    let mut our_chain = true;
    if let Amount::Cw20(coin) = &amount {
        let addr = deps.api.addr_validate(&coin.address)?;
        ALLOW_LIST
            .may_load(deps.storage, &addr)?
            .ok_or(ContractError::NotOnAllowList)?;

        let token = find_external_token(deps.storage, coin.clone().address)?;
        if let Some(ext_denom) = token {
            denom = get_ibc_full_denom(deps.as_ref(), msg.channel.as_str(), ext_denom.as_str())?;
            our_chain = false;
        }
    };

    // delta from user is in seconds
    let timeout_delta = match msg.timeout {
        Some(t) => t,
        None => CONFIG.load(deps.storage)?.default_timeout,
    };
    // timeout is in nanoseconds
    let timeout = env.block.time.plus_seconds(timeout_delta);

    // build ics20 packet
    let packet = Ics20Packet::new(
        amount.amount(),
        denom,
        sender.as_ref(),
        &msg.remote_address,
        action,
    );

    if our_chain {
        increase_channel_balance(deps.storage, &msg.channel, &amount.denom(), amount.amount())?;
    }

    // prepare ibc message
    let msg = IbcMsg::SendPacket {
        channel_id: msg.channel,
        data: to_binary(&packet)?,
        timeout: timeout.into(),
    }
    .into();
    let mut msgs: Vec<CosmosMsg> = vec![msg];

    let burn = safe_burn(amount, our_chain);
    if let Some(msg) = burn {
        msgs.push(msg);
    }

    let mut attributes = vec![
        attr("action", action_label),
        attr("sender", &packet.sender),
        attr("denom", &packet.denom),
        attr("amount", &packet.amount.to_string()),
    ];
    if !packet.receiver.is_empty() {
        attributes.push(attr("receiver", &packet.receiver));
    }

    // send response
    let res = Response::new()
        .add_messages(msgs)
        .add_attributes(attributes);

    Ok(res)
}

pub fn execute_only_action(
    deps: DepsMut,
    env: Env,
    msg: TransferMsg,
    sender: Addr,
    action: OsmoPacket,
    action_label: &str,
) -> Result<Response, ContractError> {
    // ensure the requested channel is registered
    if !CHANNEL_INFO.has(deps.storage, &msg.channel) {
        return Err(ContractError::NoSuchChannel { id: msg.channel });
    }

    let config = CONFIG.load(deps.storage)?;
    if config.default_remote_denom.is_none() {
        return Err(ContractError::NoSuchChannel { id: msg.channel });
    }

    // delta from user is in seconds
    let timeout_delta = match msg.timeout {
        Some(t) => t,
        None => config.default_timeout,
    };
    // timeout is in nanoseconds
    let timeout = env.block.time.plus_seconds(timeout_delta);

    let denom = get_ibc_full_denom(
        deps.as_ref(),
        msg.channel.as_str(),
        config.default_remote_denom.unwrap().as_str(),
    )?;

    // build ics20 packet
    let packet = Ics20Packet::new(
        0u8.into(),
        denom,
        sender.as_ref(),
        &msg.remote_address,
        Some(action),
    );

    // prepare ibc message
    let msg: CosmosMsg = IbcMsg::SendPacket {
        channel_id: msg.channel,
        data: to_binary(&packet)?,
        timeout: timeout.into(),
    }
    .into();

    // send response
    let res = Response::new()
        .add_message(msg)
        .add_attribute("action", action_label)
        .add_attribute("sender", &packet.sender);

    Ok(res)
}

fn get_ibc_full_denom(deps: Deps, channel: &str, denom: &str) -> StdResult<String> {
    let query = IbcQuery::PortId {}.into();
    let PortIdResponse { port_id } = deps.querier.query(&query)?;

    let ibc_prefix = join_ibc_paths(port_id.as_str(), channel);

    Ok(join_ibc_paths(ibc_prefix.as_str(), denom))
}

pub fn execute_swap(
    deps: DepsMut,
    env: Env,
    msg: SwapMsg,
    amount: Amount,
    sender: Addr,
) -> Result<Response, ContractError> {
    let swap_packet = SwapPacket {
        routes: vec![SwapAmountInRoute {
            pool_id: msg.pool,
            token_out_denom: msg.token_out,
        }],
        token_out_min_amount: msg.min_amount_out,
    };
    let transfer_msg = TransferMsg {
        channel: msg.channel,
        remote_address: String::new(),
        timeout: msg.timeout,
    };

    execute_transfer_with_action(
        deps,
        env,
        transfer_msg,
        amount,
        sender,
        Some(OsmoPacket::Swap(swap_packet)),
        "swap",
    )
}

pub fn execute_join_pool(
    deps: DepsMut,
    env: Env,
    msg: JoinPoolMsg,
    amount: Amount,
    sender: Addr,
) -> Result<Response, ContractError> {
    let gamm_packet = JoinPoolPacket {
        pool_id: msg.pool,
        share_out_min_amount: msg.share_min_out,
    };
    let transfer_msg = TransferMsg {
        channel: msg.channel,
        remote_address: String::new(),
        timeout: msg.timeout,
    };

    execute_transfer_with_action(
        deps,
        env,
        transfer_msg,
        amount,
        sender,
        Some(OsmoPacket::JoinPool(gamm_packet)),
        "join_pool",
    )
}

pub fn execute_exit_pool(
    deps: DepsMut,
    env: Env,
    msg: ExitPoolMsg,
    amount: Amount,
    sender: Addr,
) -> Result<Response, ContractError> {
    let gamm_packet = ExitPoolPacket {
        token_out_denom: msg.token_out,
        token_out_min_amount: msg.min_amount_out,
    };
    let transfer_msg = TransferMsg {
        channel: msg.channel,
        remote_address: String::new(),
        timeout: msg.timeout,
    };

    execute_transfer_with_action(
        deps,
        env,
        transfer_msg,
        amount,
        sender,
        Some(OsmoPacket::ExitPool(gamm_packet)),
        "exit_pool",
    )
}

pub fn execute_create_lockup(
    deps: DepsMut,
    env: Env,
    msg: CreateLockupMsg,
    sender: Addr,
) -> Result<Response, ContractError> {
    let lockup_key = (msg.channel.as_str(), sender.as_str());
    if LOCKUP.has(deps.storage, lockup_key) {
        return Err(ContractError::LockupAccountFound {});
    }

    let gamm_packet = OsmoPacket::LockupAccount {};
    let transfer_msg = TransferMsg {
        channel: msg.channel,
        remote_address: String::new(),
        timeout: msg.timeout,
    };

    execute_only_action(
        deps,
        env,
        transfer_msg,
        sender,
        gamm_packet,
        "create_lockup",
    )
}

pub fn execute_lock_tokens(
    deps: DepsMut,
    env: Env,
    msg: LockTokensMsg,
    amount: Amount,
    sender: Addr,
) -> Result<Response, ContractError> {
    assert_lockup_owner(deps.as_ref(), msg.channel.as_str(), sender.as_str())?;

    let gamm_packet = OsmoPacket::Lock(LockPacket {
        duration: msg.duration,
    });
    let transfer_msg = TransferMsg {
        channel: msg.channel,
        remote_address: String::new(),
        timeout: msg.timeout,
    };

    execute_transfer_with_action(
        deps,
        env,
        transfer_msg,
        amount,
        sender,
        Some(gamm_packet),
        "lock_tokens",
    )
}

pub fn execute_claim_tokens(
    deps: DepsMut,
    env: Env,
    msg: ClaimTokensMsg,
    sender: Addr,
) -> Result<Response, ContractError> {
    assert_lockup_owner(deps.as_ref(), msg.channel.as_str(), sender.as_str())?;

    let gamm_packet = OsmoPacket::Claim(ClaimPacket { denom: msg.denom });
    let transfer_msg = TransferMsg {
        channel: msg.channel,
        remote_address: String::new(),
        timeout: msg.timeout,
    };

    execute_only_action(deps, env, transfer_msg, sender, gamm_packet, "claim_tokens")
}

pub fn execute_unlock_tokens(
    deps: DepsMut,
    env: Env,
    msg: UnlockTokensMsg,
    sender: Addr,
) -> Result<Response, ContractError> {
    assert_lockup_owner(deps.as_ref(), msg.channel.as_str(), sender.as_str())?;

    let gamm_packet = OsmoPacket::Unlock(UnlockPacket { id: msg.lock_id });
    let transfer_msg = TransferMsg {
        channel: msg.channel,
        remote_address: String::new(),
        timeout: msg.timeout,
    };

    execute_only_action(
        deps,
        env,
        transfer_msg,
        sender,
        gamm_packet,
        "begin_unlock_tokens",
    )
}

fn assert_lockup_owner(deps: Deps, channel: &str, owner: &str) -> Result<(), ContractError> {
    let lockup_key = (channel, owner);
    if !LOCKUP.has(deps.storage, lockup_key) {
        return Err(ContractError::NoLockupAccount {});
    }

    Ok(())
}

fn safe_burn(amount: Amount, our_chain: bool) -> Option<CosmosMsg> {
    match amount {
        Amount::Native(_) => None,
        Amount::Cw20(coin) => {
            if our_chain {
                return None;
            }

            let msg = Cw20ExecuteMsg::Burn {
                amount: coin.amount,
            };

            Some(
                WasmMsg::Execute {
                    contract_addr: coin.address,
                    msg: to_binary(&msg).unwrap(),
                    funds: vec![],
                }
                .into(),
            )
        }
    }
}

/// The gov contract can allow new contracts, or increase the gas limit on existing contracts.
/// It cannot block or reduce the limit to avoid forcible sticking tokens in the channel.
pub fn execute_allow(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    allow: AllowMsg,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    add_allow_token(deps, allow.clone())?;
    let gas = if let Some(gas) = allow.gas_limit {
        gas.to_string()
    } else {
        "None".to_string()
    };

    let res = Response::new()
        .add_attribute("action", "allow")
        .add_attribute("contract", allow.contract)
        .add_attribute("gas_limit", gas);
    Ok(res)
}

fn add_allow_token(deps: DepsMut, allow: AllowMsg) -> Result<(), ContractError> {
    let contract = deps.api.addr_validate(&allow.contract)?;
    let set = AllowInfo {
        gas_limit: allow.gas_limit,
    };
    ALLOW_LIST.update(deps.storage, &contract, |old| {
        if let Some(old) = old {
            // we must ensure it increases the limit
            match (old.gas_limit, set.gas_limit) {
                (None, Some(_)) => return Err(ContractError::CannotLowerGas),
                (Some(old), Some(new)) if new < old => return Err(ContractError::CannotLowerGas),
                _ => {}
            };
        }
        Ok(AllowInfo {
            gas_limit: allow.gas_limit,
        })
    })?;

    Ok(())
}

pub fn allow_external_token(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    allow: ExternalTokenMsg,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;
    if EXTERNAL_TOKENS.has(deps.storage, &allow.denom) {
        return Err(ContractError::ExternalTokenExists {});
    }

    let contract = deps.api.addr_validate(&allow.contract)?;
    let set = ExternalTokenInfo { contract };

    EXTERNAL_TOKENS.save(deps.storage, &allow.denom, &set)?;
    let set_allow = AllowMsg {
        contract: allow.contract.to_owned(),
        gas_limit: None,
    };

    // Save denom for only action.
    let mut cfg = CONFIG.load(deps.storage)?;
    if cfg.default_remote_denom.is_none() {
        cfg.default_remote_denom = Some(allow.denom.to_owned());
        CONFIG.save(deps.storage, &cfg)?;
    }
    add_allow_token(deps, set_allow)?;

    let res = Response::new()
        .add_attribute("action", "allow_external_token")
        .add_attribute("denom", allow.denom)
        .add_attribute("contract", allow.contract);
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ListChannels {} => to_binary(&query_list(deps)?),
        QueryMsg::Channel { id } => to_binary(&query_channel(deps, id)?),
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Allowed { contract } => to_binary(&query_allowed(deps, contract)?),
        QueryMsg::ExternalToken { denom } => to_binary(&query_external_token(deps, denom)?),
        QueryMsg::ListAllowed { start_after, limit } => {
            to_binary(&list_allowed(deps, start_after, limit)?)
        }
        QueryMsg::ListExternalTokens { start_after, limit } => {
            to_binary(&list_external_tokens(deps, start_after, limit)?)
        }
        QueryMsg::Admin {} => to_binary(&ADMIN.query_admin(deps)?),
        QueryMsg::Lockup { channel, owner } => to_binary(&query_lockup(deps, channel, owner)?),
    }
}

fn query_list(deps: Deps) -> StdResult<ListChannelsResponse> {
    let channels = CHANNEL_INFO
        .range_raw(deps.storage, None, None, Order::Ascending)
        .map(|r| r.map(|(_, v)| v))
        .collect::<StdResult<_>>()?;
    Ok(ListChannelsResponse { channels })
}

// make public for ibc tests
pub fn query_channel(deps: Deps, id: String) -> StdResult<ChannelResponse> {
    let info = CHANNEL_INFO.load(deps.storage, &id)?;
    // this returns Vec<(outstanding, total)>
    let state = CHANNEL_STATE
        .prefix(&id)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|r| {
            r.map(|(denom, v)| {
                let outstanding = Amount::from_parts(denom.clone(), v.outstanding);
                let total = Amount::from_parts(denom, v.total_sent);
                (outstanding, total)
            })
        })
        .collect::<StdResult<Vec<_>>>()?;
    // we want (Vec<outstanding>, Vec<total>)
    let (balances, total_sent) = state.into_iter().unzip();

    Ok(ChannelResponse {
        info,
        balances,
        total_sent,
    })
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    let admin = ADMIN.get(deps)?.unwrap_or_else(|| Addr::unchecked(""));
    let res = ConfigResponse {
        default_timeout: cfg.default_timeout,
        gov_contract: admin.into(),
    };
    Ok(res)
}

fn query_allowed(deps: Deps, contract: String) -> StdResult<AllowedResponse> {
    let addr = deps.api.addr_validate(&contract)?;
    let info = ALLOW_LIST.may_load(deps.storage, &addr)?;
    let res = match info {
        None => AllowedResponse {
            is_allowed: false,
            gas_limit: None,
        },
        Some(a) => AllowedResponse {
            is_allowed: true,
            gas_limit: a.gas_limit,
        },
    };
    Ok(res)
}

fn query_external_token(deps: Deps, denom: String) -> StdResult<AllowedTokenResponse> {
    let info = EXTERNAL_TOKENS.may_load(deps.storage, denom.as_str())?;
    let res = match info {
        None => AllowedTokenResponse {
            is_allowed: false,
            contract: None,
        },
        Some(a) => AllowedTokenResponse {
            is_allowed: true,
            contract: Some(a.contract.to_string()),
        },
    };
    Ok(res)
}

fn query_lockup(deps: Deps, channel_id: String, owner: String) -> StdResult<LockupResponse> {
    let lockup_key = (channel_id.as_str(), owner.as_str());
    let lockup_address = LOCKUP.load(deps.storage, lockup_key).unwrap_or_default();
    let res = LockupResponse {
        owner,
        address: lockup_address,
    };
    Ok(res)
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

fn list_allowed(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<ListAllowedResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let addr = maybe_addr(deps.api, start_after)?;
    let start = addr.as_ref().map(Bound::exclusive);

    let allow = ALLOW_LIST
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            item.map(|(addr, allow)| AllowedInfo {
                contract: addr.into(),
                gas_limit: allow.gas_limit,
            })
        })
        .collect::<StdResult<_>>()?;
    Ok(ListAllowedResponse { allow })
}

fn list_external_tokens(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<ListExternalTokensResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into()));

    let tokens = EXTERNAL_TOKENS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            item.map(|(denom, allow)| AllowedTokenInfo {
                denom,
                contract: allow.contract.into(),
            })
        })
        .collect::<StdResult<_>>()?;
    Ok(ListExternalTokensResponse { tokens })
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_helpers::*;

    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{coin, coins, CosmosMsg, IbcMsg, StdError, Uint128};

    use cw_utils::PaymentError;

    #[test]
    fn setup_and_query() {
        let deps = setup(&["channel-3"], &[]);

        let raw_list = query(deps.as_ref(), mock_env(), QueryMsg::ListChannels {}).unwrap();
        let list_res: ListChannelsResponse = from_binary(&raw_list).unwrap();
        assert_eq!(1, list_res.channels.len());
        assert_eq!(mock_channel_info("channel-3"), list_res.channels[0]);
        // assert_eq!(mock_channel_info("channel-7"), list_res.channels[1]);

        let raw_channel = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Channel {
                id: "channel-3".to_string(),
            },
        )
        .unwrap();
        let chan_res: ChannelResponse = from_binary(&raw_channel).unwrap();
        assert_eq!(chan_res.info, mock_channel_info("channel-3"));
        assert_eq!(0, chan_res.total_sent.len());
        assert_eq!(0, chan_res.balances.len());

        let err = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Channel {
                id: "channel-10".to_string(),
            },
        )
        .unwrap_err();
        assert_eq!(err, StdError::not_found("oraiswap_ibc::state::ChannelInfo"));
    }

    #[test]
    fn proper_checks_on_execute_native() {
        let send_channel = "channel-5";
        let mut deps = setup(&[send_channel], &[]);

        let mut transfer = TransferMsg {
            channel: send_channel.to_string(),
            remote_address: "foreign-address".to_string(),
            timeout: None,
        };

        // works with proper funds
        let msg = ExecuteMsg::Transfer(transfer.clone());
        let info = mock_info("foobar", &coins(1234567, "ucosm"));
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(1, res.messages.len());
        if let CosmosMsg::Ibc(IbcMsg::SendPacket {
            channel_id,
            data,
            timeout,
        }) = &res.messages[0].msg
        {
            let expected_timeout = mock_env().block.time.plus_seconds(DEFAULT_TIMEOUT);
            assert_eq!(timeout, &expected_timeout.into());
            assert_eq!(channel_id.as_str(), send_channel);
            let msg: Ics20Packet = from_binary(data).unwrap();

            assert_eq!(msg.amount, Uint128::new(1234567));
            assert_eq!(msg.denom.as_str(), "ucosm");
            assert_eq!(msg.sender.as_str(), "foobar");
            assert_eq!(msg.receiver.as_str(), "foreign-address");
        } else {
            panic!("Unexpected return message: {:?}", res.messages[0]);
        }

        // reject with no funds
        let msg = ExecuteMsg::Transfer(transfer.clone());
        let info = mock_info("foobar", &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::Payment(PaymentError::NoFunds {}));

        // reject with multiple tokens funds
        let msg = ExecuteMsg::Transfer(transfer.clone());
        let info = mock_info("foobar", &[coin(1234567, "ucosm"), coin(54321, "uatom")]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::Payment(PaymentError::MultipleDenoms {}));

        // reject with bad channel id
        transfer.channel = "channel-45".to_string();
        let msg = ExecuteMsg::Transfer(transfer);
        let info = mock_info("foobar", &coins(1234567, "ucosm"));
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(
            err,
            ContractError::NoSuchChannel {
                id: "channel-45".to_string()
            }
        );
    }

    #[test]
    fn proper_checks_on_execute_cw20() {
        let send_channel = "channel-15";
        let cw20_addr = "my-token";
        let mut deps = setup(&[send_channel], &[(cw20_addr, 123456)]);

        let transfer = ExecuteMsg::Transfer(TransferMsg {
            channel: send_channel.to_string(),
            remote_address: "foreign-address".to_string(),
            timeout: Some(7777),
        });
        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "my-account".into(),
            amount: Uint128::new(888777666),
            msg: to_binary(&transfer).unwrap(),
        });

        // works with proper funds
        let info = mock_info(cw20_addr, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
        assert_eq!(1, res.messages.len());
        assert_eq!(res.messages[0].gas_limit, None);
        if let CosmosMsg::Ibc(IbcMsg::SendPacket {
            channel_id,
            data,
            timeout,
        }) = &res.messages[0].msg
        {
            let expected_timeout = mock_env().block.time.plus_seconds(7777);
            assert_eq!(timeout, &expected_timeout.into());
            assert_eq!(channel_id.as_str(), send_channel);
            let msg: Ics20Packet = from_binary(data).unwrap();
            assert_eq!(msg.amount, Uint128::new(888777666));
            assert_eq!(msg.denom, format!("cw20:{}", cw20_addr));
            assert_eq!(msg.sender.as_str(), "my-account");
            assert_eq!(msg.receiver.as_str(), "foreign-address");
        } else {
            panic!("Unexpected return message: {:?}", res.messages[0]);
        }

        // reject with tokens funds
        let info = mock_info("foobar", &coins(1234567, "ucosm"));
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::Payment(PaymentError::NonPayable {}));
    }

    #[test]
    fn execute_cw20_fails_if_not_whitelisted() {
        let send_channel = "channel-15";
        let mut deps = setup(&[send_channel], &[]);

        let cw20_addr = "my-token";
        let transfer = ExecuteMsg::Transfer(TransferMsg {
            channel: send_channel.to_string(),
            remote_address: "foreign-address".to_string(),
            timeout: Some(7777),
        });
        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "my-account".into(),
            amount: Uint128::new(888777666),
            msg: to_binary(&transfer).unwrap(),
        });

        // works with proper funds
        let info = mock_info(cw20_addr, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::NotOnAllowList);
    }
}
