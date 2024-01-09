# OraiSwap

Uniswap-inspired automated market-maker (AMM) protocol powered by Smart Contracts on the [Orai](https://orai.io) blockchain.

## Contracts

| Name                                                     | Description                                              |
| -------------------------------------------------------- | -------------------------------------------------------- |
| [`oraiswap_factory`](contracts/oraiswap_factory)         | Proxy contract to create oraiswap_pair instance          |
| [`oraiswap_oracle`](contracts/oraiswap_oracle)           | Global parameters updated by multisig wallet             |
| [`oraiswap_pair`](contracts/oraiswap_pair)               | Logic for building liquidity pool and trade between pair |
| [`oraiswap_router`](contracts/oraiswap_router)           | Facilitate multi-hop swap operations                     |
| [`oraiswap_limit_order`](contracts/oraiswap_limit_order) | Orderbook implementation                                 |
| [`oraiswap_staking`](contracts/oraiswap_staking)         | Stake LPs to get ORAIX reward                            |
| [`oraiswap_token`](contracts/oraiswap_token)             | (ERC20 equivalent) token implementation, AIRI, ORAIX     |

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

- oraiswap_limit_order

  Mainnet: `orai1ulgw0td86nvs4wtpsc80thv6xelk76ut7a7apj`

  Testnet: `orai18qpjm4zkvqnpjpw0zn0tdr8gdzvt8au35v45xf`

- oraiswap_staking

  Mainnet: `orai19p43y0tqnr5qlhfwnxft2u5unph5yn60y7tuvu`

  Testnet: `orai1yzncqj7f8sculc3849w9hg9r4f4u79e3swnlr7`

- oraiswap_token

  Mainnet (CodeID): 3

  Testnet (CodeID): 148

## Running this contract

You will need Rust 1.44.1+ with wasm32-unknown-unknown target installed.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown
```

You can run unit tests on this on each contracts directory via :

```bash
cargo unit-test
cargo integration-test
```

## Gen proto definitions

```bash
# gen protobuf response.rs
cargo install protobuf-codegen
protoc --rust_out . response.proto
tee -a response.proto << END
impl ::std::convert::TryFrom<&[u8]> for MsgInstantiateContractResponse {
    type Error = ::protobuf::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
       ::protobuf::Message::parse_from_bytes(value)
    }
}
END
```
