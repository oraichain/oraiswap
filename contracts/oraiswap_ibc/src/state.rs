use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, IbcEndpoint, Order, StdError, StdResult, Storage, Uint128};
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};

use crate::ContractError;

pub const ADMIN: Admin = Admin::new("admin");

pub const CONFIG: Item<Config> = Item::new("ics20_config");

// Used to pass info from the ibc_packet_receive to the reply handler
pub const REPLY_ARGS: Item<ReplyArgs> = Item::new("reply_args");

pub const LOCKUP: Map<(&str, &str), String> = Map::new("lockups");

/// static info on one channel that doesn't change
pub const CHANNEL_INFO: Map<&str, ChannelInfo> = Map::new("channel_info");

/// indexed by (channel_id, denom) maintaining the balance of the channel in that currency
pub const CHANNEL_STATE: Map<(&str, &str), ChannelState> = Map::new("channel_state");

/// Every cw20 contract we allow to be sent is stored here, possibly with a gas_limit
pub const ALLOW_LIST: Map<&Addr, AllowInfo> = Map::new("allow_list");

pub const EXTERNAL_TOKENS: Map<&str, ExternalTokenInfo> = Map::new("external_tokens");

#[cw_serde]
pub struct ChannelState {
    pub outstanding: Uint128,
    pub total_sent: Uint128,
}

#[cw_serde]
pub struct Config {
    pub default_timeout: u64,
    pub init_channel: bool,
    /// Default remote denom for send standalone actions
    pub default_remote_denom: Option<String>,
}

#[cw_serde]
pub struct ChannelInfo {
    /// id of this channel
    pub id: String,
    /// the remote channel/port we connect to
    pub counterparty_endpoint: IbcEndpoint,
    /// the connection this exists on (you can use to query client/consensus info)
    pub connection_id: String,
}

#[cw_serde]
pub struct AllowInfo {
    pub gas_limit: Option<u64>,
}

#[cw_serde]
pub struct ExternalTokenInfo {
    pub contract: Addr,
}

#[cw_serde]
pub struct ReplyArgs {
    pub channel: String,
    pub denom: String,
    pub amount: Uint128,
    pub our_chain: bool,
}

pub fn join_ibc_paths(path_a: &str, path_b: &str) -> String {
    format!("{}/{}", path_a, path_b)
}

pub fn increase_channel_balance(
    storage: &mut dyn Storage,
    channel: &str,
    denom: &str,
    amount: Uint128,
) -> Result<(), ContractError> {
    CHANNEL_STATE.update(storage, (channel, denom), |orig| -> StdResult<_> {
        if let Some(mut state) = orig {
            state.outstanding += amount;
            state.total_sent += amount;
            return Ok(state);
        }
        Err(StdError::generic_err("Channel is empty"))
    })?;
    Ok(())
}

pub fn reduce_channel_balance(
    storage: &mut dyn Storage,
    channel: &str,
    denom: &str,
    amount: Uint128,
) -> Result<(), ContractError> {
    CHANNEL_STATE.update(
        storage,
        (channel, denom),
        |orig| -> Result<_, ContractError> {
            // this will return error if we don't have the funds there to cover the request (or no denom registered)
            let mut cur = orig.ok_or(ContractError::InsufficientFunds {})?;
            cur.outstanding = cur
                .outstanding
                .checked_sub(amount)
                .or(Err(ContractError::InsufficientFunds {}))?;
            Ok(cur)
        },
    )?;
    Ok(())
}

pub fn find_external_token(
    storage: &mut dyn Storage,
    contract: String,
) -> StdResult<Option<String>> {
    let allow: Vec<String> = EXTERNAL_TOKENS
        .range(storage, None, None, Order::Ascending)
        .filter(|item| {
            if let Ok((_, allow)) = item {
                allow.contract.eq(&contract)
            } else {
                false
            }
        })
        .map(|d| d.map(|(denom, _)| denom))
        .collect::<StdResult<_>>()?;

    if allow.is_empty() {
        return Ok(None);
    }

    return Ok(Some(allow.get(0).unwrap().to_string()));
}

// this is like increase, but it only "un-subtracts" (= adds) outstanding, not total_sent
// calling `reduce_channel_balance` and then `undo_reduce_channel_balance` should leave state unchanged.
pub fn undo_reduce_channel_balance(
    storage: &mut dyn Storage,
    channel: &str,
    denom: &str,
    amount: Uint128,
) -> Result<(), ContractError> {
    CHANNEL_STATE.update(storage, (channel, denom), |orig| -> StdResult<_> {
        if let Some(mut state) = orig {
            state.outstanding += amount;
            return Ok(state);
        }
        Err(StdError::generic_err("Channel is empty"))
    })?;
    Ok(())
}
