# Oraiswap Limit Order <!-- omit in toc -->

**NOTE**: Reference documentation for this contract is available [here](https://docs.mirror.finance/contracts/limit-order).

The Limit Order Contract is to provide limit order interface to a bidder and also provide arbitrage opportunity to a market maker.

## Use Cases

1. UST -> ASSET

```
Order
+-----------------+    Oraiswap Price (UST-mAAPL)
| OrderId       1 |    +------------------------+
| Offer   100 UST |    | price 95 UST : 1 mAAPL |
| Ask     1 mAAPL |    +------------------------+
+-----------------+                 |
        ^                           |
        |       +-------------+     |
        +-----  | Arbitrageur |<----+
         Sell   +-------------+  Buy

```

2. ASSET => UST

```
Order
+-----------------+    Oraiswap Price (UST-mAAPL)
| OrderId       1 |    +-------------------------+
| Offer   1 mAAPL |    | price 110 UST : 1 mAAPL |
| Ask     100 UST |    +-------------------------+
+-----------------+                 ^
        |                           |
        |       +-------------+     |
        +-----> | Arbitrageur |-----+
         Buy    +-------------+  Sell
```

## Handlers

### Submit Order

Depends on the offer asset type

- Native Token

  ```
  MsgExecuteContract(
      'limit_order_contract_addr',
      [Coin('denom', 'amount')],
      base64(SubmitOrder {
          offer_asset: Asset,
          ask_asset: Asset,
      })
  )
  ```

- Token
  ```
  MsgExecuteContract(
      'token_contract',
      [],
      base64(Send {
          contract_addr: 'limit_order_contract_addr',
          amount: 'amount',
          msg: Some(base64(SubmitOrder {
              ask_asset: Asset,
          })),
      })
  )
  ```

### Cancel Order

```
MsgExecuteContract(
    'limit_order_contract_addr',
    [],
    base64(CancelOrder {
        order_id: u64,
    })
)
```

### Execute Order

> Order can be executed partially

Depends on the `ask asset`(= `execute asset`) type

- Native Token

  ```
  MsgExecuteContract(
      'limit_order_contract_addr',
      [Coin('denom', 'amount')],
      base64(ExecuteOrder {
       execute_asset: Asset,
       order_id: u64,
      })
  )
  ```

- Token
  ```
  MsgExecuteContract(
      'token_contract',
      [],
      base64(Send {
          contract_addr: 'limit_order_contract_addr',
          amount: 'amount',
          msg: Some(base64(ExecuteOrder {
              order_id: u64,
          })),
      })
  )
  ```

# Query Orders

- Query a order

  - https://lcd.orai.io/cosmwasm/wasm/contracts/`limit_order_contract`/store?query_msg={"order":{"order_id": 100}}

- Query orders

  - Query with bidder address
    - https://lcd.orai.io/cosmwasm/wasm/contracts/`limit_order_contract`/store?query_msg={"orders":{"bidder_addr": "orai~"}}
    - https://lcd.orai.io/cosmwasm/wasm/contracts/`limit_order_contract`/store?query_msg={"orders":{"bidder_addr": "orai~", "start_after": 50, "limit": 10, "order_by": "desc"}}
  - Query without filter
    - https://lcd.orai.io/cosmwasm/wasm/contracts/`limit_order_contract`/store?query_msg={"orders":{}}
    - https://lcd.orai.io/cosmwasm/wasm/contracts/`limit_order_contract`/store?query_msg={"orders":{"start_after": 50, "limit": 10, "order_by": "desc"}}

- Query last order id
  - https://lcd.orai.io/cosmwasm/wasm/contracts/`limit_order_contract`/store?query_msg={"last_order_id":{}}

# TODO

- find matching price so that we can call intervally to auto matching
- store total amount sell, buy for an orderbook => as liquidity (update total when remove order, store order)
