# Oraiswap: Common Types

This is a collection of common types and the queriers which are commonly used in oraiswap contracts.

## Data Types

### AssetInfo

AssetInfo is a convenience wrapper to represent the native token and the contract token as a single type.
Currently there is only Orai native token in Oraichain blockchain.

```rust
#[cw_serde]
pub enum AssetInfo {
    Token { contract_addr: Addr },
    NativeToken { denom: String },
}
```

### Asset

It contains asset info with the amount of token.

```rust
pub struct Asset {
    pub info: AssetInfo,
    pub amount: Uint128,
}
```

### PairInfo

It is used to represent response data of [Pair-Info-Querier](#Pair-Info-Querier)

```rust
pub struct PairInfo {
    pub contract_addr: Addr,
    pub asset_infos: [AssetInfo; 2],
}
```

## Queriers

### Native Token Balance Querier

It uses CosmWasm standard interface to query the account balance to chain.

```rust
pub fn query_balance(
    querier: &QuerierWrapper,
    account_addr: Addr,
    denom: String,
) -> StdResult<Uint128> {
```

### Token Balance Querier

It provides similar query interface with [Native-Token-Balance-Querier](Native-Token-Balance-Querier) for CW20 token balance.

```rust
pub fn query_token_balance(
    querier: &QuerierWrapper,
    contract_addr: Addr,
    account_addr: Addr,
) -> StdResult<Uint128> {
```

### Token Supply Querier

It provides token supply querier for CW20 token contract.

```rust
pub fn query_supply(
    querier: &QuerierWrapper,
    contract_addr: Addr,
) -> StdResult<Uint128> {
```

### Pair Info Querier

It also provides the query interface to query available oraiswap pair contract info. Any contract can query pair info to oraiswap factory contract.

```rust
pub fn query_pair_info(
    querier: &QuerierWrapper,
    factory_contract: Addr,
    asset_infos: &[AssetInfo; 2],
) -> StdResult<PairInfo> {
```
