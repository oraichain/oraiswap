# OraiSwap Token

# CW20 Basic with expanded name and symbol range

This is a basic implementation of a cw20 contract. It implements
the [CW20 spec](https://github.com/CosmWasm/cosmwasm-plus/tree/master/packages/cw20) and is designed to
be deployed as is, or imported into other contracts to easily build
cw20-compatible tokens with custom logic.

Implements:

- [x] CW20 Base
- [ ] Mintable extension
- [ ] Allowances extension

## Running this contract

You will need Rust 1.44.1+ with `wasm32-unknown-unknown` target installed.

You can run unit tests on this via:

`cargo test`

## Importing this contract

You can also import much of the logic of this contract to build another
ERC20-contract, such as a bonding curve, overiding or extending what you
need.

Basically, you just need to write your execute function and import
`cw20_base::contract::execute_transfer`, etc and dispatch to them.
This allows you to use custom `ExecuteMsg` and `QueryMsg` with your additional
calls, but then use the underlying implementation for the standard cw20
messages you want to support. The same with `QueryMsg`. You _could_ reuse `init`
as it, but it is likely you will want to change it. And it is rather simple.

**TODO: add example**
