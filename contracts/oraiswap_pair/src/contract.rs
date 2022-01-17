use crate::state::PAIR_INFO;

use cosmwasm_std::{
    attr, from_binary, to_binary, Binary, CanonicalAddr, Coin, CosmosMsg, Decimal, Deps, DepsMut,
    Env, HandleResponse, HumanAddr, InitResponse, MessageInfo, MigrateResponse, StdError,
    StdResult, Uint128, WasmMsg,
};

use cw20::{Cw20HandleMsg, Cw20ReceiveMsg, MinterResponse};
use integer_sqrt::IntegerSquareRoot;
use oraiswap::asset::{Asset, AssetInfo, PairInfo, PairInfoRaw};
use oraiswap::error::ContractError;
use oraiswap::oracle::OracleContract;
use oraiswap::pair::{
    Cw20HookMsg, HandleMsg, InitMsg, MigrateMsg, PoolResponse, QueryMsg, ReverseSimulationResponse,
    SimulationResponse,
};
use oraiswap::querier::query_supply;
use oraiswap::token::InitMsg as TokenInitMsg;
use oraiswap::{Decimal256, Uint256};
use std::str::FromStr;

/// Default commission rate == 0.3%
/// in the future need to update ?
const COMMISSION_RATE: &str = "0.003";

pub fn init(deps: DepsMut, env: Env, info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    let pair_info = &PairInfoRaw {
        creator: deps.api.canonical_address(&info.sender)?,
        // return infomation from oracle, update by multisig wallet
        oracle_addr: deps.api.canonical_address(&msg.oracle_addr)?,
        // the current contract address
        contract_addr: deps.api.canonical_address(&env.contract.address)?,
        // liquidity token address is ow20 to reward, mint and burn
        liquidity_token: CanonicalAddr::from(vec![]),
        // pair info
        asset_infos: [
            msg.asset_infos[0].to_raw(deps.api)?,
            msg.asset_infos[1].to_raw(deps.api)?,
        ],
    };

    PAIR_INFO.save(deps.storage, pair_info)?;

    Ok(InitResponse {
        // Create LP token
        // when init is done, will get the deployed address
        // to update
        messages: vec![WasmMsg::Instantiate {
            code_id: msg.token_code_id,
            msg: to_binary(&TokenInitMsg {
                name: "oraiswap liquidity token".to_string(),
                symbol: "uLP".to_string(),
                decimals: 6,
                initial_balances: vec![],
                // only this pair contract can mint
                mint: Some(MinterResponse {
                    minter: env.contract.address,
                    cap: None,
                }),
            })?,
            send: vec![],
            label: None,
        }
        .into()],
        attributes: vec![],
    })
}

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        // when liquidity token is deploy, need to update the address
        HandleMsg::Update { contract_address } => update_pair(deps, env, info, contract_address),
        // when transfer ow20 token to this contract
        HandleMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        // add more liquidity
        HandleMsg::ProvideLiquidity {
            assets,
            slippage_tolerance,
            receiver,
        } => provide_liquidity(deps, env, info, assets, slippage_tolerance, receiver),
        // swap token, can not swap native token directly
        HandleMsg::Swap {
            offer_asset,
            belief_price,
            max_spread,
            to,
        } => {
            if !offer_asset.is_native_token() {
                return Err(ContractError::Unauthorized {});
            }

            swap(
                deps,
                env,
                info.clone(),
                info.sender,
                offer_asset,
                belief_price,
                max_spread,
                to,
            )
        }
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<HandleResponse, ContractError> {
    let contract_addr = info.sender.clone();

    match from_binary(&cw20_msg.msg.unwrap_or_default()) {
        Ok(Cw20HookMsg::Swap {
            belief_price,
            max_spread,
            to,
        }) => {
            // only asset contract can execute this message
            let mut authorized: bool = false;
            let config: PairInfoRaw = PAIR_INFO.load(deps.storage)?;
            let pools: [Asset; 2] =
                config.query_pools(&deps.querier, deps.api, env.contract.address.clone())?;
            for pool in pools.iter() {
                if let AssetInfo::Token { contract_addr, .. } = &pool.info {
                    if info.sender.eq(contract_addr) {
                        authorized = true;
                        break;
                    }
                }
            }

            if !authorized {
                return Err(ContractError::Unauthorized {});
            }

            let to_addr = to.map(HumanAddr);

            swap(
                deps,
                env,
                info,
                cw20_msg.sender,
                Asset {
                    info: AssetInfo::Token { contract_addr },
                    amount: cw20_msg.amount,
                },
                belief_price,
                max_spread,
                to_addr,
            )
        }
        // remove liquidity
        Ok(Cw20HookMsg::WithdrawLiquidity {}) => {
            let config: PairInfoRaw = PAIR_INFO.load(deps.storage)?;
            if deps.api.canonical_address(&info.sender)? != config.liquidity_token {
                return Err(ContractError::Unauthorized {});
            }

            withdraw_liquidity(deps, env, info, cw20_msg.sender, cw20_msg.amount)
        }
        Err(err) => Err(ContractError::Std(err)),
    }
}

/// This just stores the result for future query, after the smart contract is instantiated
pub fn update_pair(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    contract_address: HumanAddr,
) -> Result<HandleResponse, ContractError> {
    // borrow dynamic
    let api = deps.api;
    let mut pair_info = PAIR_INFO.load(deps.storage)?;

    // only creator can update the liquidity_token address
    if pair_info.creator.ne(&api.canonical_address(&info.sender)?) {
        return Err(ContractError::Unauthorized {});
    }

    // update liquidity_token
    pair_info.liquidity_token = api.canonical_address(&contract_address)?;
    PAIR_INFO.save(deps.storage, &pair_info)?;

    Ok(HandleResponse {
        attributes: vec![attr("liquidity_token_addr", contract_address)],
        messages: vec![],
        data: None,
    })
}

/// CONTRACT - should approve contract to use the amount of token
pub fn provide_liquidity(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    assets: [Asset; 2],
    slippage_tolerance: Option<Decimal>,
    receiver: Option<HumanAddr>,
) -> Result<HandleResponse, ContractError> {
    for asset in assets.iter() {
        asset.assert_sent_native_token_balance(&info)?;
    }

    let pair_info: PairInfoRaw = PAIR_INFO.load(deps.storage)?;
    let mut pools: [Asset; 2] =
        pair_info.query_pools(&deps.querier, deps.api, env.contract.address.clone())?;
    let deposits: [Uint128; 2] = [
        assets
            .iter()
            .find(|a| a.info.equal(&pools[0].info))
            .map(|a| a.amount)
            .expect("Wrong asset info is given"),
        assets
            .iter()
            .find(|a| a.info.equal(&pools[1].info))
            .map(|a| a.amount)
            .expect("Wrong asset info is given"),
    ];

    let mut messages: Vec<CosmosMsg> = vec![];
    for (i, pool) in pools.iter_mut().enumerate() {
        // If the pool is token contract, then we need to execute TransferFrom msg to receive funds
        if let AssetInfo::Token { contract_addr, .. } = &pool.info {
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_owned().into(),
                msg: to_binary(&Cw20HandleMsg::TransferFrom {
                    owner: info.sender.clone(),
                    recipient: env.contract.address.clone(),
                    amount: deposits[i],
                })?,
                send: vec![],
            }));
        } else {
            // If the asset is native token, balance is already increased
            // To calculated properly we should subtract user deposit from the pool
            pool.amount = Asset::checked_sub(pool.amount, deposits[i])?;
        }
    }

    // assert slippage tolerance
    assert_slippage_tolerance(&slippage_tolerance, &deposits, &pools)?;

    let liquidity_token = deps.api.human_address(&pair_info.liquidity_token)?;
    let total_share = query_supply(&deps.querier, liquidity_token)?;
    let share = if total_share == Uint128::zero() {
        // Initial share = collateral amount
        Uint128::from((deposits[0].u128() * deposits[1].u128()).integer_sqrt())
    } else {
        // min(1, 2)
        // 1. sqrt(deposit_0 * exchange_rate_0_to_1 * deposit_0) * (total_share / sqrt(pool_0 * pool_1))
        // == deposit_0 * total_share / pool_0
        // 2. sqrt(deposit_1 * exchange_rate_1_to_0 * deposit_1) * (total_share / sqrt(pool_1 * pool_1))
        // == deposit_1 * total_share / pool_1
        std::cmp::min(
            deposits[0].multiply_ratio(total_share, pools[0].amount),
            deposits[1].multiply_ratio(total_share, pools[1].amount),
        )
    };

    // prevent providing free token
    if share.is_zero() {
        return Err(ContractError::InvalidZeroAmount {});
    }

    // mint LP token to sender
    let receiver = receiver.unwrap_or(info.sender.clone());
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: deps.api.human_address(&pair_info.liquidity_token)?,
        msg: to_binary(&Cw20HandleMsg::Mint {
            recipient: receiver.clone(),
            amount: share,
        })?,
        send: vec![],
    }));

    Ok(HandleResponse {
        messages,
        attributes: vec![
            attr("action", "provide_liquidity"),
            attr("sender", info.sender.as_str()),
            attr("receiver", receiver.as_str()),
            attr("assets", &format!("{}, {}", assets[0], assets[1])),
            attr("share", &share.to_string()),
        ],
        data: None,
    })
}

pub fn withdraw_liquidity(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    sender: HumanAddr,
    amount: Uint128,
) -> Result<HandleResponse, ContractError> {
    let pair_info: PairInfoRaw = PAIR_INFO.load(deps.storage)?;
    let liquidity_addr = deps.api.human_address(&pair_info.liquidity_token)?;

    let pools: [Asset; 2] =
        pair_info.query_pools(&deps.querier, deps.api, env.contract.address.clone())?;
    let total_share: Uint128 = query_supply(&deps.querier, liquidity_addr)?;

    let share_ratio: Decimal = Decimal::from_ratio(amount, total_share);
    let refund_assets: Vec<Asset> = pools
        .iter()
        .map(|a| Asset {
            info: a.info.clone(),
            amount: a.amount * share_ratio,
        })
        .collect();

    let oracle_contract = OracleContract(deps.api.human_address(&pair_info.oracle_addr)?);

    // update pool info
    Ok(HandleResponse {
        messages: vec![
            refund_assets[0].clone().into_msg(
                &oracle_contract,
                &deps.querier,
                env.contract.address.clone(),
                sender.clone(),
            )?,
            refund_assets[1].clone().into_msg(
                &oracle_contract,
                &deps.querier,
                env.contract.address,
                sender.clone(),
            )?,
            // burn liquidity token
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: deps.api.human_address(&pair_info.liquidity_token)?,
                msg: to_binary(&Cw20HandleMsg::Burn { amount })?,
                send: vec![],
            }),
        ],
        attributes: vec![
            attr("action", "withdraw_liquidity"),
            attr("sender", sender.as_str()),
            attr("withdrawn_share", &amount.to_string()),
            attr(
                "refund_assets",
                &format!("{}, {}", refund_assets[0], refund_assets[1]),
            ),
        ],
        data: None,
    })
}

/// CONTRACT - a user must do token approval
/// some params retrieving from oracle contract
#[allow(clippy::too_many_arguments)]
pub fn swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sender: HumanAddr,
    offer_asset: Asset,
    belief_price: Option<Decimal>,
    max_spread: Option<Decimal>,
    to: Option<HumanAddr>,
) -> Result<HandleResponse, ContractError> {
    offer_asset.assert_sent_native_token_balance(&info)?;

    let pair_info: PairInfoRaw = PAIR_INFO.load(deps.storage)?;

    let pools: [Asset; 2] =
        pair_info.query_pools(&deps.querier, deps.api, env.contract.address.clone())?;

    let offer_pool: Asset;
    let ask_pool: Asset;

    // If the asset balance is already increased
    // To calculated properly we should subtract user deposit from the pool
    if offer_asset.info.equal(&pools[0].info) {
        offer_pool = Asset {
            amount: Asset::checked_sub(pools[0].amount, offer_asset.amount)?,
            info: pools[0].info.clone(),
        };
        ask_pool = pools[1].clone();
    } else if offer_asset.info.equal(&pools[1].info) {
        offer_pool = Asset {
            amount: Asset::checked_sub(pools[1].amount, offer_asset.amount)?,
            info: pools[1].info.clone(),
        };
        ask_pool = pools[0].clone();
    } else {
        return Err(ContractError::AssetMismatch {});
    }

    let offer_amount = offer_asset.amount;
    let (return_amount, spread_amount, commission_amount) =
        compute_swap(offer_pool.amount, ask_pool.amount, offer_amount);

    // check max spread limit if exist
    assert_max_spread(
        belief_price,
        max_spread,
        offer_amount,
        return_amount + commission_amount,
        spread_amount,
    )?;

    // compute tax
    let return_asset = Asset {
        info: ask_pool.info.clone(),
        amount: return_amount,
    };

    let oracle_contract = OracleContract(deps.api.human_address(&pair_info.oracle_addr)?);

    let tax_amount = return_asset.compute_tax(&oracle_contract, &deps.querier)?;
    let receiver = to.unwrap_or_else(|| sender.clone());

    // update oracle_contract
    let mut messages: Vec<CosmosMsg> = vec![];
    if !return_amount.is_zero() {
        messages.push(return_asset.into_msg(
            &oracle_contract,
            &deps.querier,
            env.contract.address,
            receiver.clone(),
        )?);
    }

    // 1. send collateral token from the contract to a user
    // 2. send inactive commission to collector
    Ok(HandleResponse {
        messages,
        attributes: vec![
            attr("action", "swap"),
            attr("sender", sender.as_str()),
            attr("receiver", receiver.as_str()),
            attr("offer_asset", &offer_asset.info.to_string()),
            attr("ask_asset", &ask_pool.info.to_string()),
            attr("offer_amount", &offer_amount.to_string()),
            attr("return_amount", &return_amount.to_string()),
            attr("tax_amount", &tax_amount.to_string()),
            attr("spread_amount", &spread_amount.to_string()),
            attr("commission_amount", &commission_amount.to_string()),
        ],
        data: None,
    })
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Pair {} => Ok(to_binary(&query_pair_info(deps)?)?),
        QueryMsg::Pool {} => Ok(to_binary(&query_pool(deps)?)?),
        QueryMsg::Simulation { offer_asset } => {
            Ok(to_binary(&query_simulation(deps, offer_asset)?)?)
        }
        QueryMsg::ReverseSimulation { ask_asset } => {
            Ok(to_binary(&query_reverse_simulation(deps, ask_asset)?)?)
        }
    }
}

pub fn query_pair_info(deps: Deps) -> Result<PairInfo, ContractError> {
    let pair_info: PairInfoRaw = PAIR_INFO.load(deps.storage)?;
    let pair_info = pair_info.to_normal(deps.api)?;

    Ok(pair_info)
}

pub fn query_pool(deps: Deps) -> Result<PoolResponse, ContractError> {
    let pair_info: PairInfoRaw = PAIR_INFO.load(deps.storage)?;
    let contract_addr = deps.api.human_address(&pair_info.contract_addr)?;
    let assets: [Asset; 2] = pair_info.query_pools(&deps.querier, deps.api, contract_addr)?;
    let total_share: Uint128 = query_supply(
        &deps.querier,
        deps.api.human_address(&pair_info.liquidity_token)?,
    )?;

    let resp = PoolResponse {
        assets,
        total_share,
    };

    Ok(resp)
}

pub fn query_simulation(
    deps: Deps,
    offer_asset: Asset,
) -> Result<SimulationResponse, ContractError> {
    let pair_info: PairInfoRaw = PAIR_INFO.load(deps.storage)?;

    let contract_addr = deps.api.human_address(&pair_info.contract_addr)?;
    let pools: [Asset; 2] = pair_info.query_pools(&deps.querier, deps.api, contract_addr)?;

    let offer_pool: Asset;
    let ask_pool: Asset;
    if offer_asset.info.equal(&pools[0].info) {
        offer_pool = pools[0].clone();
        ask_pool = pools[1].clone();
    } else if offer_asset.info.equal(&pools[1].info) {
        offer_pool = pools[1].clone();
        ask_pool = pools[0].clone();
    } else {
        return Err(ContractError::AssetMismatch {});
    }

    let (return_amount, spread_amount, commission_amount) =
        compute_swap(offer_pool.amount, ask_pool.amount, offer_asset.amount);

    Ok(SimulationResponse {
        return_amount,
        spread_amount,
        commission_amount,
    })
}

pub fn query_reverse_simulation(
    deps: Deps,
    ask_asset: Asset,
) -> Result<ReverseSimulationResponse, ContractError> {
    let pair_info: PairInfoRaw = PAIR_INFO.load(deps.storage)?;

    let contract_addr = deps.api.human_address(&pair_info.contract_addr)?;
    let pools: [Asset; 2] = pair_info.query_pools(&deps.querier, deps.api, contract_addr)?;

    let offer_pool: Asset;
    let ask_pool: Asset;
    if ask_asset.info.equal(&pools[0].info) {
        ask_pool = pools[0].clone();
        offer_pool = pools[1].clone();
    } else if ask_asset.info.equal(&pools[1].info) {
        ask_pool = pools[1].clone();
        offer_pool = pools[0].clone();
    } else {
        return Err(ContractError::AssetMismatch {});
    }

    let (offer_amount, spread_amount, commission_amount) =
        compute_offer_amount(offer_pool.amount, ask_pool.amount, ask_asset.amount)?;

    Ok(ReverseSimulationResponse {
        offer_amount,
        spread_amount,
        commission_amount,
    })
}

pub fn amount_of(coins: &[Coin], denom: String) -> Uint128 {
    match coins.iter().find(|x| x.denom == denom) {
        Some(coin) => coin.amount,
        None => Uint128::zero(),
    }
}

fn compute_swap(
    offer_pool: Uint128,
    ask_pool: Uint128,
    offer_amount: Uint128,
) -> (Uint128, Uint128, Uint128) {
    // convert to uint256
    let offer_pool: Uint256 = offer_pool.into();
    let ask_pool: Uint256 = ask_pool.into();
    let offer_amount: Uint256 = offer_amount.into();

    let commission_rate = Decimal256::from_str(COMMISSION_RATE).unwrap();

    // offer => ask
    // ask_amount = (ask_pool - cp / (offer_pool + offer_amount)) * (1 - commission_rate)
    let cp: Uint256 = offer_pool * ask_pool;
    let return_amount: Uint256 = (Decimal256::from_uint256(ask_pool)
        - Decimal256::from_ratio(cp, offer_pool + offer_amount))
        * Uint256::one();

    // calculate spread & commission
    let spread_amount: Uint256 =
        (offer_amount * Decimal256::from_ratio(ask_pool, offer_pool)) - return_amount;
    let commission_amount: Uint256 = return_amount * commission_rate;

    // commission will be absorbed to pool
    let return_amount: Uint256 = return_amount - commission_amount;
    (
        return_amount.into(),
        spread_amount.into(),
        commission_amount.into(),
    )
}

#[test]
fn test_compute_swap_with_huge_pool_variance() {
    let offer_pool = Uint128::from(395451850234u128);
    let ask_pool = Uint128::from(317u128);

    assert_eq!(
        compute_swap(offer_pool, ask_pool, Uint128::from(1u128)).0,
        Uint128::zero()
    );
}

fn compute_offer_amount(
    offer_pool: Uint128,
    ask_pool: Uint128,
    ask_amount: Uint128,
) -> Result<(Uint128, Uint128, Uint128), ContractError> {
    let offer_pool: Uint256 = offer_pool.into();
    let ask_pool: Uint256 = ask_pool.into();
    let ask_amount: Uint256 = ask_amount.into();

    let commission_rate = Decimal256::from_str(COMMISSION_RATE).unwrap();

    // ask => offer
    // offer_amount = cp / (ask_pool - ask_amount / (1 - commission_rate)) - offer_pool
    let cp: Uint256 = offer_pool * ask_pool;

    let one_minus_commission = Decimal256::one() - commission_rate;
    let inv_one_minus_commission = Decimal256::one() / one_minus_commission;

    let offer_amount: Uint256 = Uint256::one()
        .multiply_ratio(cp, ask_pool - ask_amount * inv_one_minus_commission)
        - offer_pool;

    let before_commission_deduction: Uint256 = ask_amount * inv_one_minus_commission;
    let before_spread_deduction: Uint256 =
        offer_amount * Decimal256::from_ratio(ask_pool, offer_pool);

    let spread_amount = if before_spread_deduction > before_commission_deduction {
        before_spread_deduction - before_commission_deduction
    } else {
        Uint256::zero()
    };

    let commission_amount = before_commission_deduction * commission_rate;

    // check small amount swap
    if spread_amount.is_zero() || commission_amount.is_zero() {
        return Err(ContractError::TooSmallOfferAmount {});
    }

    Ok((
        offer_amount.into(),
        spread_amount.into(),
        commission_amount.into(),
    ))
}

/// If `belief_price` and `max_spread` both are given,
/// we compute new spread else we just use oraiswap
/// spread to check `max_spread`
pub fn assert_max_spread(
    belief_price: Option<Decimal>,
    max_spread: Option<Decimal>,
    offer_amount: Uint128,
    return_amount: Uint128,
    spread_amount: Uint128,
) -> Result<(), ContractError> {
    let offer_amount: Uint256 = offer_amount.into();
    let return_amount: Uint256 = return_amount.into();
    let spread_amount: Uint256 = spread_amount.into();

    if let (Some(max_spread), Some(belief_price)) = (max_spread, belief_price) {
        let belief_price: Decimal256 = belief_price.into();
        let max_spread: Decimal256 = max_spread.into();

        let expected_return = offer_amount / belief_price;
        let spread_amount = if expected_return > return_amount {
            expected_return - return_amount
        } else {
            Uint256::zero()
        };

        if return_amount < expected_return
            && Decimal256::from_ratio(spread_amount, expected_return) > max_spread
        {
            return Err(ContractError::MaxSpreadAssertion {});
        }
    } else if let Some(max_spread) = max_spread {
        let max_spread: Decimal256 = max_spread.into();
        if Decimal256::from_ratio(spread_amount, return_amount + spread_amount) > max_spread {
            return Err(ContractError::MaxSpreadAssertion {});
        }
    }

    Ok(())
}

fn assert_slippage_tolerance(
    slippage_tolerance: &Option<Decimal>,
    deposits: &[Uint128; 2],
    pools: &[Asset; 2],
) -> Result<(), ContractError> {
    if let Some(slippage_tolerance) = *slippage_tolerance {
        let slippage_tolerance: Decimal256 = slippage_tolerance.into();
        if slippage_tolerance > Decimal256::one() {
            return Err(StdError::generic_err("slippage_tolerance cannot bigger than 1").into());
        }

        let one_minus_slippage_tolerance = Decimal256::one() - slippage_tolerance;
        let deposits: [Uint256; 2] = [deposits[0].into(), deposits[1].into()];
        let pools: [Uint256; 2] = [pools[0].amount.into(), pools[1].amount.into()];

        // Ensure each prices are not dropped as much as slippage tolerance rate
        if Decimal256::from_ratio(deposits[0], deposits[1]) * one_minus_slippage_tolerance
            > Decimal256::from_ratio(pools[0], pools[1])
            || Decimal256::from_ratio(deposits[1], deposits[0]) * one_minus_slippage_tolerance
                > Decimal256::from_ratio(pools[1], pools[0])
        {
            return Err(ContractError::MaxSlippageAssertion {});
        }
    }

    Ok(())
}

pub fn migrate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: MigrateMsg,
) -> Result<MigrateResponse, ContractError> {
    Ok(MigrateResponse::default())
}
