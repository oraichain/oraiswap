use crate::contract::*;
use cosmwasm_std::testing::{
    mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{coins, from_binary, Decimal, OwnedDeps, StdError};
use oraiswap::asset::ORAI_DENOM;
use oraiswap::oracle::{
    ExchangeRateResponse, InitMsg, OracleExchangeMsg, OracleExchangeQuery, OracleMsg, OracleQuery,
    OracleTreasuryQuery,
};

const OWNER: &str = "owner0000";

fn setup_contract() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    let mut deps = mock_dependencies(&coins(100000, ORAI_DENOM));

    let msg = InitMsg {
        name: None,
        version: None,
        admin: None,
        min_rate: None,
        max_rate: None,
    };
    let info = mock_info(OWNER, &[]);
    let res = init(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    deps
}

#[test]
fn proper_initialization() {
    let mut deps = setup_contract();

    let msg = OracleMsg::Exchange(OracleExchangeMsg::UpdateExchangeRate {
        denom: "usdt".to_string(),
        exchange_rate: Decimal::percent(10), // 1 orai = 10 usdt
    });

    let _res = handle(deps.as_mut(), mock_env(), mock_info(OWNER, &[]), msg).unwrap();

    let msg = OracleQuery::Exchange(OracleExchangeQuery::ExchangeRate {
        base_denom: Some("usdt".to_string()),
        quote_denom: ORAI_DENOM.to_string(),
    });

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let exchange_rate_res: ExchangeRateResponse = from_binary(&res).unwrap();

    assert_eq!("10", exchange_rate_res.item.exchange_rate.to_string());
}

#[test]
fn tax_cap_notfound() {
    let deps = setup_contract();

    let msg = OracleQuery::Treasury(OracleTreasuryQuery::TaxCap {
        denom: "airi".to_string(),
    });

    let res = query(deps.as_ref(), mock_env(), msg);
    match res {
        Err(StdError::NotFound { kind }) => {
            assert_eq!(kind, format!("Tax cap not found for denom: {}", "airi"))
        }
        _ => panic!("DO NOT ENTER HERE"),
    }
}
