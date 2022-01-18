# OraiSwap

Uniswap-inspired automated market-maker (AMM) protocol powered by Smart Contracts on the [Orai](https://orai.io) blockchain.

## Contracts

| Name                                             | Description                                              |
| ------------------------------------------------ | -------------------------------------------------------- |
| [`oraiswap_factory`](contracts/oraiswap_factory) | Proxy contract to creat oraiswap_pair instance           |
| [`oraiswap_oracle`](contracts/oraiswap_oracle)   | Global parameters updated by multisig wallet             |
| [`oraiswap_pair`](contracts/oraiswap_pair)       | Logic for building liquidity pool and trade between pair |
| [`oraiswap_router`](contracts/oraiswap_router)   | facilitate multi-hop swap operations                     |
| [`oraiswap_token`](contracts/oraiswap_token)     | CW20 (ERC20 equivalent) token implementation             |

- oraiswap_factory

  Mainnet: `orai1ulgw0td86nvs4wtpsc80thv6xelk76ut7a7apj`

  Testnet: `orai18qpjm4zkvqnpjpw0zn0tdr8gdzvt8au35v45xf`

- oraiswap_oracle

  Mainnet: `orai1ulgw0td86nvs4wtpsc80thv6xelk76ut7a7apj`

  Testnet: `orai18qpjm4zkvqnpjpw0zn0tdr8gdzvt8au35v45xf`

- oraiswap_pair

  Mainnet (CodeID): 4

  Testnet (CodeID): 7869

- oraiswap_route

  Mainnet: `orai1ulgw0td86nvs4wtpsc80thv6xelk76ut7a7apj`

  Testnet: `orai18qpjm4zkvqnpjpw0zn0tdr8gdzvt8au35v45xf`

- oraiswap_token

  Mainnet (CodeID): 3

  Testnet (CodeID): 148

## Running this contract

You will need Rust 1.44.1+ with wasm32-unknown-unknown target installed.

You can run unit tests on this on each contracts directory via :

```
cargo unit-test
cargo integration-test
```

Once you are happy with the content, you can compile it to wasm on each contracts directory via:

```
RUSTFLAGS='-C link-arg=-s' cargo wasm
cp ../../target/wasm32-unknown-unknown/release/cw1_subkeys.wasm .
ls -l cw1_subkeys.wasm
sha256sum cw1_subkeys.wasm
```

Or for a production-ready (compressed) build, run the following from the repository root:

```
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.10.2
```

The optimized contracts are generated in the artifacts/ directory.
