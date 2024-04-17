# Oraiswap Limit Order <!-- omit in toc -->

The Limit Order Contract is to provide limit order interface to a bidder and also provide arbitrage opportunity to a market maker.

## Use Cases

1. usdt -> ASSET

```
Order
+-----------------+    Oraiswap Price (usdt-orai)
| OrderId       1 |    +------------------------+
| Offer   100 usdt |    | price 95 usdt : 1 orai |
| Ask     1 orai |    +------------------------+
+-----------------+                 |
        ^                           |
        |       +-------------+     |
        +-----  | Arbitrageur |<----+
         Sell   +-------------+  Buy

```

2. ASSET => usdt

```
Order
+-----------------+    Oraiswap Price (usdt-orai)
| OrderId       1 |    +-------------------------+
| Offer   1 orai |    | price 110 usdt : 1 orai |
| Ask     100 usdt |    +-------------------------+
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
      'orderbook_contract_addr',
      [Coin('denom', 'amount')],
      base64(SubmitOrder {
          direction: OrderDirection::Buy,
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
          contract_addr: 'orderbook_contract_addr',
          amount: 'amount',
          msg: Some(base64(SubmitOrder {
              direction: OrderDirection::Buy,
              ask_asset: Asset,
          })),
      })
  )
  ```

### Cancel Order

```
MsgExecuteContract(
    'orderbook_contract_addr',
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
      'orderbook_contract_addr',
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
          contract_addr: 'orderbook_contract_addr',
          amount: 'amount',
          msg: Some(base64(ExecuteOrder {
              order_id: u64,
          })),
      })
  )
  ```

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

### Code coverage:

```bash
cargo tarpaulin --lib --ignore-tests -o html
```
