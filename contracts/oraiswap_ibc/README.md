# Swap ICS20 (Juno)
[![contracts-ci](https://img.shields.io/github/workflow/status/giansalex/cw-osmo-swap/contracts-ci/master?label=contract-ci)](https://github.com/giansalex/cw-osmo-swap/actions/workflows/rust.yml)

This is an *IBC Enabled* contract implements the standard ICS20 (IBC transfers), and can send custom
actions to osmosis chain, e.g. swap, join pool, exit pool.

## Messages
- `Transfer{}`: IBC Transfer
- `Swap{}`: Swap assets in Osmosis
- `JoinPool{}`: Add liquidity to a pool in Osmosis
- `ExitPool{}`: Remove liquidity to a pool in Osmosis
- `CreateLockup{}`: Create lockup account
- `LockTokens{}`: Lock tokens (Start farming)
- `ClaimTokens{}`: Claim rewards or LP tokens unlocked
- `UnLockTokens{}`: Begin unlock tokens
- `AllowExternalToken{}`: Allow external native tokens (from osmosis)

## Query

- `ListChannels{}`: List channels
- `Channel{}`: Get channel info by ID
- `ListExternalTokens{}`: List external tokens allowed
- `Lockup{}`: Get lockup account by user
