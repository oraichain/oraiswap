use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, IbcEndpoint, StdResult, Storage, Uint128};
use cw_controllers::Admin;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use oraiswap::asset::AssetInfo;

use crate::ContractError;

pub const ADMIN: Admin = Admin::new("admin");

pub const CONFIG: Item<Config> = Item::new("ics20_config");

// Used to pass info from the ibc_packet_receive to the reply handler
pub const REPLY_ARGS: Item<ReplyArgs> = Item::new("reply_args");

/// static info on one channel that doesn't change
pub const CHANNEL_INFO: Map<&str, ChannelInfo> = Map::new("channel_info");

/// indexed by (channel_id, denom) maintaining the balance of the channel in that currency. Forward channel state is used when local chain initiates ibc transfer to remote chain
pub const CHANNEL_FORWARD_STATE: Map<(&str, &str), ChannelState> =
    Map::new("channel_forward_state");

/// indexed by (channel_id, denom) maintaining the balance of the channel in that currency. Reverse channel state is used when remote chain initiates ibc transfer to local chain
pub const CHANNEL_REVERSE_STATE: Map<(&str, &str), ChannelState> =
    Map::new("channel_reverse_state");

/// Every cw20 contract we allow to be sent is stored here, possibly with a gas_limit
pub const ALLOW_LIST: Map<&Addr, AllowInfo> = Map::new("allow_list");

// MappingMetadataIndexex structs keeps a list of indexers
pub struct MappingMetadataIndexex<'a> {
    // token.identifier
    pub asset_info: MultiIndex<'a, String, MappingMetadata, String>,
}

// IndexList is just boilerplate code for fetching a struct's indexes
impl<'a> IndexList<MappingMetadata> for MappingMetadataIndexex<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<MappingMetadata>> + '_> {
        let v: Vec<&dyn Index<MappingMetadata>> = vec![&self.asset_info];
        Box::new(v.into_iter())
    }
}

// used when chain A (no cosmwasm) sends native token to chain B (has cosmwasm). key - original denom of chain A, in form of ibc no hash for destination port & channel - transfer/channel-0/uatom for example; value - mapping data including asset info, can be either native or cw20
pub fn ics20_denoms<'a>() -> IndexedMap<'a, &'a str, MappingMetadata, MappingMetadataIndexex<'a>> {
    let indexes = MappingMetadataIndexex {
        asset_info: MultiIndex::new(
            |_k, d| d.asset_info.to_string(),
            "ics20_mapping_namespace",
            "asset__info",
        ),
    };
    IndexedMap::new("ics20_mapping_namespace", indexes)
}

#[cw_serde]
#[derive(Default)]
pub struct ChannelState {
    pub outstanding: Uint128,
    pub total_sent: Uint128,
}

#[cw_serde]
pub struct Config {
    pub default_timeout: u64,
    pub default_gas_limit: Option<u64>,
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
pub struct MappingMetadata {
    /// asset info on local chain. Can be either cw20 or native
    pub asset_info: AssetInfo,
    pub remote_decimals: u8,
    pub asset_info_decimals: u8,
}

#[cw_serde]
pub struct ReplyArgs {
    pub channel: String,
    pub denom: String,
    pub amount: Uint128,
}

pub fn increase_channel_balance(
    storage: &mut dyn Storage,
    channel: &str,
    denom: &str,
    amount: Uint128,
    forward: bool,
) -> Result<(), ContractError> {
    let mut state = CHANNEL_REVERSE_STATE;
    if forward {
        state = CHANNEL_FORWARD_STATE;
    }

    state.update(storage, (channel, denom), |orig| -> StdResult<_> {
        let mut state = orig.unwrap_or_default();
        state.outstanding += amount;
        state.total_sent += amount;
        Ok(state)
    })?;
    Ok(())
}

pub fn reduce_channel_balance(
    storage: &mut dyn Storage,
    channel: &str,
    denom: &str,
    amount: Uint128,
    forward: bool,
) -> Result<(), ContractError> {
    let mut state = CHANNEL_REVERSE_STATE;
    if forward {
        state = CHANNEL_FORWARD_STATE;
    }
    state.update(
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

// this is like increase, but it only "un-subtracts" (= adds) outstanding, not total_sent
// calling `reduce_channel_balance` and then `undo_reduce_channel_balance` should leave state unchanged.
pub fn undo_reduce_channel_balance(
    storage: &mut dyn Storage,
    channel: &str,
    denom: &str,
    amount: Uint128,
    forward: bool,
) -> Result<(), ContractError> {
    let mut state = CHANNEL_REVERSE_STATE;
    if forward {
        state = CHANNEL_FORWARD_STATE;
    }
    state.update(storage, (channel, denom), |orig| -> StdResult<_> {
        let mut state = orig.unwrap_or_default();
        state.outstanding += amount;
        Ok(state)
    })?;
    Ok(())
}

// this is like decrease, but it only "un-add" (= adds) outstanding, not total_sent
// calling `increase_channel_balance` and then `undo_increase_channel_balance` should leave state unchanged.
pub fn undo_increase_channel_balance(
    storage: &mut dyn Storage,
    channel: &str,
    denom: &str,
    amount: Uint128,
    forward: bool,
) -> Result<(), ContractError> {
    let mut state = CHANNEL_REVERSE_STATE;
    if forward {
        state = CHANNEL_FORWARD_STATE;
    }
    state.update(storage, (channel, denom), |orig| -> StdResult<_> {
        let mut state = orig.unwrap_or_default();
        state.outstanding -= amount;
        Ok(state)
    })?;
    Ok(())
}

pub fn get_key_ics20_ibc_denom(port_id: &str, channel_id: &str, denom: &str) -> String {
    format!("{}/{}/{}", port_id, channel_id, denom)
}
