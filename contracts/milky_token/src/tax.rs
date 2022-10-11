use cosmwasm_std::{CanonicalAddr, StdError, StdResult, Storage, Uint128};
use cw20_base::state::balances;
use oraiswap::error::OverflowError;

use crate::state::TAX_RATE;

pub fn checked_sub(left: Uint128, right: Uint128) -> StdResult<Uint128> {
    left.0.checked_sub(right.0).map(Uint128).ok_or_else(|| {
        StdError::generic_err(
            OverflowError {
                operation: oraiswap::error::OverflowOperation::Sub,
                operand1: left.to_string(),
                operand2: right.to_string(),
            }
            .to_string(),
        )
    })
}

pub fn compute_tax(amount: Uint128) -> StdResult<Uint128> {
    // get oracle params from oracle contract
    let new_amount = checked_sub(amount, amount.multiply_ratio(TAX_RATE, Uint128(100u128)))?;
    if new_amount.is_zero() {
        return Ok(amount);
    }
    Ok(new_amount)
}

pub fn handle_tax(
    storage: &mut dyn Storage,
    tax_receiver: CanonicalAddr,
    amount: Uint128,
) -> StdResult<Uint128> {
    // get new amount after deduct tax
    let new_amount = compute_tax(amount)?;
    let mut accounts = balances(storage);

    // increment tax amount to receiver wallet
    accounts.update(&tax_receiver, |balance: Option<Uint128>| -> StdResult<_> {
        balance.unwrap_or_default() + amount - new_amount
    })?;
    Ok(new_amount)
}

#[cfg(test)]
mod tests {

    use crate::contract::new_query_token_info;
    use crate::msg::HandleMsg;
    use crate::{
        contract::{handle, init},
        mock_querier::mock_dependencies,
        msg::InitMsg,
        tax::compute_tax,
    };
    use cosmwasm_std::{
        testing::{mock_env, mock_info},
        HumanAddr, Uint128,
    };
    use cw20::{Cw20CoinHuman, MinterResponse};
    use cw20_base::contract::query_balance;
    use cw20_base::ContractError;

    #[test]
    fn test_compute_tax() {
        let amount = Uint128(1000u128);
        let new_amount = compute_tax(amount).unwrap();
        assert_eq!(new_amount, Uint128(950u128));
    }

    #[test]
    fn test_handle_tax() {
        let mut deps = mock_dependencies(&[]);
        let amount = Uint128(11223344);
        let minter = HumanAddr::from("minter");
        let tax_receiver = HumanAddr::from("tax_receiver");
        let random_contract = HumanAddr::from("random_contract");
        let router_contract = HumanAddr::from("router_contract");
        let limit = Uint128(511223344);
        let init_msg = InitMsg {
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: vec![Cw20CoinHuman {
                address: minter.clone(),
                amount,
            }],
            mint: Some(MinterResponse {
                minter: minter.clone(),
                cap: Some(limit),
            }),
            init_hook: None,
            tax_receiver: tax_receiver.clone(),
            router_contract: HumanAddr::from("router_contract"),
        };
        let info = mock_info(&HumanAddr("creator".to_string()), &[]);
        let env = mock_env();
        init(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();

        let info = mock_info(minter.clone(), &[]);
        let env = mock_env();

        // // init some amount to smart contract swap, it must have access to balance by user approvals
        // let msg = HandleMsg::Transfer {
        //     recipient: tax_receiver.clone(),
        //     amount: Uint128(1000u128),
        // };
        // handle(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // case amount is too small, then we dont charge tax
        let msg = HandleMsg::Send {
            contract: router_contract.clone(),
            amount: Uint128(1u128),
            msg: None,
        };
        handle(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let balance = query_balance(deps.as_ref(), tax_receiver.clone())
            .unwrap()
            .balance;
        assert_eq!(balance, Uint128(1000u128));

        let msg = HandleMsg::Send {
            contract: router_contract.clone(),
            amount: Uint128(1000u128),
            msg: None,
        };
        handle(
            deps.as_mut(),
            env.clone(),
            mock_info(minter.clone(), &[]),
            msg,
        )
        .unwrap();

        // tax receiver address should receive 50 CASH because of tax
        let balance = query_balance(deps.as_ref(), tax_receiver.clone())
            .unwrap()
            .balance;
        assert_eq!(balance, Uint128(50u128));

        // when send to other contract => tax doesnt increase
        let msg = HandleMsg::Send {
            contract: random_contract.clone(),
            amount: Uint128(1000u128),
            msg: None,
        };
        handle(deps.as_mut(), env, mock_info(minter.clone(), &[]), msg).unwrap();

        // still the same amount
        let balance = query_balance(deps.as_ref(), tax_receiver).unwrap().balance;
        assert_eq!(balance, Uint128(50u128));
    }

    #[test]
    fn test_change_tax_info() {
        let mut deps = mock_dependencies(&[]);
        let amount = Uint128(11223344);
        let minter = HumanAddr::from("minter");
        let tax_receiver = HumanAddr::from("tax_receiver");
        let limit = Uint128(511223344);
        let init_msg = InitMsg {
            name: "Cash Token".to_string(),
            symbol: "CASH".to_string(),
            decimals: 9,
            initial_balances: vec![Cw20CoinHuman {
                address: minter.clone(),
                amount,
            }],
            mint: Some(MinterResponse {
                minter: minter.clone(),
                cap: Some(limit),
            }),
            init_hook: None,
            tax_receiver: tax_receiver.clone(),
            router_contract: HumanAddr::from("router_contract"),
        };
        let info = mock_info(&HumanAddr("creator".to_string()), &[]);
        let env = mock_env();
        init(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();

        let info = mock_info(minter.clone(), &[]);
        let env = mock_env();

        let msg = HandleMsg::ChangeTaxInfo {
            new_tax_receiver: Some(HumanAddr("foobar".to_string())),
            new_router_contract: Some(HumanAddr("new_router".to_string())),
        };

        // unauthorized case

        let res = handle(
            deps.as_mut(),
            env.clone(),
            mock_info(HumanAddr::from("hacker"), &[]),
            msg.clone(),
        );
        match res.unwrap_err() {
            ContractError::Unauthorized { .. } => {}
            e => panic!("expected unauthorized error, got {}", e),
        }

        // valid case

        handle(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // query new tax receiver & new router contract
        let result = new_query_token_info(deps.as_ref()).unwrap();
        assert_eq!(result.router_contract, HumanAddr::from("new_router"));
        assert_eq!(result.tax_receiver, HumanAddr::from("foobar"))
    }
}
