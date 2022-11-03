use cosmwasm_schema::{cw_serde, QueryResponses};

use crate::{
    asset::{Asset, AssetInfo, PairInfo},
    error::ContractError,
    Decimal256, Uint256,
};

use cosmwasm_std::{Addr, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;

/// Default commission rate == 0.3%
/// in the future need to update ?
pub const DEFAULT_COMMISSION_RATE: &str = "0.003";

#[cw_serde]
pub struct InstantiateMsg {
    /// Asset infos
    pub asset_infos: [AssetInfo; 2],
    /// Token contract code id for initialization
    pub token_code_id: u64,

    /// Oracle contract for query oracle information
    pub oracle_addr: Addr,

    pub commission_rate: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    /// ProvideLiquidity a user provides pool liquidity
    ProvideLiquidity {
        assets: [Asset; 2],
        slippage_tolerance: Option<Decimal>,
        receiver: Option<Addr>,
    },
    /// Swap an offer asset to the other
    Swap {
        offer_asset: Asset,
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
        to: Option<Addr>,
    },
}

#[cw_serde]
pub enum Cw20HookMsg {
    /// Sell a given amount of asset
    Swap {
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
        to: Option<String>,
    },
    WithdrawLiquidity {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(PairResponse)]
    Pair {},
    #[returns(PoolResponse)]
    Pool {},
    #[returns(SimulationResponse)]
    Simulation { offer_asset: Asset },
    #[returns(ReverseSimulationResponse)]
    ReverseSimulation { ask_asset: Asset },
}

// We define a custom struct for each query response
#[cw_serde]
pub struct PoolResponse {
    pub assets: [Asset; 2],
    pub total_share: Uint128,
}

#[cw_serde]
pub struct PairResponse {
    pub info: PairInfo,
}

/// SimulationResponse returns swap simulation response
#[cw_serde]
pub struct SimulationResponse {
    pub return_amount: Uint128,
    pub spread_amount: Uint128,
    pub commission_amount: Uint128,
}

/// ReverseSimulationResponse returns reverse swap simulation response
#[cw_serde]
pub struct ReverseSimulationResponse {
    pub offer_amount: Uint128,
    pub spread_amount: Uint128,
    pub commission_amount: Uint128,
}

/// We currently take no arguments for migrations
#[cw_serde]
pub struct MigrateMsg {}

pub fn compute_swap(
    offer_pool: Uint128,
    ask_pool: Uint128,
    offer_amount: Uint128,
    commission_rate: Decimal256,
) -> Result<(Uint128, Uint128, Uint128), ContractError> {
    if offer_pool.is_zero() {
        return Err(ContractError::OfferPoolIsZero {});
    }

    // convert to uint256
    let offer_pool: Uint256 = offer_pool.into();
    let ask_pool: Uint256 = ask_pool.into();
    let offer_amount: Uint256 = offer_amount.into();

    // offer => ask
    // ask_amount = (ask_pool - cp / (offer_pool + offer_amount)) * (1 - commission_rate)
    let cp = offer_pool * ask_pool;

    let return_amount =
        ask_pool - Decimal256::from_ratio(cp, offer_pool + offer_amount) * Uint256::one();

    // calculate spread & commission
    let spread_amount =
        (offer_amount * Decimal256::from_ratio(ask_pool, offer_pool)) - return_amount;

    let commission_amount = return_amount * commission_rate;

    // commission will be absorbed to pool
    let return_amount = return_amount - commission_amount;
    Ok((
        return_amount.into(),
        spread_amount.into(),
        commission_amount.into(),
    ))
}
