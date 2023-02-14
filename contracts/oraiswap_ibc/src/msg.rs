use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Uint128, Uint64};

use cw20::Cw20ReceiveMsg;

#[allow(unused_imports)]
use cw_controllers::AdminResponse;

use crate::amount::Amount;
use crate::state::ChannelInfo;

#[cw_serde]
pub struct InstantiateMsg {
    /// Default timeout for ics20 packets, specified in seconds
    pub default_timeout: u64,
    /// who can allow more contracts
    pub gov_contract: String,
    /// initial allowlist - all cw20 tokens we will send must be previously allowed by governance
    pub allowlist: Vec<AllowMsg>,
}

#[cw_serde]
pub struct AllowMsg {
    pub contract: String,
    pub gas_limit: Option<u64>,
}

#[cw_serde]
pub struct ExternalTokenMsg {
    /// External denom
    pub denom: String,
    /// CW20 Token
    pub contract: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// This accepts a properly-encoded ReceiveMsg from a cw20 contract
    Receive(Cw20ReceiveMsg),
    /// This allows us to transfer *exactly one* native token
    Transfer(TransferMsg),
    /// This allows us to swap tokens
    Swap(SwapMsg),
    /// This allows us to add liquidity
    JoinPool(JoinPoolMsg),
    /// This allows us to remove liquidity
    ExitPool(ExitPoolMsg),
    /// Create lockup account in osmosis.
    CreateLockup(CreateLockupMsg),
    /// Lock Tokens (Start Farming)
    LockTokens(LockTokensMsg),
    /// This allows us to claim rewards and LP tokens (Unlocked).
    ClaimTokens(ClaimTokensMsg),
    /// Begin Unlocking tokens
    UnlockTokens(UnlockTokensMsg),
    /// This must be called by gov_contract, will allow a new cw20 token to be sent
    Allow(AllowMsg),
    /// This must be called by gov_contract, will allow a new external token to be received
    AllowExternalToken(ExternalTokenMsg),
    /// Change the admin (must be called by current admin)
    UpdateAdmin { admin: String },
}

#[cw_serde]
pub struct SwapMsg {
    pub channel: String,
    pub pool: Uint64,
    pub token_out: String,
    pub min_amount_out: Uint128,
    pub timeout: Option<u64>,
}

#[cw_serde]
pub struct JoinPoolMsg {
    pub channel: String,
    pub pool: Uint64,
    pub share_min_out: Uint128,
    pub timeout: Option<u64>,
}

#[cw_serde]
pub struct ExitPoolMsg {
    pub channel: String,
    pub token_out: String,
    pub min_amount_out: Uint128,
    pub timeout: Option<u64>,
}

#[cw_serde]
pub struct CreateLockupMsg {
    pub channel: String,
    pub timeout: Option<u64>,
}

#[cw_serde]
pub struct LockTokensMsg {
    pub channel: String,
    pub timeout: Option<u64>,
    pub duration: Uint64,
}

#[cw_serde]
pub struct ClaimTokensMsg {
    pub channel: String,
    pub timeout: Option<u64>,
    pub denom: String,
}

#[cw_serde]
pub struct UnlockTokensMsg {
    pub channel: String,
    pub timeout: Option<u64>,
    pub lock_id: Uint64,
}

/// This is the message we accept via Receive
#[cw_serde]
pub struct TransferMsg {
    /// The local channel to send the packets on
    pub channel: String,
    /// The remote address to send to.
    /// Don't use HumanAddress as this will likely have a different Bech32 prefix than we use
    /// and cannot be validated locally
    pub remote_address: String,
    /// How long the packet lives in seconds. If not specified, use default_timeout
    pub timeout: Option<u64>,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Show all channels we have connected to. Return type is ListChannelsResponse.
    #[returns(ListChannelsResponse)]
    ListChannels {},
    /// Returns the details of the name channel, error if not created.
    /// Return type: ChannelResponse.
    #[returns(ChannelResponse)]
    Channel { id: String },
    /// Show the Config. Returns ConfigResponse (currently including admin as well)
    #[returns(ConfigResponse)]
    Config {},
    /// Return AdminResponse
    #[returns(AdminResponse)]
    Admin {},
    /// Query if a given cw20 contract is allowed. Returns AllowedResponse
    #[returns(AllowedResponse)]
    Allowed { contract: String },
    /// Query if a given external token is allowed. Returns AllowedTokenResponse
    #[returns(AllowedTokenResponse)]
    ExternalToken { denom: String },
    /// List all allowed cw20 contracts. Returns ListAllowedResponse
    #[returns(ListAllowedResponse)]
    ListAllowed {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// List all allowed external tokens. Returns ListExternalTokensResponse
    #[returns(ListExternalTokensResponse)]
    ListExternalTokens {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Returns the lockup address of the channel and owner, empty if not created.
    /// Return type: LockupResponse.
    #[returns(LockupResponse)]
    Lockup { channel: String, owner: String },
}

#[cw_serde]
pub struct ListChannelsResponse {
    pub channels: Vec<ChannelInfo>,
}

#[cw_serde]
pub struct ChannelResponse {
    /// Information on the channel's connection
    pub info: ChannelInfo,
    /// How many tokens we currently have pending over this channel
    pub balances: Vec<Amount>,
    /// The total number of tokens that have been sent over this channel
    /// (even if many have been returned, so balance is low)
    pub total_sent: Vec<Amount>,
}

#[cw_serde]
pub struct ConfigResponse {
    pub default_timeout: u64,
    pub gov_contract: String,
}

#[cw_serde]
pub struct AllowedResponse {
    pub is_allowed: bool,
    pub gas_limit: Option<u64>,
}

#[cw_serde]
pub struct ListAllowedResponse {
    pub allow: Vec<AllowedInfo>,
}

#[cw_serde]
pub struct AllowedTokenResponse {
    pub is_allowed: bool,
    pub contract: Option<String>,
}

#[cw_serde]
pub struct AllowedInfo {
    pub contract: String,
    pub gas_limit: Option<u64>,
}

#[cw_serde]
pub struct ListExternalTokensResponse {
    pub tokens: Vec<AllowedTokenInfo>,
}

#[cw_serde]
pub struct AllowedTokenInfo {
    pub denom: String,
    pub contract: String,
}

#[cw_serde]
pub struct LockupResponse {
    pub owner: String,
    pub address: String,
}
