/*!
Shared msgs for the cw20-ics20 and other contracts that interact with it
*/

pub mod ack_fail;
pub mod amount;

use amount::Amount;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Binary;

#[cw_serde]
pub struct DelegateCw20Msg {
    /// token from the remote chain
    pub token: Amount,
    /// the decimals of the native token, popular is 18 or 6
    pub from_decimals: u8,
    /// additional data from the memo of the IBC transfer packet
    pub data: Binary,
}
