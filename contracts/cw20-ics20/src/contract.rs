#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, Deps, DepsMut, Env, IbcMsg, IbcQuery, MessageInfo, Order,
    PortIdResponse, Response, StdError, StdResult,
};
use cw2::set_contract_version;
use cw20::{Cw20Coin, Cw20ReceiveMsg};
use cw_storage_plus::Bound;

use crate::error::ContractError;
use crate::ibc::{parse_ibc_wasm_port_id, Ics20Packet};
use crate::msg::{
    AllowMsg, AllowedInfo, AllowedResponse, ChannelResponse, ConfigResponse, Cw20PairMsg,
    Cw20PairQuery, ExecuteMsg, InitMsg, ListAllowedResponse, ListChannelsResponse,
    ListCw20MappingResponse, MigrateMsg, PortResponse, QueryMsg, TransferBackMsg, TransferMsg,
};
use crate::state::{
    cw20_ics20_denoms, get_key_ics20_ibc_denom, increase_channel_balance, reduce_channel_balance,
    AllowInfo, Config, Cw20MappingMetadata, ADMIN, ALLOW_LIST, CHANNEL_INFO, CHANNEL_STATE, CONFIG,
};
use cw20_ics20_msg::amount::Amount;
use cw_utils::{maybe_addr, nonpayable, one_coin};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw20-ics20";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InitMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let cfg = Config {
        default_timeout: msg.default_timeout,
        default_gas_limit: msg.default_gas_limit,
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

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        // TODO: add update cw20 pair
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
        ExecuteMsg::Transfer(msg) => {
            let coin = one_coin(&info)?;
            execute_transfer(deps, env, msg, Amount::Native(coin), info.sender)
        }
        ExecuteMsg::UpdateCw20MappingPair(msg) => {
            execute_update_cw20_mapping_pair(deps, env, info, msg)
        }
        ExecuteMsg::Allow(allow) => execute_allow(deps, env, info, allow),
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

    let amount = Amount::Cw20(Cw20Coin {
        address: info.sender.to_string(),
        amount: wrapper.amount,
    });
    let api = deps.api;

    let msg_result: StdResult<TransferMsg> = from_binary(&wrapper.msg);
    if msg_result.is_ok() {
        let msg: TransferMsg = msg_result.unwrap();
        return execute_transfer(deps, env, msg, amount, api.addr_validate(&wrapper.sender)?);
    }

    let msg: TransferBackMsg = from_binary(&wrapper.msg)?;
    execute_transfer_back_to_remote_chain(
        deps,
        env,
        msg,
        amount,
        api.addr_validate(&wrapper.sender)?,
    )
}

pub fn execute_transfer(
    deps: DepsMut,
    env: Env,
    msg: TransferMsg,
    amount: Amount,
    sender: Addr,
) -> Result<Response, ContractError> {
    if amount.is_empty() {
        return Err(ContractError::NoFunds {});
    }
    // ensure the requested channel is registered
    if !CHANNEL_INFO.has(deps.storage, &msg.channel) {
        return Err(ContractError::NoSuchChannel { id: msg.channel });
    }
    let config = CONFIG.load(deps.storage)?;

    // if cw20 token, validate and ensure it is whitelisted, or we set default gas limit
    if let Amount::Cw20(coin) = &amount {
        let addr = deps.api.addr_validate(&coin.address)?;
        // if limit is set, then we always allow cw20
        if config.default_gas_limit.is_none() {
            ALLOW_LIST
                .may_load(deps.storage, &addr)?
                .ok_or(ContractError::NotOnAllowList)?;
        }
    };

    // delta from user is in seconds
    let timeout_delta = match msg.timeout {
        Some(t) => t,
        None => config.default_timeout,
    };
    // timeout is in nanoseconds
    let timeout = env.block.time.plus_seconds(timeout_delta);

    // build ics20 packet
    let packet = Ics20Packet::new(
        amount.amount(),
        amount.denom(),
        sender.as_ref(),
        &msg.remote_address,
        msg.memo,
    );
    packet.validate()?;

    // Update the balance now (optimistically) like ibctransfer modules.
    // In on_packet_failure (ack with error message or a timeout), we reduce the balance appropriately.
    // This means the channel works fine if success acks are not relayed.
    increase_channel_balance(deps.storage, &msg.channel, &amount.denom(), amount.amount())?;

    // prepare ibc message
    let msg = IbcMsg::SendPacket {
        channel_id: msg.channel,
        data: to_binary(&packet)?,
        timeout: timeout.into(),
    };

    // send response
    let res = Response::new()
        .add_message(msg)
        .add_attribute("action", "transfer")
        .add_attribute("sender", &packet.sender)
        .add_attribute("receiver", &packet.receiver)
        .add_attribute("denom", &packet.denom)
        .add_attribute("amount", &packet.amount.to_string());
    Ok(res)
}

pub fn execute_transfer_back_to_remote_chain(
    deps: DepsMut,
    env: Env,
    msg: TransferBackMsg,
    amount: Amount,
    sender: Addr,
) -> Result<Response, ContractError> {
    if amount.is_empty() {
        return Err(ContractError::NoFunds {});
    }

    // should be in form port/channel/denom
    let cw20_mapping = get_cw20_mapping_from_cw20_denom(deps.as_ref(), amount.denom())?;
    let ibc_denom = cw20_mapping.key;

    // ensure the requested channel is registered
    if !CHANNEL_INFO.has(deps.storage, &msg.local_channel_id) {
        return Err(ContractError::NoSuchChannel {
            id: msg.local_channel_id,
        });
    }
    let config = CONFIG.load(deps.storage)?;

    // delta from user is in seconds
    let timeout_delta = match msg.timeout {
        Some(t) => t,
        None => config.default_timeout,
    };
    // timeout is in nanoseconds
    let timeout = env.block.time.plus_seconds(timeout_delta);
    // need to convert decimal of cw20 to remote decimal before transferring
    let amount_remote = amount.convert_cw20_to_remote(
        cw20_mapping.cw20_map.remote_decimals,
        cw20_mapping.cw20_map.cw20_decimals,
    )?;

    // build ics20 packet
    let packet = Ics20Packet::new(
        amount_remote.clone(),
        ibc_denom.clone(),
        sender.as_str(),
        &msg.remote_address,
        msg.memo,
    );
    packet.validate()?;

    // because we are transferring back, we reduce the channel's balance
    reduce_channel_balance(
        deps.storage,
        &msg.local_channel_id,
        &ibc_denom,
        amount_remote,
    )?;

    // prepare ibc message
    let msg = IbcMsg::SendPacket {
        channel_id: msg.local_channel_id,
        data: to_binary(&packet)?,
        timeout: timeout.into(),
    };

    // send response
    let res = Response::new()
        .add_message(msg)
        .add_attribute("action", "transfer")
        .add_attribute("type", "transfer_back_to_remote_chain")
        .add_attribute("sender", &packet.sender)
        .add_attribute("receiver", &packet.receiver)
        .add_attribute("denom", &packet.denom)
        .add_attribute("amount", &packet.amount.to_string());
    Ok(res)
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

/// The gov contract can allow new contracts, or increase the gas limit on existing contracts.
/// It cannot block or reduce the limit to avoid forcible sticking tokens in the channel.
pub fn execute_update_cw20_mapping_pair(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mapping_pair_msg: Cw20PairMsg,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    let ibc_denom = get_key_ics20_ibc_denom(
        &parse_ibc_wasm_port_id(env.contract.address.into_string()),
        &mapping_pair_msg.local_channel_id,
        &mapping_pair_msg.denom,
    );

    cw20_ics20_denoms().update(deps.storage, &ibc_denom, |_| -> StdResult<_> {
        Ok(Cw20MappingMetadata {
            cw20_denom: mapping_pair_msg.cw20_denom.clone(),
            remote_decimals: mapping_pair_msg.remote_decimals,
            cw20_decimals: mapping_pair_msg.cw20_decimals,
        })
    })?;

    let res = Response::new()
        .add_attribute("action", "update_cw20_ics20_mapping_pair")
        .add_attribute("denom", mapping_pair_msg.denom)
        .add_attribute("new_cw20", mapping_pair_msg.cw20_denom.clone());
    Ok(res)
}

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    // we don't need to save anything if migrating from the same version
    if msg.default_gas_limit.is_some() {
        CONFIG.update(deps.storage, |mut old| -> StdResult<_> {
            old.default_gas_limit = msg.default_gas_limit;
            Ok(old)
        })?;
    }
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new())
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Port {} => to_binary(&query_port(deps)?),
        QueryMsg::ListChannels {} => to_binary(&query_list(deps)?),
        QueryMsg::Channel { id } => to_binary(&query_channel(deps, id)?),
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Allowed { contract } => to_binary(&query_allowed(deps, contract)?),
        QueryMsg::ListAllowed {
            start_after,
            limit,
            order,
        } => to_binary(&list_allowed(deps, start_after, limit, order)?),
        QueryMsg::Cw20Mapping {
            start_after,
            limit,
            order,
        } => to_binary(&list_cw20_mapping(deps, start_after, limit, order)?),
        QueryMsg::Cw20MappingFromKey { key } => to_binary(&get_cw20_mapping_from_key(deps, key)?),
        QueryMsg::Cw20MappingFromCw20Denom { cw20_denom } => {
            to_binary(&get_cw20_mapping_from_cw20_denom(deps, cw20_denom)?)
        }
        QueryMsg::Admin {} => to_binary(&ADMIN.query_admin(deps)?),
    }
}

fn query_port(deps: Deps) -> StdResult<PortResponse> {
    let query = IbcQuery::PortId {}.into();
    let PortIdResponse { port_id } = deps.querier.query(&query)?;
    Ok(PortResponse { port_id })
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
    let (balances, total_sent): (Vec<Amount>, Vec<Amount>) = state.into_iter().unzip();

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
        default_gas_limit: cfg.default_gas_limit,
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

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

fn list_allowed(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
    order: Option<u8>,
) -> StdResult<ListAllowedResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let addr = maybe_addr(deps.api, start_after)?;
    let start = addr.as_ref().map(Bound::exclusive);

    let allow = ALLOW_LIST
        .range(deps.storage, start, None, map_order(order))
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

fn list_cw20_mapping(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
    order: Option<u8>,
) -> StdResult<ListCw20MappingResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let mut allow_range = cw20_ics20_denoms().range(deps.storage, None, None, map_order(order));
    if let Some(start_after) = start_after {
        let start = Some(Bound::exclusive::<&str>(&start_after));
        allow_range = cw20_ics20_denoms().range(deps.storage, start, None, map_order(order));
    }
    let pairs = allow_range
        .take(limit)
        .map(|item| {
            item.map(|(key, mapping)| Cw20PairQuery {
                key,
                cw20_map: mapping,
            })
        })
        .collect::<StdResult<_>>()?;
    Ok(ListCw20MappingResponse { pairs })
}

fn get_cw20_mapping_from_key(deps: Deps, ibc_denom: String) -> StdResult<Cw20PairQuery> {
    let result = cw20_ics20_denoms().load(deps.storage, &ibc_denom)?;
    Ok(Cw20PairQuery {
        key: ibc_denom,
        cw20_map: result,
    })
}

fn get_cw20_mapping_from_cw20_denom(deps: Deps, cw20_denom: String) -> StdResult<Cw20PairQuery> {
    let cw20_mapping_result = cw20_ics20_denoms()
        .idx
        .cw20_denom
        .item(deps.storage, cw20_denom)?;
    if cw20_mapping_result.is_none() {
        return Err(StdError::generic_err(
            "cw20 mapping pair from the given cw20 denom is not found",
        ));
    }
    let cw20_mapping = cw20_mapping_result.unwrap();
    Ok(Cw20PairQuery {
        key: String::from_utf8(cw20_mapping.0)?,
        cw20_map: cw20_mapping.1,
    })
}

fn map_order(order: Option<u8>) -> Order {
    if order.is_none() {
        return Order::Ascending;
    }
    let order_unwrap = order.unwrap();
    if order_unwrap == 1 {
        return Order::Ascending;
    }
    Order::Descending
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ibc::ibc_packet_receive;
    use crate::test_helpers::*;

    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{
        coin, coins, CosmosMsg, IbcEndpoint, IbcMsg, IbcPacket, IbcPacketReceiveMsg, StdError,
        Timestamp, Uint128,
    };
    use cw_controllers::AdminError;

    use cw_utils::PaymentError;

    #[test]
    fn test_split_denom() {
        let split_denom: Vec<&str> = "orai".splitn(3, '/').collect();
        assert_eq!(split_denom.len(), 1);

        let split_denom: Vec<&str> = "a/b/c".splitn(3, '/').collect();
        assert_eq!(split_denom.len(), 3)
    }

    #[test]
    fn setup_and_query() {
        let deps = setup(&["channel-3", "channel-7"], &[]);

        let raw_list = query(deps.as_ref(), mock_env(), QueryMsg::ListChannels {}).unwrap();
        let list_res: ListChannelsResponse = from_binary(&raw_list).unwrap();
        assert_eq!(2, list_res.channels.len());
        assert_eq!(mock_channel_info("channel-3"), list_res.channels[0]);
        assert_eq!(mock_channel_info("channel-7"), list_res.channels[1]);

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
        assert_eq!(err, StdError::not_found("cw20_ics20::state::ChannelInfo"));
    }

    #[test]
    fn test_update_cw20_mapping() {
        let mut deps = setup(&["channel-3", "channel-7"], &[]);

        let update = Cw20PairMsg {
            local_channel_id: "mars-channel".to_string(),
            denom: "earth".to_string(),
            cw20_denom: "cw20:foobar".to_string(),
            remote_decimals: 18,
            cw20_decimals: 18,
        };

        // works with proper funds
        let msg = ExecuteMsg::UpdateCw20MappingPair(update.clone());

        // unauthorized case
        let info = mock_info("foobar", &coins(1234567, "ucosm"));
        let res_err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
        assert_eq!(res_err, ContractError::Admin(AdminError::NotAdmin {}));

        let info = mock_info("gov", &coins(1234567, "ucosm"));
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // query to verify if the mapping has been updated
        let mappings = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Cw20Mapping {
                start_after: None,
                limit: None,
                order: None,
            },
        )
        .unwrap();
        let response: ListCw20MappingResponse = from_binary(&mappings).unwrap();
        println!("response: {:?}", response);
        assert_eq!(
            response.pairs.first().unwrap().key,
            format!("{}/mars-channel/earth", CONTRACT_PORT)
        );

        // not found case
        let mappings = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Cw20Mapping {
                start_after: None,
                limit: None,
                order: None,
            },
        )
        .unwrap();
        let response: ListCw20MappingResponse = from_binary(&mappings).unwrap();
        assert_ne!(response.pairs.first().unwrap().key, "foobar".to_string());
    }

    #[test]
    fn proper_checks_on_execute_native() {
        let send_channel = "channel-5";
        let mut deps = setup(&[send_channel, "channel-10"], &[]);

        let mut transfer = TransferMsg {
            channel: send_channel.to_string(),
            remote_address: "foreign-address".to_string(),
            timeout: None,
            memo: Some("memo".to_string()),
        };

        // works with proper funds
        let msg = ExecuteMsg::Transfer(transfer.clone());
        let info = mock_info("foobar", &coins(1234567, "ucosm"));
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages[0].gas_limit, None);
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
        let mut deps = setup(&["channel-3", send_channel], &[(cw20_addr, 123456)]);

        let transfer = TransferMsg {
            channel: send_channel.to_string(),
            remote_address: "foreign-address".to_string(),
            timeout: Some(7777),
            memo: Some("memo".to_string()),
        };
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
    fn execute_cw20_fails_if_not_whitelisted_unless_default_gas_limit() {
        let send_channel = "channel-15";
        let mut deps = setup(&[send_channel], &[]);

        let cw20_addr = "my-token";
        let transfer = TransferMsg {
            channel: send_channel.to_string(),
            remote_address: "foreign-address".to_string(),
            timeout: Some(7777),
            memo: Some("memo".to_string()),
        };
        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "my-account".into(),
            amount: Uint128::new(888777666),
            msg: to_binary(&transfer).unwrap(),
        });

        // rejected as not on allow list
        let info = mock_info(cw20_addr, &[]);
        let err = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::NotOnAllowList);

        // add a default gas limit
        migrate(
            deps.as_mut(),
            mock_env(),
            MigrateMsg {
                default_gas_limit: Some(123456),
            },
        )
        .unwrap();

        // try again
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    }
    // test execute transfer back to native remote chain

    fn mock_receive_packet(
        remote_channel: &str,
        local_channel: &str,
        amount: u128,
        denom: &str,
        receiver: &str,
    ) -> IbcPacket {
        let data = Ics20Packet {
            // this is returning a foreign (our) token, thus denom is <port>/<channel>/<denom>
            denom: denom.to_string(),
            amount: amount.into(),
            sender: "remote-sender".to_string(),
            receiver: receiver.to_string(),
            memo: Some("memo".to_string()),
        };
        IbcPacket::new(
            to_binary(&data).unwrap(),
            IbcEndpoint {
                port_id: REMOTE_PORT.to_string(),
                channel_id: remote_channel.to_string(),
            },
            IbcEndpoint {
                port_id: CONTRACT_PORT.to_string(),
                channel_id: local_channel.to_string(),
            },
            3,
            Timestamp::from_seconds(1665321069).into(),
        )
    }

    #[test]
    fn proper_checks_on_execute_native_transfer_back_to_remote() {
        // arrange
        let remote_channel = "channel-5";
        let custom_addr = "custom-addr";
        let original_sender = "original_sender";
        let denom = "uatom";
        let amount = 1234567u128;
        let cw20_denom = "cw20:token-addr";
        let cw20_raw_denom = "token-addr";
        let local_channel = "channel-1234";
        let mut deps = setup(&[remote_channel, local_channel], &[]);

        let pair = Cw20PairMsg {
            local_channel_id: local_channel.to_string(),
            denom: denom.to_string(),
            cw20_denom: cw20_denom.to_string(),
            remote_decimals: 18u8,
            cw20_decimals: 18u8,
        };

        let _ = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("gov", &[]),
            ExecuteMsg::UpdateCw20MappingPair(pair),
        )
        .unwrap();

        // execute
        let mut transfer = TransferBackMsg {
            local_channel_id: local_channel.to_string(),
            remote_address: "foreign-address".to_string(),
            timeout: Some(DEFAULT_TIMEOUT),
            memo: None,
        };

        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: original_sender.to_string(),
            amount: Uint128::from(amount),
            msg: to_binary(&transfer).unwrap(),
        });

        // insufficient funds case because we need to receive from remote chain first
        let info = mock_info(cw20_raw_denom, &[]);
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap_err();
        assert_eq!(res, ContractError::InsufficientFunds {});

        // prepare some mock packets
        let recv_packet =
            mock_receive_packet(remote_channel, local_channel, amount, denom, custom_addr);

        // receive some tokens. Assume that the function works perfectly because the test case is elsewhere
        let ibc_msg = IbcPacketReceiveMsg::new(recv_packet.clone());
        ibc_packet_receive(deps.as_mut(), mock_env(), ibc_msg).unwrap();

        // error cases
        // revert transfer state to correct state
        transfer.local_channel_id = local_channel.to_string();
        let invalid_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: original_sender.to_string(),
            amount: Uint128::from(amount),
            msg: to_binary(&transfer).unwrap(),
        });

        // now we execute transfer back to remote chain
        let res = execute(deps.as_mut(), mock_env(), info.clone(), invalid_msg).unwrap();

        assert_eq!(res.messages[0].gas_limit, None);
        assert_eq!(1, res.messages.len());
        if let CosmosMsg::Ibc(IbcMsg::SendPacket {
            channel_id,
            data,
            timeout,
        }) = &res.messages[0].msg
        {
            let expected_timeout = mock_env().block.time.plus_seconds(DEFAULT_TIMEOUT);
            assert_eq!(timeout, &expected_timeout.into());
            assert_eq!(channel_id.as_str(), local_channel);
            let msg: Ics20Packet = from_binary(data).unwrap();
            assert_eq!(msg.amount, Uint128::new(1234567));
            assert_eq!(
                msg.denom.as_str(),
                get_key_ics20_ibc_denom(CONTRACT_PORT, local_channel, denom)
            );
            assert_eq!(msg.sender.as_str(), original_sender);
            assert_eq!(msg.receiver.as_str(), "foreign-address");
            assert_eq!(msg.memo, None);
        } else {
            panic!("Unexpected return message: {:?}", res.messages[0]);
        }

        // check new channel state after reducing balance
        let chan = query_channel(deps.as_ref(), local_channel.into()).unwrap();
        assert_eq!(
            chan.balances,
            vec![Amount::native(
                0,
                &get_key_ics20_ibc_denom(CONTRACT_PORT, local_channel, denom)
            )]
        );
        assert_eq!(
            chan.total_sent,
            vec![Amount::native(
                amount,
                &get_key_ics20_ibc_denom(CONTRACT_PORT, local_channel, denom)
            )]
        );

        // reject case with bad channel id
        let pair = Cw20PairMsg {
            local_channel_id: "not_registered_channel".to_string(),
            denom: denom.to_string(),
            cw20_denom: "random_cw20_denom".to_string(),
            remote_decimals: 18u8,
            cw20_decimals: 18u8,
        };

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("gov", &[]),
            ExecuteMsg::UpdateCw20MappingPair(pair),
        )
        .unwrap();

        transfer.local_channel_id = "not_registered_channel".to_string();
        let invalid_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: original_sender.to_string(),
            amount: Uint128::from(amount),
            msg: to_binary(&transfer).unwrap(),
        });
        let err = execute(deps.as_mut(), mock_env(), info.clone(), invalid_msg).unwrap_err();
        assert_eq!(
            err,
            ContractError::NoSuchChannel {
                id: "not_registered_channel".to_string()
            }
        );
    }
}
