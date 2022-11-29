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

# Orderbook

Each orderbook corresponding to a pair of assets (ask_asset, offer_asset)  
when user place a buy transaction, the Order format is

```json
{
  "direction": "buy",
  "offer_amount": "16485",
  "ask_amount": "15000"
}
```

which means: **paid 16485 usdt, buy 15000 orai at 1.098**

when user place a sell transaction, the Order format is

```json
[
  {
    "direction": "sell",
    "offer_amount": "10990",
    "ask_amount": "10000"
  },
  {
    "direction": "sell",
    "offer_amount": "5945",
    "ask_amount": "5000"
  }
]
```

which means: **want 10990 usdt, sell 10000 orai at 1.098**

Each [price key,order direction] (offer_amount / ask_amount) is used to store an order list. At an interval amount of time, should be block time, the orderbook will loop through each orderbook to find out the best match (highest_buy_price for buy direction and lowest_sell_price for sell direction with a precision should be 1%). The formulation is:

**lowest_buy_price \* (1 + precision) >= highest_buy_price >= lowest_buy_price**

At each match price, the orderbook will distribute the ask order (buy direction) to all matchable offer orders (sell direction) limited by the storage query limit in Ascending order.  
The process is repeatedly running to create batch transactions delivering desired Assets to all bidders. All filled up orders will be removed from storage, the orthers are updated with new filled amounts.

The output is:  
ask order

```json
{
  "order_id": 3,
  "direction": "buy",
  "bidder_addr": "AAAAAAAAAAAAAABkMAAAAAAAAAAAAAAAAHIwAAAAAAAAAAAAAABhMAAAAAAAAAAAAAAAAGQw",
  "offer_amount": "16485",
  "ask_amount": "15000",
  "filled_offer_amount": "16485",
  "filled_ask_amount": "15000"
}
```

offer order

```json
[
  {
    "order_id": 5,
    "direction": "sell",
    "bidder_addr": "AAAAAAAAAAAAAABkMAAAAAAAAAAAAAAAAHIwAAAAAAAAAAAAAABhMAAAAAAAAAAAAAAAAGQw",
    "offer_amount": "10990",
    "ask_amount": "10000",
    "filled_offer_amount": "10990",
    "filled_ask_amount": "10000"
  },
  {
    "order_id": 6,
    "direction": "sell",
    "bidder_addr": "AAAAAAAAAAAAAABkMAAAAAAAAAAAAAAAAHIwAAAAAAAAAAAAAABhMAAAAAAAAAAAAAAAAGQw",
    "offer_amount": "5495",
    "ask_amount": "5000",
    "filled_offer_amount": "5495",
    "filled_ask_amount": "5000"
  }
]
```
