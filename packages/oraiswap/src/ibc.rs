use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, Timestamp, Uint128, Uint64};

/// Swap Packet
#[cw_serde]
pub struct SwapPacket {
    pub routes: Vec<SwapAmountInRoute>,
    pub token_out_min_amount: Uint128,
}

#[cw_serde]
pub struct SwapAmountInRoute {
    pub pool_id: Uint64,
    pub token_out_denom: String,
}

/// JoinPool Packet
#[cw_serde]
pub struct JoinPoolPacket {
    pub pool_id: Uint64,
    pub share_out_min_amount: Uint128,
}

/// ExitPool Packet
#[cw_serde]
pub struct ExitPoolPacket {
    pub token_out_denom: String,
    pub token_out_min_amount: Uint128,
}

#[cw_serde]
pub struct LockPacket {
    pub duration: Uint64,
}

#[cw_serde]
pub struct ClaimPacket {
    pub denom: String,
}

#[cw_serde]
pub struct UnlockPacket {
    pub id: Uint64,
}

/// Just acknowledge success or error
#[cw_serde]
pub struct SwapAmountInAck {
    pub amount: Uint128,
    pub denom: String,
}

#[cw_serde]
pub struct CreateLockupAck {
    pub contract: String,
}

#[cw_serde]
pub struct LockResultAck {
    pub lock_id: Uint64,
}

#[cw_serde]
pub struct UnLockResultAck {
    pub end_time: Timestamp,
}

/// The format for sending an ics20 packet.
/// Proto defined here: https://github.com/cosmos/ibc-go/blob/v2.0.2/proto/ibc/applications/transfer/v2/packet.proto#L10-L19
/// This is compatible with the JSON serialization
#[cw_serde]
pub struct Ics20Packet {
    /// amount of tokens to transfer is encoded as a string, but limited to u64 max
    pub amount: Uint128,
    /// the token denomination to be transferred
    pub denom: String,
    /// the recipient address on the destination chain
    pub receiver: String,
    /// the sender address
    pub sender: String,

    /// Action packet
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<OsmoPacket>,
}

#[cw_serde]
pub enum OsmoPacket {
    Swap(SwapPacket),
    JoinPool(JoinPoolPacket),
    ExitPool(ExitPoolPacket),
    LockupAccount {},
    Lock(LockPacket),
    Claim(ClaimPacket),
    Unlock(UnlockPacket),
}

impl Ics20Packet {
    pub fn new<T: Into<String>>(
        amount: Uint128,
        denom: T,
        sender: &str,
        receiver: &str,
        action_packet: Option<OsmoPacket>,
    ) -> Self {
        Ics20Packet {
            denom: denom.into(),
            amount,
            sender: sender.to_string(),
            receiver: receiver.to_string(),
            action: action_packet,
        }
    }
}

/// This is a generic ICS acknowledgement format.
/// Proto defined here: https://github.com/cosmos/cosmos-sdk/blob/v0.42.0/proto/ibc/core/channel/v1/channel.proto#L141-L147
/// This is compatible with the JSON serialization
#[cw_serde]
pub enum Ics20Ack {
    Result(Binary),
    Error(String),
}

pub struct Voucher {
    pub denom: String,
    /// denom is from source chain.
    pub our_chain: bool,
}
