use crate::contract::*;
use crate::error::ContractError;
use crate::msg::*;
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{
    attr, coins, from_binary, from_slice, to_binary, Api, CosmosMsg, HandleResponse, HumanAddr,
    OwnedDeps, WasmMsg,
};

use cw721::{
    ApprovedForAllResponse, ContractInfoResponse, Cw721ReceiveMsg, Expiration, NftInfoResponse,
    NumTokensResponse, OwnerOfResponse, TokensResponse,
};

const MINTER: &str = "orai1up8ct7kk2hr6x9l37ev6nfgrtqs268tdrevk3d";
const OWNER: &str = "owner";
const CONTRACT_NAME: &str = "Magic Power";
const CONTRACT_VERSION: &str = "0.1.1";
const SYMBOL: &str = "MGK";

fn setup_contract() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies(&coins(100000, "orai"));
    deps.api.canonical_length = 54;
    let msg = InitMsg {
        name: Some(CONTRACT_NAME.to_string()),
        symbol: SYMBOL.to_string(),
        minter: MINTER.into(),
        version: Some(CONTRACT_VERSION.to_string()),
    };
    let info = mock_info(OWNER, &[]);
    let res = init(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    deps
}

#[test]
fn proper_initialization() {
    let deps = setup_contract();

    // it worked, let's query the state
    let res: MinterResponse =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Minter {}).unwrap()).unwrap();
    assert_eq!(MINTER, res.minter.as_str());
    let info: ContractInfoResponse =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::ContractInfo {}).unwrap()).unwrap();
    assert_eq!(
        info,
        ContractInfoResponse {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            version: CONTRACT_VERSION.to_string(),
        }
    );

    let count: NumTokensResponse =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::NumTokens {}).unwrap()).unwrap();
    assert_eq!(0, count.count);

    // list the token_ids
    let tokens: TokensResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::AllTokens {
                start_after: None,
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(0, tokens.tokens.len());
}

#[test]
fn minting() {
    let mut deps = setup_contract();

    let token_id = "petrify".to_string();
    let name = "Petrify with Gaze".to_string();
    let description = "Allows the owner to petrify anyone looking at him or her".to_string();
    let image = "".to_string();
    let owner = "orai1up8ct7kk2hr6x9l37ev6nfgrtqs268tdrevk3t".to_string();
    let mint_str = format!(
            "{{\"token_id\":\"{}\",\"owner\":\"{}\",\"name\":\"{}\",\"description\":\"{}\",\"image\":\"{}\"
    }}",
    token_id, owner, name, description,image
        );
    println!("length count: {}", owner.len());
    let mint_msg: MintMsg = from_slice(mint_str.as_bytes()).unwrap();
    println!(
        "mint msg: {}",
        deps.api.canonical_address(&mint_msg.owner).unwrap()
    );

    let mint_msg = HandleMsg::Mint(mint_msg);

    // random cannot mint
    let random = mock_info("random", &[]);
    let err = handle(deps.as_mut(), mock_env(), random, mint_msg.clone()).unwrap_err();
    match err {
        ContractError::Unauthorized {} => {}
        e => panic!("unexpected error: {}", e),
    }

    // minter can mint
    let allowed = mock_info(MINTER, &[]);
    let _ = handle(deps.as_mut(), mock_env(), allowed, mint_msg.clone()).unwrap();

    // ensure num tokens increases
    let count: NumTokensResponse =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::NumTokens {}).unwrap()).unwrap();
    assert_eq!(1, count.count);

    // unknown nft returns error
    let _ = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::NftInfo {
            token_id: "unknown".to_string(),
        },
    )
    .unwrap_err();

    // this nft info is correct
    let info: NftInfoResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::NftInfo {
                token_id: token_id.clone(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        info,
        NftInfoResponse {
            name: name.clone(),
            description: description.clone(),
            image: "".to_string(),
        }
    );

    // Cannot mint same token_id again
    let mint_msg2 = HandleMsg::Mint(MintMsg {
        token_id: token_id.clone(),
        owner: "hercules".into(),
        name: "copy cat".into(),
        description: None,
        image: "".to_string(),
    });

    let allowed = mock_info(MINTER, &[]);
    let err = handle(deps.as_mut(), mock_env(), allowed, mint_msg2).unwrap_err();
    match err {
        ContractError::Claimed {} => {}
        e => panic!("unexpected error: {}", e),
    }

    // list the token_ids
    let tokens: TokensResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::AllTokens {
                start_after: None,
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(1, tokens.tokens.len());
    assert_eq!(vec![token_id], tokens.tokens);
}

#[test]
fn transferring_nft() {
    let mut deps = setup_contract();

    // Mint a token
    let token_id = "melt".to_string();
    let name = "Melting power".to_string();
    let description = "Allows the owner to melt anyone looking at him or her".to_string();

    let mint_msg = HandleMsg::Mint(MintMsg {
        token_id: token_id.clone(),
        owner: "venus".into(),
        name: name.clone(),
        description: Some(description.clone()),
        image: "".to_string(),
    });

    let minter = mock_info(MINTER, &[]);
    handle(deps.as_mut(), mock_env(), minter, mint_msg).unwrap();

    // random cannot transfer
    let random = mock_info("random", &[]);
    let transfer_msg = HandleMsg::TransferNft {
        recipient: "random".into(),
        token_id: token_id.clone(),
    };

    let err = handle(deps.as_mut(), mock_env(), random, transfer_msg.clone()).unwrap_err();

    match err {
        ContractError::Unauthorized {} => {}
        e => panic!("unexpected error: {}", e),
    }

    // owner can
    let random = mock_info("venus", &[]);
    let transfer_msg = HandleMsg::TransferNft {
        recipient: "random".into(),
        token_id: token_id.clone(),
    };

    let res = handle(deps.as_mut(), mock_env(), random, transfer_msg.clone()).unwrap();

    assert_eq!(
        res,
        HandleResponse {
            messages: vec![],
            attributes: vec![
                attr("action", "transfer_nft"),
                attr("sender", "venus"),
                attr("recipient", "random"),
                attr("token_id", token_id),
            ],
            data: None,
        }
    );
}

#[test]
fn test_owner_rights() {
    let mut deps = setup_contract();

    // Mint a token
    let token_id = "melt".to_string();
    let name = "Melting power".to_string();
    let description = "Allows the owner to melt anyone looking at him or her".to_string();

    let mint_msg = HandleMsg::Mint(MintMsg {
        token_id: token_id.clone(),
        owner: "venus".into(),
        name: name.clone(),
        description: Some(description.clone()),
        image: "".to_string(),
    });

    let minter = mock_info(MINTER, &[]);
    handle(deps.as_mut(), mock_env(), minter, mint_msg).unwrap();

    // owner of the smart contract can transfer
    let random = mock_info(OWNER, &[]);
    let transfer_msg = HandleMsg::TransferNft {
        recipient: "random".into(),
        token_id: token_id.clone(),
    };

    let _ = handle(
        deps.as_mut(),
        mock_env(),
        random.clone(),
        transfer_msg.clone(),
    )
    .unwrap();

    // owner can also approve the nft
    let approve_msg = HandleMsg::Approve {
        spender: HumanAddr::from("some random guy"),
        token_id: "melt".to_string(),
        expires: None,
    };
    handle(
        deps.as_mut(),
        mock_env(),
        random.clone(),
        approve_msg.clone(),
    )
    .unwrap();

    // can also revoke the nft
    let revoke_msg = HandleMsg::Revoke {
        spender: HumanAddr::from("some random guy"),
        token_id: "melt".to_string(),
    };
    handle(
        deps.as_mut(),
        mock_env(),
        random.clone(),
        revoke_msg.clone(),
    )
    .unwrap();

    // burn the nft
    let burn_msg = HandleMsg::Burn {
        token_id: "melt".to_string(),
    };

    // random dude cannot burn
    assert!(matches!(
        handle(
            deps.as_mut(),
            mock_env(),
            mock_info("random guy", &[]),
            burn_msg.clone()
        ),
        Err(ContractError::Unauthorized {})
    ));

    handle(deps.as_mut(), mock_env(), random.clone(), burn_msg.clone()).unwrap();
    // should be burnt

    let is_err = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::NftInfo {
            token_id: "melt".to_string(),
        },
    )
    .is_err();

    assert_eq!(is_err, true);
}

#[test]
fn sending_nft() {
    let mut deps = setup_contract();

    // Mint a token
    let token_id = "melt".to_string();
    let name = "Melting power".to_string();
    let description = "Allows the owner to melt anyone looking at him or her".to_string();

    let mint_msg = HandleMsg::Mint(MintMsg {
        token_id: token_id.clone(),
        owner: "venus".into(),
        name: name.clone(),
        description: Some(description.clone()),
        image: "".to_string(),
    });

    let minter = mock_info(MINTER, &[]);
    handle(deps.as_mut(), mock_env(), minter, mint_msg).unwrap();

    let msg = to_binary("You now have the melting power").unwrap();
    let target = HumanAddr::from("another_contract");
    let send_msg = HandleMsg::SendNft {
        contract: target.clone(),
        token_id: token_id.clone(),
        msg: Some(msg.clone()),
    };

    let random = mock_info("random", &[]);
    let err = handle(deps.as_mut(), mock_env(), random, send_msg.clone()).unwrap_err();
    match err {
        ContractError::Unauthorized {} => {}
        e => panic!("unexpected error: {}", e),
    }

    // but owner can
    let random = mock_info("venus", &[]);
    let res = handle(deps.as_mut(), mock_env(), random, send_msg).unwrap();

    let payload = Cw721ReceiveMsg {
        sender: "venus".into(),
        token_id: token_id.clone(),
        msg: Some(msg),
    };
    let expected = payload.into_cosmos_msg(target.clone()).unwrap();
    // ensure expected serializes as we think it should
    match &expected {
        CosmosMsg::Wasm(WasmMsg::Execute { contract_addr, .. }) => {
            assert_eq!(contract_addr, &target)
        }
        m => panic!("Unexpected message type: {:?}", m),
    }
    // and make sure this is the request sent by the contract
    assert_eq!(
        res,
        HandleResponse {
            messages: vec![expected],
            attributes: vec![
                attr("action", "send_nft"),
                attr("sender", "venus"),
                attr("recipient", "another_contract"),
                attr("token_id", token_id),
            ],
            data: None,
        }
    );
}

#[test]
fn approving_revoking() {
    let mut deps = setup_contract();

    // Mint a token
    let token_id = "grow".to_string();
    let name = "Growing power".to_string();
    let description = "Allows the owner to grow anything".to_string();

    let mint_msg = HandleMsg::Mint(MintMsg {
        token_id: token_id.clone(),
        owner: "demeter".into(),
        name: name.clone(),
        description: Some(description.clone()),
        image: "".to_string(),
    });

    let minter = mock_info(MINTER, &[]);
    handle(deps.as_mut(), mock_env(), minter, mint_msg).unwrap();

    // Give random transferring power
    let approve_msg = HandleMsg::Approve {
        spender: "random".into(),
        token_id: token_id.clone(),
        expires: None,
    };
    let owner = mock_info("demeter", &[]);
    let res = handle(deps.as_mut(), mock_env(), owner, approve_msg).unwrap();
    assert_eq!(
        res,
        HandleResponse {
            messages: vec![],
            attributes: vec![
                attr("action", "approve"),
                attr("sender", "demeter"),
                attr("spender", "random"),
                attr("token_id", token_id.clone()),
            ],
            data: None,
        }
    );

    // random can now transfer
    let random = mock_info("random", &[]);
    let transfer_msg = HandleMsg::TransferNft {
        recipient: "person".into(),
        token_id: token_id.clone(),
    };
    handle(deps.as_mut(), mock_env(), random, transfer_msg).unwrap();

    // Approvals are removed / cleared
    let query_msg = QueryMsg::OwnerOf {
        token_id: token_id.clone(),
        include_expired: None,
    };
    let res: OwnerOfResponse =
        from_binary(&query(deps.as_ref(), mock_env(), query_msg.clone()).unwrap()).unwrap();
    assert_eq!(
        res,
        OwnerOfResponse {
            owner: "person".into(),
            approvals: vec![],
        }
    );

    // Approve, revoke, and check for empty, to test revoke
    let approve_msg = HandleMsg::Approve {
        spender: "random".into(),
        token_id: token_id.clone(),
        expires: None,
    };
    let owner = mock_info("person", &[]);
    handle(deps.as_mut(), mock_env(), owner.clone(), approve_msg).unwrap();

    let revoke_msg = HandleMsg::Revoke {
        spender: "random".into(),
        token_id: token_id.clone(),
    };
    handle(deps.as_mut(), mock_env(), owner, revoke_msg).unwrap();

    // Approvals are now removed / cleared
    let res: OwnerOfResponse =
        from_binary(&query(deps.as_ref(), mock_env(), query_msg).unwrap()).unwrap();
    assert_eq!(
        res,
        OwnerOfResponse {
            owner: "person".into(),
            approvals: vec![],
        }
    );
}

#[test]
fn approving_all_revoking_all() {
    let mut deps = setup_contract();

    // Mint a couple tokens (from the same owner)
    let token_id1 = "grow1".to_string();
    let name1 = "Growing power".to_string();
    let description1 = "Allows the owner the power to grow anything".to_string();
    let token_id2 = "grow2".to_string();
    let name2 = "More growing power".to_string();
    let description2 = "Allows the owner the power to grow anything even faster".to_string();

    let mint_msg1 = HandleMsg::Mint(MintMsg {
        token_id: token_id1.clone(),
        owner: "demeter".into(),
        name: name1.clone(),
        description: Some(description1.clone()),
        image: "".to_string(),
    });

    let minter = mock_info(MINTER, &[]);
    handle(deps.as_mut(), mock_env(), minter.clone(), mint_msg1).unwrap();

    let mint_msg2 = HandleMsg::Mint(MintMsg {
        token_id: token_id2.clone(),
        owner: "demeter".into(),
        name: name2.clone(),
        description: Some(description2.clone()),
        image: "".to_string(),
    });

    handle(deps.as_mut(), mock_env(), minter, mint_msg2).unwrap();

    // paginate the token_ids
    let tokens: TokensResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::AllTokens {
                start_after: None,
                limit: Some(1),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(1, tokens.tokens.len());
    assert_eq!(vec![token_id1.clone()], tokens.tokens);
    let tokens: TokensResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::AllTokens {
                start_after: Some(token_id1.clone()),
                limit: Some(3),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(1, tokens.tokens.len());
    assert_eq!(vec![token_id2.clone()], tokens.tokens);

    // demeter gives random full (operator) power over her tokens
    let approve_all_msg = HandleMsg::ApproveAll {
        operator: "random".into(),
        expires: None,
    };
    let owner = mock_info("demeter", &[]);
    let res = handle(deps.as_mut(), mock_env(), owner, approve_all_msg).unwrap();
    assert_eq!(
        res,
        HandleResponse {
            messages: vec![],
            attributes: vec![
                attr("action", "approve_all"),
                attr("sender", "demeter"),
                attr("operator", "random"),
            ],
            data: None,
        }
    );

    // random can now transfer
    let random = mock_info("random", &[]);
    let transfer_msg = HandleMsg::TransferNft {
        recipient: "person".into(),
        token_id: token_id1.clone(),
    };
    handle(deps.as_mut(), mock_env(), random.clone(), transfer_msg).unwrap();

    // random can now send
    let inner_msg = WasmMsg::Execute {
        contract_addr: "another_contract".into(),
        msg: to_binary("You now also have the growing power").unwrap(),
        send: vec![],
    };
    let msg: CosmosMsg = CosmosMsg::Wasm(inner_msg);

    let send_msg = HandleMsg::SendNft {
        contract: "another_contract".into(),
        token_id: token_id2.clone(),
        msg: Some(to_binary(&msg).unwrap()),
    };
    handle(deps.as_mut(), mock_env(), random, send_msg).unwrap();

    // Approve_all, revoke_all, and check for empty, to test revoke_all
    let approve_all_msg = HandleMsg::ApproveAll {
        operator: "operator".into(),
        expires: None,
    };
    // person is now the owner of the tokens
    let owner = mock_info("person", &[]);
    handle(deps.as_mut(), mock_env(), owner.clone(), approve_all_msg).unwrap();

    let res: ApprovedForAllResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::ApprovedForAll {
                owner: "person".into(),
                include_expired: Some(true),
                start_after: None,
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(
        res,
        ApprovedForAllResponse {
            operators: vec![cw721::Approval {
                spender: "operator".into(),
                expires: Expiration::Never {}
            }]
        }
    );

    // second approval
    let buddy_expires = Expiration::AtHeight(1234567);
    let approve_all_msg = HandleMsg::ApproveAll {
        operator: "buddy".into(),
        expires: Some(buddy_expires),
    };

    handle(deps.as_mut(), mock_env(), owner.clone(), approve_all_msg).unwrap();

    // and paginate queries
    let res: ApprovedForAllResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::ApprovedForAll {
                owner: "person".into(),
                include_expired: Some(true),
                start_after: Some("operator".into()),
                limit: Some(1),
            },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(
        res,
        ApprovedForAllResponse {
            operators: vec![cw721::Approval {
                spender: "buddy".into(),
                expires: buddy_expires,
            }]
        }
    );

    let res: ApprovedForAllResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::ApprovedForAll {
                owner: "person".into(),
                include_expired: Some(true),
                start_after: None,
                limit: Some(2),
            },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(
        res,
        ApprovedForAllResponse {
            operators: vec![
                cw721::Approval {
                    spender: "operator".into(),
                    expires: Expiration::Never {}
                },
                cw721::Approval {
                    spender: "buddy".into(),
                    expires: buddy_expires,
                }
            ]
        }
    );

    let revoke_all_msg = HandleMsg::RevokeAll {
        operator: "operator".into(),
    };
    handle(deps.as_mut(), mock_env(), owner, revoke_all_msg).unwrap();

    // Approvals are removed / cleared without affecting others
    let res: ApprovedForAllResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::ApprovedForAll {
                owner: "person".into(),
                include_expired: None,
                start_after: None,
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(
        res,
        ApprovedForAllResponse {
            operators: vec![cw721::Approval {
                spender: "buddy".into(),
                expires: buddy_expires,
            }]
        }
    );

    // ensure the filter works (nothing should be here
    let mut late_env = mock_env();
    late_env.block.height = 1234568; //expired

    let res: ApprovedForAllResponse = from_binary(
        &query(
            deps.as_ref(),
            late_env,
            QueryMsg::ApprovedForAll {
                owner: "person".into(),
                include_expired: None,
                start_after: None,
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(0, res.operators.len());
}

#[test]
fn query_tokens_by_owner() {
    let mut deps = setup_contract();
    let minter = mock_info(MINTER, &[]);

    // Mint a couple tokens (from the same owner)
    let token_id1 = "grow1".to_string();
    let demeter = HumanAddr::from("Demeter");
    let token_id2 = "grow2".to_string();
    let ceres = HumanAddr::from("Ceres");
    let token_id3 = "sing".to_string();

    let mint_msg = HandleMsg::Mint(MintMsg {
        token_id: token_id1.clone(),
        owner: demeter.clone(),
        name: "Growing power".to_string(),
        description: Some("Allows the owner the power to grow anything".to_string()),
        image: "".to_string(),
    });
    handle(deps.as_mut(), mock_env(), minter.clone(), mint_msg).unwrap();

    let mint_msg = HandleMsg::Mint(MintMsg {
        token_id: token_id2.clone(),
        owner: ceres.clone(),
        name: "More growing power".to_string(),
        description: Some("Allows the owner the power to grow anything even faster".to_string()),
        image: "".to_string(),
    });
    handle(deps.as_mut(), mock_env(), minter.clone(), mint_msg).unwrap();

    let mint_msg = HandleMsg::Mint(MintMsg {
        token_id: token_id3.clone(),
        owner: demeter.clone(),
        name: "Sing a lullaby".to_string(),
        description: Some("Calm even the most excited children".to_string()),
        image: "".to_string(),
    });
    handle(deps.as_mut(), mock_env(), minter.clone(), mint_msg).unwrap();

    // get all tokens in order:
    let expected = vec![token_id1.clone(), token_id2.clone(), token_id3.clone()];
    let tokens: TokensResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::AllTokens {
                start_after: None,
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(&expected, &tokens.tokens);
    // paginate
    let tokens: TokensResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::AllTokens {
                start_after: None,
                limit: Some(2),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(&expected[..2], &tokens.tokens[..]);
    let tokens: TokensResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::AllTokens {
                start_after: Some(expected[1].clone()),
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(&expected[2..], &tokens.tokens[..]);

    // get by owner
    let by_ceres = vec![token_id2.clone()];
    let by_demeter = vec![token_id1.clone(), token_id3.clone()];
    // all tokens by owner
    let tokens: TokensResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Tokens {
                owner: demeter.clone(),
                start_after: None,
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(&by_demeter, &tokens.tokens);

    let tokens: TokensResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Tokens {
                owner: ceres.clone(),
                start_after: None,
                limit: None,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(&by_ceres, &tokens.tokens);

    // paginate for demeter
    let tokens: TokensResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Tokens {
                owner: demeter.clone(),
                start_after: None,
                limit: Some(1),
            },
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(&by_demeter[..1], &tokens.tokens[..]);
    let tokens: TokensResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Tokens {
                owner: demeter.clone(),
                start_after: Some(by_demeter[0].clone()),
                limit: Some(3),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(&by_demeter[1..], &tokens.tokens[..]);
}

#[test]
fn mint_nft_invalid_args() {
    let mut deps = setup_contract();
    let token_id = "petrify".to_string();
    let name = "Petrify with Gaze".to_string();
    let description = "Very long".repeat(200); // 1800 > 1024
    let image = "https://ipfs.io/ipfs/QmWCp5t1TLsLQyjDFa87ZAp72zYqmC7L2DsNjFdpH8bBoz".to_string();
    let owner = "orai1up8ct7kk2hr6x9l37ev6nfgrtqs268tdrevk3t".to_string();
    let mint_str = format!(
            "{{\"token_id\":\"{}\",\"owner\":\"{}\",\"name\":\"{}\",\"description\":\"{}\",\"image\":\"{}\"
    }}",
    token_id, owner, name, description,image
        );
    println!("length count: {}", owner.len());
    let mint_msg: MintMsg = from_slice(mint_str.as_bytes()).unwrap();

    let mint_msg = HandleMsg::Mint(mint_msg);
    let allowed = mock_info(MINTER, &[]);
    let err = handle(deps.as_mut(), mock_env(), allowed.clone(), mint_msg).unwrap_err();

    match err {
        ContractError::InvalidArgument { reason } => {
            assert_eq!(reason, "`description` exceeds 1024 chars");
        }
        e => panic!("unexpected error: {}", e),
    }
}

#[test]
fn update_nft() {
    let mut deps = setup_contract();

    let token_id = "petrify".to_string();
    let name = "Petrify with Gaze".to_string();
    let description = "Allows the owner to petrify anyone looking at him or her".to_string();
    let image = "https://ipfs.io/ipfs/QmWCp5t1TLsLQyjDFa87ZAp72zYqmC7L2DsNjFdpH8bBoz".to_string();
    let owner = "orai1up8ct7kk2hr6x9l37ev6nfgrtqs268tdrevk3t".to_string();
    let mint_str = format!(
            "{{\"token_id\":\"{}\",\"owner\":\"{}\",\"name\":\"{}\",\"description\":\"{}\",\"image\":\"{}\"
    }}",
    token_id, owner, name, description,image
        );
    println!("length count: {}", owner.len());
    let mint_msg: MintMsg = from_slice(mint_str.as_bytes()).unwrap();

    let mint_msg = HandleMsg::Mint(mint_msg);
    let allowed = mock_info(MINTER, &[]);
    handle(deps.as_mut(), mock_env(), allowed.clone(), mint_msg).unwrap();

    // now update
    handle(
        deps.as_mut(),
        mock_env(),
        mock_info(owner, &[]),
        HandleMsg::UpdateNft {
            token_id: token_id.clone(),
            name: "new name".to_string(),
            description: None,
            image: None,
        },
    )
    .unwrap();

    // this nft info is correct
    let info: NftInfoResponse = from_binary(
        &query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::NftInfo {
                token_id: token_id.clone(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        info,
        NftInfoResponse {
            name: "new name".to_string(),
            description: description.clone(),
            image: image.clone(),
        }
    );
}
