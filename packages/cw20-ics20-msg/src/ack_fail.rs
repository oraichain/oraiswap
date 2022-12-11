use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_binary, Binary, CosmosMsg, StdResult, WasmMsg};

use crate::amount::Amount;

/// Cw20ReceiveMsg should be de/serialized under `IbcWasmReceive()` variant in a ExecuteMsg
#[cw_serde]

pub struct TransferBackFailAckMsg {
    pub original_sender: String,
    pub from_decimals: u8,
    pub amount: Amount,
}

impl TransferBackFailAckMsg {
    /// serializes the message
    pub fn into_binary(self) -> StdResult<Binary> {
        let msg = ReceiverExecuteMsg::IbcWasmTransferAckFailed(self);
        to_binary(&msg)
    }

    /// creates a cosmos_msg sending this struct to the named contract
    pub fn into_cosmos_msg<T: Into<String>>(self, contract_addr: T) -> StdResult<CosmosMsg> {
        let msg = self.into_binary()?;
        let execute = WasmMsg::Execute {
            contract_addr: contract_addr.into(),
            msg,
            funds: vec![],
        };
        Ok(execute.into())
    }
}

// This is just a helper to properly serialize the above message
#[cw_serde]

enum ReceiverExecuteMsg {
    IbcWasmTransferAckFailed(TransferBackFailAckMsg),
}
