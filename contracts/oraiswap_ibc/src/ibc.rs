use cosmwasm_std::{
    attr, entry_point, from_binary, to_binary, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env,
    IbcBasicResponse, IbcChannel, IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg,
    IbcEndpoint, IbcOrder, IbcPacket, IbcPacketAckMsg, IbcPacketReceiveMsg, IbcPacketTimeoutMsg,
    IbcReceiveResponse, Reply, Response, Storage, SubMsg, SubMsgResult, WasmMsg,
};

use crate::amount::{get_cw20_denom, Amount};
use crate::error::{ContractError, Never};
use crate::state::{
    join_ibc_paths, reduce_channel_balance, undo_reduce_channel_balance, ChannelInfo, ReplyArgs,
    ALLOW_LIST, CHANNEL_INFO, CONFIG, EXTERNAL_TOKENS, LOCKUP, REPLY_ARGS,
};
use cw20::Cw20ExecuteMsg;
use oraiswap::ibc::{
    CreateLockupAck, Ics20Ack, Ics20Packet, LockResultAck, OsmoPacket, SwapAmountInAck, Voucher,
};

pub const ICS20_VERSION: &str = "ics20-1";
pub const ICS20_ORDERING: IbcOrder = IbcOrder::Unordered;

// create a serialized success message
fn ack_success() -> Binary {
    let res = Ics20Ack::Result(b"1".into());
    to_binary(&res).unwrap()
}

// create a serialized error message
fn ack_fail(err: String) -> Binary {
    let res = Ics20Ack::Error(err);
    to_binary(&res).unwrap()
}

const RECEIVE_ID: u64 = 1337;
const ACK_TRANSFER_ID: u64 = 0xfa17;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> Result<Response, ContractError> {
    match reply.id {
        RECEIVE_ID => match reply.result {
            SubMsgResult::Ok(_) => Ok(Response::new()),
            SubMsgResult::Err(err) => {
                let reply_args = REPLY_ARGS.load(deps.storage)?;

                if reply_args.our_chain {
                    undo_reduce_channel_balance(
                        deps.storage,
                        &reply_args.channel,
                        &reply_args.denom,
                        reply_args.amount,
                    )?;
                }

                Ok(Response::new().set_data(ack_fail(err)))
            }
        },
        ACK_TRANSFER_ID => match reply.result {
            SubMsgResult::Ok(_) => Ok(Response::new()),
            SubMsgResult::Err(err) => Ok(Response::new().set_data(ack_fail(err))),
        },
        _ => Err(ContractError::UnknownReplyId { id: reply.id }),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// enforces ordering and versioning constraints
pub fn ibc_channel_open(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelOpenMsg,
) -> Result<(), ContractError> {
    enforce_order_and_version(msg.channel(), msg.counterparty_version())?;

    let cfg = CONFIG.load(deps.storage)?;
    if cfg.init_channel {
        return Err(ContractError::OnlyOneChannel {});
    }

    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// record the channel in CHANNEL_INFO
pub fn ibc_channel_connect(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelConnectMsg,
) -> Result<IbcBasicResponse, ContractError> {
    // we need to check the counter party version in try and ack (sometimes here)
    enforce_order_and_version(msg.channel(), msg.counterparty_version())?;

    let channel: IbcChannel = msg.into();
    let info = ChannelInfo {
        id: channel.endpoint.channel_id,
        counterparty_endpoint: channel.counterparty_endpoint,
        connection_id: channel.connection_id,
    };
    CHANNEL_INFO.save(deps.storage, &info.id, &info)?;
    CONFIG.update(deps.storage, |mut cfg| -> Result<_, ContractError> {
        cfg.init_channel = true;
        Ok(cfg)
    })?;

    Ok(IbcBasicResponse::default())
}

fn enforce_order_and_version(
    channel: &IbcChannel,
    counterparty_version: Option<&str>,
) -> Result<(), ContractError> {
    if channel.version.as_str() != ICS20_VERSION {
        return Err(ContractError::InvalidIbcVersion {
            version: channel.version.clone(),
        });
    }
    if let Some(version) = counterparty_version {
        if version != ICS20_VERSION {
            return Err(ContractError::InvalidIbcVersion {
                version: version.to_string(),
            });
        }
    }
    if channel.order != ICS20_ORDERING {
        return Err(ContractError::OnlyOrderedChannel {});
    }
    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
#[allow(unreachable_patterns)]
pub fn ibc_channel_close(
    _deps: DepsMut,
    _env: Env,
    channel: IbcChannelCloseMsg,
) -> Result<IbcBasicResponse, ContractError> {
    match channel {
        IbcChannelCloseMsg::CloseConfirm { .. } => Ok(IbcBasicResponse::new()),
        IbcChannelCloseMsg::CloseInit { .. } => Err(ContractError::CannotClose {}),
        _ => panic!(),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// Check to see if we have any balance here
/// We should not return an error if possible, but rather an acknowledgement of failure
pub fn ibc_packet_receive(
    deps: DepsMut,
    _env: Env,
    msg: IbcPacketReceiveMsg,
) -> Result<IbcReceiveResponse, Never> {
    let packet = msg.packet;

    do_ibc_packet_receive(deps, &packet).or_else(|err| {
        Ok(IbcReceiveResponse::new()
            .set_ack(ack_fail(err.to_string()))
            .add_attributes(vec![
                attr("action", "receive"),
                attr("success", "false"),
                attr("error", err.to_string()),
            ]))
    })
}

// Returns local denom if the denom is an encoded voucher from the expected endpoint
// Otherwise, error
fn parse_voucher(
    storage: &mut dyn Storage,
    voucher_denom: String,
    remote_endpoint: &IbcEndpoint,
) -> Result<Voucher, ContractError> {
    let ibc_prefix = join_ibc_paths(&remote_endpoint.port_id, &remote_endpoint.channel_id);
    if !voucher_denom.starts_with(&ibc_prefix) {
        let token = EXTERNAL_TOKENS
            .load(storage, voucher_denom.as_ref())
            .map_err(|_| ContractError::NoAllowedToken {})?;

        let data = Voucher {
            denom: get_cw20_denom(token.contract.as_str()),
            our_chain: false,
        };
        return Ok(data);
    }

    let split_denom: Vec<&str> = voucher_denom.splitn(3, '/').collect();
    if split_denom.len() != 3 {
        return Err(ContractError::NoForeignTokens {});
    }
    // a few more sanity checks
    if split_denom[0] != remote_endpoint.port_id {
        return Err(ContractError::FromOtherPort {
            port: split_denom[0].into(),
        });
    }
    if split_denom[1] != remote_endpoint.channel_id {
        return Err(ContractError::FromOtherChannel {
            channel: split_denom[1].into(),
        });
    }

    Ok(Voucher {
        denom: split_denom[2].to_string(),
        our_chain: true,
    })
}

fn parse_voucher_ack(
    storage: &mut dyn Storage,
    voucher_denom: String,
    remote_endpoint: &IbcEndpoint,
) -> Result<Voucher, ContractError> {
    let ibc_prefix = join_ibc_paths(&remote_endpoint.port_id, &remote_endpoint.channel_id);
    if !voucher_denom.starts_with(&ibc_prefix) {
        return Ok(Voucher {
            denom: voucher_denom,
            our_chain: true,
        });
    }

    let split_denom: Vec<&str> = voucher_denom.splitn(3, '/').collect();
    if split_denom.len() != 3 {
        return Err(ContractError::NoForeignTokens {});
    }

    let token = EXTERNAL_TOKENS
        .load(storage, split_denom[2])
        .map_err(|_| ContractError::NoAllowedToken {})?;

    Ok(Voucher {
        denom: get_cw20_denom(token.contract.as_str()),
        our_chain: false,
    })
}

// this does the work of ibc_packet_receive, we wrap it to turn errors into acknowledgements
fn do_ibc_packet_receive(
    deps: DepsMut,
    packet: &IbcPacket,
) -> Result<IbcReceiveResponse, ContractError> {
    let msg: Ics20Packet = from_binary(&packet.data)?;
    let channel = packet.dest.channel_id.clone();

    // If the token originated on the remote chain, it looks like "ucosm".
    // If it originated on our chain, it looks like "port/channel/ucosm".
    let voucher = parse_voucher(deps.storage, msg.denom, &packet.src)?;
    let denom = voucher.denom.as_str();

    if voucher.our_chain {
        // make sure we have enough balance for this
        reduce_channel_balance(deps.storage, &channel, denom, msg.amount)?;
    }

    // we need to save the data to update the balances in reply
    let reply_args = ReplyArgs {
        channel,
        denom: denom.to_string(),
        amount: msg.amount,
        our_chain: voucher.our_chain,
    };
    REPLY_ARGS.save(deps.storage, &reply_args)?;

    let to_send = Amount::from_parts(denom.to_string(), msg.amount);
    let gas_limit = check_gas_limit(deps.as_ref(), &to_send)?;
    let send = send_amount(to_send, msg.receiver.clone(), voucher.our_chain);
    let mut submsg = SubMsg::reply_on_error(send, RECEIVE_ID);
    submsg.gas_limit = gas_limit;

    let res = IbcReceiveResponse::new()
        .set_ack(ack_success())
        .add_submessage(submsg)
        .add_attribute("action", "receive")
        .add_attribute("sender", msg.sender)
        .add_attribute("receiver", msg.receiver)
        .add_attribute("denom", denom)
        .add_attribute("amount", msg.amount)
        .add_attribute("success", "true");

    Ok(res)
}

fn check_gas_limit(deps: Deps, amount: &Amount) -> Result<Option<u64>, ContractError> {
    match amount {
        Amount::Cw20(coin) => {
            // if cw20 token, use the registered gas limit, or error if not whitelisted
            let addr = deps.api.addr_validate(&coin.address)?;
            Ok(ALLOW_LIST
                .may_load(deps.storage, &addr)?
                .ok_or(ContractError::NotOnAllowList)?
                .gas_limit)
        }
        _ => Ok(None),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// check if success or failure and update balance, or return funds
pub fn ibc_packet_ack(
    deps: DepsMut,
    _env: Env,
    msg: IbcPacketAckMsg,
) -> Result<IbcBasicResponse, ContractError> {
    let packet_data: Ics20Packet = from_binary(&msg.original_packet.data)?;
    let ics20msg: Ics20Ack = from_binary(&msg.acknowledgement.data)?;

    if let Some(ref action) = packet_data.action {
        match action {
            OsmoPacket::Swap(_) => {
                on_gamm_packet(deps, msg, packet_data.sender, ics20msg, "acknowledge_swap")
            }
            OsmoPacket::JoinPool(_) => on_gamm_packet(
                deps,
                msg,
                packet_data.sender,
                ics20msg,
                "acknowledge_join_pool",
            ),
            OsmoPacket::ExitPool(_) => on_gamm_packet(
                deps,
                msg,
                packet_data.sender,
                ics20msg,
                "acknowledge_exit_pool",
            ),
            OsmoPacket::LockupAccount {} => on_create_lockup_packet(
                deps,
                msg,
                packet_data.sender,
                ics20msg,
                "acknowledge_create_lockup",
            ),
            OsmoPacket::Lock(_) => {
                on_lock_packet(deps, msg, &packet_data, ics20msg, "acknowledge_lock")
            }
            OsmoPacket::Claim(_) => on_claim_tokens_packet(
                deps,
                msg,
                packet_data.sender,
                ics20msg,
                "acknowledge_claim_tokens",
            ),
            OsmoPacket::Unlock(_) => {
                on_unlock_packet(packet_data.sender, ics20msg, "acknowledge_unlock")
            }
        }
    } else {
        match ics20msg {
            Ics20Ack::Result(_) => on_packet_success(packet_data),
            Ics20Ack::Error(err) => {
                on_packet_failure(deps, msg.original_packet, "acknowledge", err)
            }
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// return fund to original sender (same as failure in ibc_packet_ack)
pub fn ibc_packet_timeout(
    deps: DepsMut,
    _env: Env,
    msg: IbcPacketTimeoutMsg,
) -> Result<IbcBasicResponse, ContractError> {
    let packet = msg.packet;
    on_packet_failure(deps, packet, "acknowledge", "timeout".to_string())
}

// update the balance stored on this (channel, denom) index
fn on_packet_success(msg: Ics20Packet) -> Result<IbcBasicResponse, ContractError> {
    // similar event messages like ibctransfer module
    let attributes = vec![
        attr("action", "acknowledge"),
        attr("sender", &msg.sender),
        attr("receiver", &msg.receiver),
        attr("denom", &msg.denom),
        attr("amount", msg.amount),
        attr("success", "true"),
    ];

    Ok(IbcBasicResponse::new().add_attributes(attributes))
}

// return the tokens to sender
fn on_packet_failure(
    deps: DepsMut,
    packet: IbcPacket,
    action_label: &str,
    err: String,
) -> Result<IbcBasicResponse, ContractError> {
    let msg: Ics20Packet = from_binary(&packet.data)?;

    let voucher = parse_voucher_ack(deps.storage, msg.denom, &packet.src)?;
    let denom = voucher.denom.as_str();

    if voucher.our_chain {
        reduce_channel_balance(deps.storage, &packet.src.channel_id, denom, msg.amount)?;
    }

    let to_send = Amount::from_parts(denom.to_string(), msg.amount);
    let gas_limit = check_gas_limit(deps.as_ref(), &to_send)?;
    let send = send_amount(to_send, msg.sender.clone(), voucher.our_chain);
    let mut submsg = SubMsg::reply_on_error(send, ACK_TRANSFER_ID);
    submsg.gas_limit = gas_limit;

    let mut attributes = vec![
        attr("action", action_label),
        attr("sender", msg.sender),
        attr("denom", denom),
        attr("amount", msg.amount.to_string()),
        attr("success", "false"),
        attr("error", err),
    ];
    if !msg.receiver.is_empty() {
        attributes.push(attr("receiver", &msg.receiver));
    }

    // similar event messages like ibctransfer module
    let res = IbcBasicResponse::new()
        .add_submessage(submsg)
        .add_attributes(attributes);

    Ok(res)
}

fn on_gamm_packet(
    deps: DepsMut,
    msg: IbcPacketAckMsg,
    sender: String,
    ics20msg: Ics20Ack,
    action_label: &str,
) -> Result<IbcBasicResponse, ContractError> {
    match ics20msg {
        Ics20Ack::Result(data) => {
            let res: SwapAmountInAck = from_binary(&data)?;
            on_gamm_packet_success(deps, msg.original_packet, sender, res, action_label)
        }
        Ics20Ack::Error(err) => on_packet_failure(
            deps,
            msg.original_packet,
            action_label,
            format!("Gamm error: {}", err),
        ),
    }
}

fn on_gamm_packet_success(
    deps: DepsMut,
    packet: IbcPacket,
    sender: String,
    res: SwapAmountInAck,
    action_label: &str,
) -> Result<IbcBasicResponse, ContractError> {
    let attributes = vec![
        attr("action", action_label),
        attr("receiver", &sender),
        attr("amount", &res.amount.to_string()),
        attr("denom", &res.denom),
        attr("success", "true"),
    ];

    let channel = packet.src.channel_id.clone();
    let voucher = parse_voucher(deps.storage, res.denom, &packet.dest)?;
    let denom = voucher.denom.as_str();

    if voucher.our_chain {
        // make sure we have enough balance for this
        reduce_channel_balance(deps.storage, &channel, denom, res.amount)?;
    }

    let to_send = Amount::from_parts(denom.to_string(), res.amount);
    let gas_limit = check_gas_limit(deps.as_ref(), &to_send)?;
    let send = send_amount(to_send, sender, voucher.our_chain);
    let mut submsg = SubMsg::reply_on_error(send, ACK_TRANSFER_ID);
    submsg.gas_limit = gas_limit;

    Ok(IbcBasicResponse::new()
        .add_submessage(submsg)
        .add_attributes(attributes))
}

fn on_create_lockup_packet(
    deps: DepsMut,
    msg: IbcPacketAckMsg,
    sender: String,
    ics20msg: Ics20Ack,
    action_label: &str,
) -> Result<IbcBasicResponse, ContractError> {
    match ics20msg {
        Ics20Ack::Result(data) => {
            let ack: CreateLockupAck = from_binary(&data)?;
            let lockup_key = (msg.original_packet.src.channel_id.as_str(), sender.as_str());
            LOCKUP.save(deps.storage, lockup_key, &ack.contract)?;

            let res = IbcBasicResponse::new()
                .add_attribute("action", action_label)
                .add_attribute("sender", sender)
                .add_attribute("success", "true")
                .add_attribute("lockup_address", ack.contract);

            Ok(res)
        }
        Ics20Ack::Error(err) => Ok(result_ack_error(action_label, sender, err)),
    }
}

fn on_lock_packet(
    deps: DepsMut,
    msg: IbcPacketAckMsg,
    ics20_packet: &Ics20Packet,
    ics20msg: Ics20Ack,
    action_label: &str,
) -> Result<IbcBasicResponse, ContractError> {
    match ics20msg {
        Ics20Ack::Result(data) => {
            let ack: LockResultAck = from_binary(&data)?;

            // similar event messages like ibctransfer module
            let attributes = vec![
                attr("action", action_label),
                attr("sender", &ics20_packet.sender),
                attr("denom", &ics20_packet.denom),
                attr("amount", ics20_packet.amount),
                attr("lock_id", ack.lock_id),
                attr("success", "true"),
            ];

            Ok(IbcBasicResponse::new().add_attributes(attributes))
        }
        Ics20Ack::Error(err) => on_packet_failure(
            deps,
            msg.original_packet,
            action_label,
            format!("Gamm error: {}", err),
        ),
    }
}

fn on_claim_tokens_packet(
    deps: DepsMut,
    msg: IbcPacketAckMsg,
    sender: String,
    ics20msg: Ics20Ack,
    action_label: &str,
) -> Result<IbcBasicResponse, ContractError> {
    match ics20msg {
        Ics20Ack::Result(data) => {
            let res: SwapAmountInAck = from_binary(&data)?;
            if res.amount.is_zero() {
                let attributes = vec![
                    attr("action", action_label),
                    attr("sender", &sender),
                    attr("success", "false"),
                    attr("error", "No claim tokens available"),
                ];

                return Ok(IbcBasicResponse::new().add_attributes(attributes));
            }

            on_gamm_packet_success(deps, msg.original_packet, sender, res, action_label)
        }
        Ics20Ack::Error(err) => Ok(result_ack_error(action_label, sender, err)),
    }
}

fn on_unlock_packet(
    sender: String,
    ics20msg: Ics20Ack,
    action_label: &str,
) -> Result<IbcBasicResponse, ContractError> {
    match ics20msg {
        Ics20Ack::Result(_) => {
            let attributes = vec![
                attr("action", action_label),
                attr("sender", &sender),
                attr("success", "true"),
            ];

            Ok(IbcBasicResponse::new().add_attributes(attributes))
        }
        Ics20Ack::Error(err) => Ok(result_ack_error(action_label, sender, err)),
    }
}

fn result_ack_error(action: &str, sender: String, err: String) -> IbcBasicResponse {
    IbcBasicResponse::new()
        .add_attribute("action", action)
        .add_attribute("sender", sender)
        .add_attribute("success", "false")
        .add_attribute("error", err)
}

fn send_amount(amount: Amount, recipient: String, our_chain: bool) -> CosmosMsg {
    match amount {
        Amount::Native(coin) => BankMsg::Send {
            to_address: recipient,
            amount: vec![coin],
        }
        .into(),
        Amount::Cw20(coin) => {
            let msg = if our_chain {
                Cw20ExecuteMsg::Transfer {
                    recipient,
                    amount: coin.amount,
                }
            } else {
                Cw20ExecuteMsg::Mint {
                    recipient,
                    amount: coin.amount,
                }
            };

            WasmMsg::Execute {
                contract_addr: coin.address,
                msg: to_binary(&msg).unwrap(),
                funds: vec![],
            }
            .into()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_helpers::*;

    use crate::contract::{execute, query_channel};
    use crate::msg::{ExecuteMsg, TransferMsg};
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{
        coins, to_vec, IbcEndpoint, IbcMsg, IbcTimeout, Timestamp, Uint128, Uint64,
    };
    use cw20::Cw20ReceiveMsg;
    use oraiswap::ibc::{OsmoPacket, SwapAmountInRoute, SwapPacket};

    #[test]
    fn check_ack_json() {
        let success = Ics20Ack::Result(b"1".into());
        let fail = Ics20Ack::Error("bad coin".into());

        let success_json = String::from_utf8(to_vec(&success).unwrap()).unwrap();
        assert_eq!(r#"{"result":"MQ=="}"#, success_json.as_str());

        let fail_json = String::from_utf8(to_vec(&fail).unwrap()).unwrap();
        assert_eq!(r#"{"error":"bad coin"}"#, fail_json.as_str());
    }

    #[test]
    fn check_packet_json() {
        let packet = Ics20Packet::new(
            Uint128::new(12345),
            "ucosm",
            "cosmos1zedxv25ah8fksmg2lzrndrpkvsjqgk4zt5ff7n",
            "wasm1fucynrfkrt684pm8jrt8la5h2csvs5cnldcgqc",
            None,
        );
        // Example message generated from the SDK
        let expected = r#"{"amount":"12345","denom":"ucosm","receiver":"wasm1fucynrfkrt684pm8jrt8la5h2csvs5cnldcgqc","sender":"cosmos1zedxv25ah8fksmg2lzrndrpkvsjqgk4zt5ff7n"}"#;

        let encoded = String::from_utf8(to_vec(&packet).unwrap()).unwrap();
        assert_eq!(expected, encoded.as_str());
    }

    #[test]
    fn check_swap_packet_json() {
        let swap_packet = SwapPacket {
            routes: vec![SwapAmountInRoute {
                token_out_denom: "ibc/AAAAAFFF".to_string(),
                pool_id: Uint64::new(1),
            }],
            token_out_min_amount: Uint128::new(1),
        };
        let packet = Ics20Packet::new(
            Uint128::new(1000),
            "uosmo",
            "cosmos1zedxv25ah8fksmg2lzrndrpkvsjqgk4zt5ff7n",
            "wasm1fucynrfkrt684pm8jrt8la5h2csvs5cnldcgqc",
            Some(OsmoPacket::Swap(swap_packet)),
        );
        // Example message generated from the SDK
        let expected = r#"{"amount":"1000","denom":"uosmo","receiver":"wasm1fucynrfkrt684pm8jrt8la5h2csvs5cnldcgqc","sender":"cosmos1zedxv25ah8fksmg2lzrndrpkvsjqgk4zt5ff7n","action":{"swap":{"routes":[{"pool_id":"1","token_out_denom":"ibc/AAAAAFFF"}],"token_out_min_amount":"1"}}}"#;

        let encdoded = String::from_utf8(to_vec(&packet).unwrap()).unwrap();
        assert_eq!(expected, encdoded.as_str());
    }

    fn cw20_payment(
        amount: u128,
        address: &str,
        recipient: &str,
        gas_limit: Option<u64>,
    ) -> SubMsg {
        let msg = Cw20ExecuteMsg::Transfer {
            recipient: recipient.into(),
            amount: Uint128::new(amount),
        };
        let exec = WasmMsg::Execute {
            contract_addr: address.into(),
            msg: to_binary(&msg).unwrap(),
            funds: vec![],
        };
        let mut msg = SubMsg::reply_on_error(exec, RECEIVE_ID);
        msg.gas_limit = gas_limit;
        msg
    }

    fn native_payment(amount: u128, denom: &str, recipient: &str) -> SubMsg {
        SubMsg::reply_on_error(
            BankMsg::Send {
                to_address: recipient.into(),
                amount: coins(amount, denom),
            },
            RECEIVE_ID,
        )
    }

    fn mock_receive_packet(
        my_channel: &str,
        amount: u128,
        denom: &str,
        receiver: &str,
    ) -> IbcPacket {
        let data = Ics20Packet {
            // this is returning a foreign (our) token, thus denom is <port>/<channel>/<denom>
            denom: format!("{}/{}/{}", REMOTE_PORT, "channel-1234", denom),
            amount: amount.into(),
            sender: "remote-sender".to_string(),
            receiver: receiver.to_string(),
            action: None,
        };
        print!("Packet denom: {}", &data.denom);
        IbcPacket::new(
            to_binary(&data).unwrap(),
            IbcEndpoint {
                port_id: REMOTE_PORT.to_string(),
                channel_id: "channel-1234".to_string(),
            },
            IbcEndpoint {
                port_id: CONTRACT_PORT.to_string(),
                channel_id: my_channel.to_string(),
            },
            3,
            Timestamp::from_seconds(1665321069).into(),
        )
    }

    #[test]
    fn send_receive_cw20() {
        let send_channel = "channel-9";
        let cw20_addr = "token-addr";
        let cw20_denom = "cw20:token-addr";
        let gas_limit = 1234567;
        let mut deps = setup(
            // &["channel-1", "channel-7", send_channel], // TODO: Allow multiple channels.
            &[send_channel],
            &[(cw20_addr, gas_limit)],
        );

        // prepare some mock packets
        let recv_packet = mock_receive_packet(send_channel, 876543210, cw20_denom, "local-rcpt");
        let recv_high_packet =
            mock_receive_packet(send_channel, 1876543210, cw20_denom, "local-rcpt");

        // cannot receive this denom yet
        let msg = IbcPacketReceiveMsg::new(recv_packet.clone());
        let res = ibc_packet_receive(deps.as_mut(), mock_env(), msg).unwrap();
        assert!(res.messages.is_empty());
        let ack: Ics20Ack = from_binary(&res.acknowledgement).unwrap();
        let no_funds = Ics20Ack::Error(ContractError::InsufficientFunds {}.to_string());
        assert_eq!(ack, no_funds);

        // we send some cw20 tokens over
        let transfer = ExecuteMsg::Transfer(TransferMsg {
            channel: send_channel.to_string(),
            remote_address: "remote-rcpt".to_string(),
            timeout: None,
        });
        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "local-sender".to_string(),
            amount: Uint128::new(987654321),
            msg: to_binary(&transfer).unwrap(),
        });
        let info = mock_info(cw20_addr, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(1, res.messages.len());
        let expected = Ics20Packet {
            denom: cw20_denom.into(),
            amount: Uint128::new(987654321),
            sender: "local-sender".to_string(),
            receiver: "remote-rcpt".to_string(),
            action: None,
        };
        let timeout = mock_env().block.time.plus_seconds(DEFAULT_TIMEOUT);
        assert_eq!(
            &res.messages[0],
            &SubMsg::new(IbcMsg::SendPacket {
                channel_id: send_channel.to_string(),
                data: to_binary(&expected).unwrap(),
                timeout: IbcTimeout::with_timestamp(timeout),
            })
        );

        // query channel state|_|
        let state = query_channel(deps.as_ref(), send_channel.to_string()).unwrap();
        assert_eq!(state.balances, vec![Amount::cw20(987654321, cw20_addr)]);
        assert_eq!(state.total_sent, vec![Amount::cw20(987654321, cw20_addr)]);

        // cannot receive more than we sent
        let msg = IbcPacketReceiveMsg::new(recv_high_packet);
        let res = ibc_packet_receive(deps.as_mut(), mock_env(), msg).unwrap();
        assert!(res.messages.is_empty());
        let ack: Ics20Ack = from_binary(&res.acknowledgement).unwrap();
        assert_eq!(ack, no_funds);

        // we can receive less than we sent
        let msg = IbcPacketReceiveMsg::new(recv_packet);
        let res = ibc_packet_receive(deps.as_mut(), mock_env(), msg).unwrap();
        assert_eq!(1, res.messages.len());
        assert_eq!(
            cw20_payment(876543210, cw20_addr, "local-rcpt", Some(gas_limit)),
            res.messages[0]
        );
        let ack: Ics20Ack = from_binary(&res.acknowledgement).unwrap();
        matches!(ack, Ics20Ack::Result(_));

        // TODO: we need to call the reply block

        // query channel state
        let state = query_channel(deps.as_ref(), send_channel.to_string()).unwrap();
        assert_eq!(state.balances, vec![Amount::cw20(111111111, cw20_addr)]);
        assert_eq!(state.total_sent, vec![Amount::cw20(987654321, cw20_addr)]);
    }

    #[test]
    fn send_receive_native() {
        let send_channel = "channel-9";
        let mut deps = setup(&[send_channel], &[]); // TODO: Allow multiple channels

        let denom = "uatom";

        // prepare some mock packets
        let recv_packet = mock_receive_packet(send_channel, 876543210, denom, "local-rcpt");
        let recv_high_packet = mock_receive_packet(send_channel, 1876543210, denom, "local-rcpt");

        // cannot receive this denom yet
        let msg = IbcPacketReceiveMsg::new(recv_packet.clone());
        let res = ibc_packet_receive(deps.as_mut(), mock_env(), msg).unwrap();
        assert!(res.messages.is_empty());
        let ack: Ics20Ack = from_binary(&res.acknowledgement).unwrap();
        let no_funds = Ics20Ack::Error(ContractError::InsufficientFunds {}.to_string());
        assert_eq!(ack, no_funds);

        // we transfer some tokens
        let msg = ExecuteMsg::Transfer(TransferMsg {
            channel: send_channel.to_string(),
            remote_address: "my-remote-address".to_string(),
            timeout: None,
        });
        let info = mock_info("local-sender", &coins(987654321, denom));
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // query channel state|_|
        let state = query_channel(deps.as_ref(), send_channel.to_string()).unwrap();
        assert_eq!(state.balances, vec![Amount::native(987654321, denom)]);
        assert_eq!(state.total_sent, vec![Amount::native(987654321, denom)]);

        // cannot receive more than we sent
        let msg = IbcPacketReceiveMsg::new(recv_high_packet);
        let res = ibc_packet_receive(deps.as_mut(), mock_env(), msg).unwrap();
        assert!(res.messages.is_empty());
        let ack: Ics20Ack = from_binary(&res.acknowledgement).unwrap();
        assert_eq!(ack, no_funds);

        // we can receive less than we sent
        let msg = IbcPacketReceiveMsg::new(recv_packet);
        let res = ibc_packet_receive(deps.as_mut(), mock_env(), msg).unwrap();
        assert_eq!(1, res.messages.len());
        assert_eq!(
            native_payment(876543210, denom, "local-rcpt"),
            res.messages[0]
        );
        let ack: Ics20Ack = from_binary(&res.acknowledgement).unwrap();
        matches!(ack, Ics20Ack::Result(_));

        // only need to call reply block on error case

        // query channel state
        let state = query_channel(deps.as_ref(), send_channel.to_string()).unwrap();
        assert_eq!(state.balances, vec![Amount::native(111111111, denom)]);
        assert_eq!(state.total_sent, vec![Amount::native(987654321, denom)]);
    }
}
