use cosmwasm_std::{Decimal, StdResult, Storage, Uint128};
use cw20_base::state::balances;

use crate::state::{tax_receiver_read, TAX_RATE};

pub const DECIMAL_FRACTION: Uint128 = Uint128(1_000_000_000_000_000_000u128);

pub fn compute_tax(amount: Uint128) -> StdResult<Uint128> {
    // get oracle params from oracle contract
    let new_amount = amount.multiply_ratio(
        DECIMAL_FRACTION,
        DECIMAL_FRACTION * Decimal::from_ratio(TAX_RATE, 100u128) + DECIMAL_FRACTION,
    );
    if new_amount.is_zero() {
        return Ok(amount);
    }
    Ok(new_amount)
}

pub fn handle_tax(storage: &mut dyn Storage, amount: Uint128) -> StdResult<Uint128> {
    // get new amount after deduct tax
    let new_amount = compute_tax(amount)?;
    let tax_receiver = tax_receiver_read(storage).load()?;
    let mut accounts = balances(storage);
    // increment tax amount to receiver wallet
    accounts.update(&tax_receiver, |balance: Option<Uint128>| -> StdResult<_> {
        balance.unwrap_or_default() + amount - new_amount
    })?;
    Ok(new_amount)
}

#[cfg(test)]
mod tests {

    use crate::{
        contract::{handle, init},
        msg::InitMsg,
        tax::compute_tax,
    };
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env, mock_info},
        HumanAddr, Uint128,
    };
    use cw20::{Cw20CoinHuman, MinterResponse};
    use cw20_base::{contract::query_balance, msg::HandleMsg};

    #[test]
    fn test_compute_tax() {
        let amount = Uint128(100u128);
        let new_amount = compute_tax(amount).unwrap();
        assert_eq!(new_amount, Uint128(95u128));
    }

    #[test]
    fn test_handle_tax() {
        let mut deps = mock_dependencies(&[]);
        let amount = Uint128(11223344);
        let minter = HumanAddr::from("minter");
        let tax_receiver = HumanAddr::from("tax_receiver");
        let receiver = HumanAddr::from("receiver");
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
        };
        let info = mock_info(&HumanAddr("creator".to_string()), &[]);
        let env = mock_env();
        init(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();

        let info = mock_info(minter.clone(), &[]);
        let env = mock_env();

        // case amount is too small, then we dont charge tax
        let msg = HandleMsg::Transfer {
            recipient: receiver.clone(),
            amount: Uint128(1u128),
        };
        handle(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // tax receiver address should receive 5 CASH because of tax
        let balance = query_balance(deps.as_ref(), tax_receiver.clone())
            .unwrap()
            .balance;
        assert_eq!(balance, Uint128(0u128));

        let msg = HandleMsg::Transfer {
            recipient: receiver.clone(),
            amount: Uint128(100u128),
        };
        handle(deps.as_mut(), env, info, msg).unwrap();

        // tax receiver address should receive 5 CASH because of tax
        let balance = query_balance(deps.as_ref(), tax_receiver).unwrap().balance;
        assert_eq!(balance, Uint128(5u128));
    }
}
