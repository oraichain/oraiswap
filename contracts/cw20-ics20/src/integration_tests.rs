#![cfg(test)]

use crate::ibc::{reply, Ics20Packet};
use crate::msg::{AllowMsg, InitMsg, UpdatePairMsg};
use crate::test_helpers::{CONTRACT_PORT, DEFAULT_TIMEOUT, REMOTE_PORT};

use cosmwasm_std::{to_binary, Addr, Empty, IbcEndpoint, IbcPacket, Timestamp};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use oraiswap::asset::AssetInfo;

use crate::contract::{execute, instantiate, query};
use crate::msg::ExecuteMsg;

fn mock_app() -> App {
    App::default()
}

pub fn contract_cw20_ics20_latest() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(execute, instantiate, query).with_reply(reply);
    Box::new(contract)
}

fn _mock_receive_packet(
    my_channel: &str,
    remote_channel: &str,
    amount: u128,
    native_denom: &str,
    remote_sender: &str,
    receiver: &str,
) -> IbcPacket {
    let data = Ics20Packet {
        // this is returning a foreign (our) token, thus denom is <port>/<channel>/<denom>
        denom: format!("{}/{}/{}", REMOTE_PORT, remote_channel, native_denom),
        amount: amount.into(),
        sender: remote_sender.to_string(),
        receiver: receiver.to_string(),
        memo: None,
    };
    IbcPacket::new(
        to_binary(&data).unwrap(),
        IbcEndpoint {
            port_id: REMOTE_PORT.to_string(),
            channel_id: remote_channel.to_string(),
        },
        IbcEndpoint {
            port_id: CONTRACT_PORT.to_string(),
            channel_id: my_channel.to_string(),
        },
        3,
        Timestamp::from_seconds(1665321069).into(),
    )
}

fn initialize_basic_data_for_testings() -> (App, Addr, Addr, IbcEndpoint, String, String, String, u8)
{
    let mut router = mock_app();

    let cw20_ics20_id = router.store_code(contract_cw20_ics20_latest());

    let allowlist: Vec<AllowMsg> = vec![];

    // arrange
    let addr1 = Addr::unchecked("addr1");
    let gov_cw20_ics20 = Addr::unchecked("gov");

    // ibc stuff
    let src_ibc_endpoint = IbcEndpoint {
        port_id: REMOTE_PORT.to_string(),
        channel_id: "channel-0".to_string(),
    };

    let local_channel_id = "channel-0".to_string();

    let native_denom = "orai";
    let asset_info = AssetInfo::Token {
        contract_addr: Addr::unchecked("cw20:oraifoobarhelloworld".to_string()),
    };
    let remote_decimals = 18u8;
    let asset_info_decimals = 18u8;

    let cw20_ics20_init_msg = InitMsg {
        default_gas_limit: Some(20000000u64),
        default_timeout: DEFAULT_TIMEOUT,
        gov_contract: gov_cw20_ics20.to_string(),
        allowlist,
    };

    let cw20_ics20_contract = router
        .instantiate_contract(
            cw20_ics20_id,
            gov_cw20_ics20.clone(),
            &cw20_ics20_init_msg,
            &[],
            "cw20_ics20",
            None,
        )
        .unwrap();

    // update receiver contract

    let update_allow_msg = ExecuteMsg::UpdateMappingPair(UpdatePairMsg {
        local_channel_id: local_channel_id.clone(),
        denom: native_denom.to_string(),
        asset_info: asset_info.clone(),
        remote_decimals,
        asset_info_decimals,
    });
    router
        .execute_contract(
            gov_cw20_ics20.clone(),
            cw20_ics20_contract.clone(),
            &update_allow_msg,
            &[],
        )
        .unwrap();

    (
        router,
        addr1,
        gov_cw20_ics20,
        src_ibc_endpoint,
        local_channel_id,
        native_denom.to_string(),
        asset_info.to_string(),
        remote_decimals,
    )
}

#[test]
// cw3 multisig account can control cw20 admin actions
fn initialize_valid_successful_cw20_ics20_and_receiver_contract() {
    initialize_basic_data_for_testings();
}

// #[test]
// // cw3 multisig account can control cw20 admin actions
// fn on_ibc_receive_invalid_submsg_when_calling_allow_contract_should_undo_increase_channel_balance()
// {
//     let (
//         router,
//         addr1,
//         gov_cw20_ics20,
//         src_ibc_endpoint,
//         dest_ibc_endpoint,
//         native_denom,
//         cw20_denom,
//         remote_decimals,
//         receiver_contract,
//     ) = initialize_basic_data_for_testings();

//     let amount = 1u128;
//     let remote_sender = Addr::unchecked("remote_sender");
//     let local_receiver = Addr::unchecked("local_receiver");

//     let recv_packet = mock_receive_packet(
//         &dest_ibc_endpoint.channel_id,
//         src_ibc_endpoint.channel_id.as_str(),
//         amount,
//         &native_denom,
//         remote_sender.as_str(),
//         local_receiver.as_str(),
//     );
//     let msg = IbcPacketReceiveMsg::new(recv_packet.clone());
// }
