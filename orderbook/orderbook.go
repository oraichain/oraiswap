package orderbook

import (
	"fmt"
	"math/big"
	"strings"
	"time"
)

const (
	// Ask : ask constant
	Ask = "ask"
	// Bid : bid constant
	Bid    = "bid"
	Market = "market"
	Limit  = "limit"

	// we use a big number as segment for storing order, order list from order tree slot.
	// as sequential id
	SlotSegment = 20
)

type OrderBookItem struct {
	Timestamp     uint64 `json:"time"`
	NextOrderID   uint64 `json:"nextOrderID"`
	MaxPricePoint uint64 `json:"maxVolume"` // maximum
	Name          string `json:"name"`
}

// OrderBook : list of orders
type OrderBook struct {
	db   *BatchDatabase // this is for orderBook
	Bids *OrderTree     `json:"bids"`
	Asks *OrderTree     `json:"asks"`
	Item *OrderBookItem

	Key  []byte
	slot *big.Int
}

// NewOrderBook : return new order book
func NewOrderBook(name string, db *BatchDatabase) *OrderBook {

	// we can implement using only one DB to faciliate cache engine
	// so that we use a big.Int number to seperate domain of the keys
	// like this keccak("orderBook") + key
	// orderBookPath := path.Join(datadir, "orderbook")
	// db := NewBatchDatabase(orderBookPath, 0, 0)

	item := &OrderBookItem{
		NextOrderID: 0,
		Name:        strings.ToLower(name),
	}

	// do slot with hash to prevent collision

	// we convert to lower case, so even with name as contract address, it is still correct
	// without converting back from hex to bytes
	key := hash.Sum([]byte(item.Name))
	slot := new(big.Int).SetBytes(key)

	// we just increase the segment at the most byte at address length level to avoid conflict
	// somehow it is like 2 hashes has the same common prefix and it is very difficult to resolve
	// the order id start at orderbook slot
	// the price of order tree start at order tree slot
	bidsKey := GetSegmentHash(key, 1, SlotSegment)
	asksKey := GetSegmentHash(key, 2, SlotSegment)

	orderBook := &OrderBook{
		db:   db,
		Item: item,
		slot: slot,
		Key:  key,
	}

	bids := NewOrderTree(db, bidsKey, orderBook)
	asks := NewOrderTree(db, asksKey, orderBook)

	// set asks and bids
	orderBook.Bids = bids
	orderBook.Asks = asks
	// orderBook.Restore()

	// no need to update when there is no operation yet
	orderBook.UpdateTime()

	return orderBook
}

func (orderBook *OrderBook) SetDebug(debug bool) {
	orderBook.db.Debug = debug
}

func (orderBook *OrderBook) Save() error {

	orderBook.Asks.Save()
	orderBook.Bids.Save()

	// commit
	// return batch.Write()
	return orderBook.db.Put(orderBook.Key, orderBook.Item)
}

// commit everything by trigger db.Commit, later we can map custom encode and decode based on item
func (orderBook *OrderBook) Commit() error {
	return orderBook.db.Commit()
}

func (orderBook *OrderBook) Restore() error {

	orderBook.Asks.Restore()
	orderBook.Bids.Restore()

	val, err := orderBook.db.Get(orderBook.Key, orderBook.Item)
	if err == nil {
		orderBook.Item = val.(*OrderBookItem)
	}

	return err
}

func (orderBook *OrderBook) GetOrderIDFromBook(key []byte) uint64 {
	orderSlot := new(big.Int).SetBytes(key)
	return Sub(orderSlot, orderBook.slot).Uint64()
}

func (orderBook *OrderBook) GetOrderIDFromKey(key []byte) []byte {
	orderSlot := new(big.Int).SetBytes(key)
	// fmt.Println("FAIL", key, orderList.slot)
	return GetKeyFromBig(Add(orderBook.slot, orderSlot))
}

func (orderBook *OrderBook) GetOrder(key []byte) *Order {
	if orderBook.db.IsEmptyKey(key) {
		return nil
	}
	// orderID := key
	storedKey := orderBook.GetOrderIDFromKey(key)
	orderItem := &OrderItem{}
	// var orderItem *OrderItem
	val, err := orderBook.db.Get(storedKey, orderItem)
	if err != nil {
		return nil
	}

	order := &Order{
		Item: val.(*OrderItem),
		Key:  key,
	}
	return order
}

// we need to store orderBook information as well
// Volume    *big.Int `json:"volume"`    // Contains total quantity from all Orders in tree
// 	NumOrders int             `json:"numOrders"` // Contains count of Orders in tree
// 	Depth

func (orderBook *OrderBook) String(startDepth int) string {
	tabs := strings.Repeat("\t", startDepth)
	return fmt.Sprintf("%s{\n\t%sName: %s\n\t%sTimestamp: %d\n\t%sNextOrderID: %d\n\t%sBids: %s\n\t%sAsks: %s\n%s}\n",
		tabs,
		tabs, orderBook.Item.Name, tabs, orderBook.Item.Timestamp, tabs, orderBook.Item.NextOrderID,
		tabs, orderBook.Bids.String(startDepth+1), tabs, orderBook.Asks.String(startDepth+1),
		tabs)
}

// UpdateTime : update time for order book
func (orderBook *OrderBook) UpdateTime() {
	timestamp := uint64(time.Now().Unix())
	orderBook.Item.Timestamp = timestamp
}

// BestBid : get the best bid of the order book
func (orderBook *OrderBook) BestBid() (value *big.Int) {
	return orderBook.Bids.MaxPrice()
}

// BestAsk : get the best ask of the order book
func (orderBook *OrderBook) BestAsk() (value *big.Int) {
	return orderBook.Asks.MinPrice()
}

// WorstBid : get the worst bid of the order book
func (orderBook *OrderBook) WorstBid() (value *big.Int) {
	return orderBook.Bids.MinPrice()
}

// WorstAsk : get the worst ask of the order book
func (orderBook *OrderBook) WorstAsk() (value *big.Int) {
	return orderBook.Asks.MaxPrice()
}

// processMarketOrder : process the market order
func (orderBook *OrderBook) processMarketOrder(quote map[string]interface{}, verbose bool) []map[string]interface{} {
	var trades []map[string]interface{}
	quantityToTrade := ToBigInt(quote["quantity"].(string))
	side := quote["side"]
	var newTrades []map[string]interface{}
	// speedup the comparison, do not assign because it is pointer
	zero := Zero()
	if side == Bid {
		for quantityToTrade.Cmp(zero) > 0 && orderBook.Asks.NotEmpty() {
			bestPriceAsks := orderBook.Asks.MinPriceList()
			quantityToTrade, newTrades = orderBook.processOrderList(Ask, bestPriceAsks, quantityToTrade, quote, verbose)
			trades = append(trades, newTrades...)
		}
		// } else if side == Ask {
	} else {
		for quantityToTrade.Cmp(zero) > 0 && orderBook.Bids.NotEmpty() {
			bestPriceBids := orderBook.Bids.MaxPriceList()
			quantityToTrade, newTrades = orderBook.processOrderList(Bid, bestPriceBids, quantityToTrade, quote, verbose)
			trades = append(trades, newTrades...)
		}
	}
	return trades
}

// processLimitOrder : process the limit order, can change the quote
// If not care for performance, we should make a copy of quote to prevent further reference problem
func (orderBook *OrderBook) processLimitOrder(quote map[string]interface{}, verbose bool) ([]map[string]interface{}, map[string]interface{}) {
	var trades []map[string]interface{}
	quantityToTrade := ToBigInt(quote["quantity"].(string))
	side := quote["side"]
	price := ToBigInt(quote["price"].(string))

	var newTrades []map[string]interface{}
	var orderInBook map[string]interface{}
	// speedup the comparison, do not assign because it is pointer
	zero := Zero()

	if side == Bid {
		minPrice := orderBook.Asks.MinPrice()
		for quantityToTrade.Cmp(zero) > 0 && orderBook.Asks.NotEmpty() && price.Cmp(minPrice) >= 0 {
			bestPriceAsks := orderBook.Asks.MinPriceList()
			quantityToTrade, newTrades = orderBook.processOrderList(Ask, bestPriceAsks, quantityToTrade, quote, verbose)
			trades = append(trades, newTrades...)
			minPrice = orderBook.Asks.MinPrice()
		}

		if quantityToTrade.Cmp(zero) > 0 {
			quote["order_id"] = orderBook.Item.NextOrderID
			quote["quantity"] = quantityToTrade.String()
			orderBook.Bids.InsertOrder(quote)
			orderInBook = quote
		}

		// } else if side == Ask {
	} else {
		maxPrice := orderBook.Bids.MaxPrice()
		for quantityToTrade.Cmp(zero) > 0 && orderBook.Bids.NotEmpty() && price.Cmp(maxPrice) <= 0 {
			bestPriceBids := orderBook.Bids.MaxPriceList()
			quantityToTrade, newTrades = orderBook.processOrderList(Bid, bestPriceBids, quantityToTrade, quote, verbose)
			trades = append(trades, newTrades...)
			maxPrice = orderBook.Bids.MaxPrice()
		}

		if quantityToTrade.Cmp(zero) > 0 {
			quote["order_id"] = orderBook.Item.NextOrderID
			quote["quantity"] = quantityToTrade.String()
			orderBook.Asks.InsertOrder(quote)
			orderInBook = quote
		}
	}
	return trades, orderInBook
}

// ProcessOrder : process the order
func (orderBook *OrderBook) ProcessOrder(quote map[string]interface{}, verbose bool) ([]map[string]interface{}, map[string]interface{}) {
	orderType := quote["type"]
	var orderInBook map[string]interface{}
	var trades []map[string]interface{}

	orderBook.UpdateTime()
	// if we do not use auto-increment orderid, we must set price slot to avoid conflict
	orderBook.Item.NextOrderID++

	if orderType == Market {
		trades = orderBook.processMarketOrder(quote, verbose)
	} else {
		trades, orderInBook = orderBook.processLimitOrder(quote, verbose)
	}

	// update orderBook
	orderBook.Save()

	return trades, orderInBook
}

// processOrderList : process the order list
func (orderBook *OrderBook) processOrderList(side string, orderList *OrderList, quantityStillToTrade *big.Int, quote map[string]interface{}, verbose bool) (*big.Int, []map[string]interface{}) {
	quantityToTrade := CloneBigInt(quantityStillToTrade)
	// quantityToTrade := quantityStillToTrade
	var trades []map[string]interface{}
	// speedup the comparison, do not assign because it is pointer
	zero := Zero()
	// var watchDog = 0
	// fmt.Printf("CMP problem :%t - %t\n", quantityToTrade.Cmp(Zero()) > 0, IsGreaterThan(quantityToTrade, Zero()))
	for orderList.Item.Length > 0 && quantityToTrade.Cmp(zero) > 0 {

		headOrder := orderList.GetOrder(orderList.Item.HeadOrder)
		// fmt.Printf("Head :%s ,%s\n", new(big.Int).SetBytes(orderList.Item.HeadOrder), orderBook.Asks.MinPriceList().String(0))
		if headOrder == nil {
			panic("headOrder is null")
			// return Zero(), trades
		}

		tradedPrice := CloneBigInt(headOrder.Item.Price)

		var newBookQuantity *big.Int
		var tradedQuantity *big.Int

		if IsStrictlySmallerThan(quantityToTrade, headOrder.Item.Quantity) {
			tradedQuantity = CloneBigInt(quantityToTrade)
			// Do the transaction
			newBookQuantity = Sub(headOrder.Item.Quantity, quantityToTrade)
			headOrder.UpdateQuantity(orderList, newBookQuantity, headOrder.Item.Timestamp)
			quantityToTrade = Zero()

		} else if IsEqual(quantityToTrade, headOrder.Item.Quantity) {
			tradedQuantity = CloneBigInt(quantityToTrade)
			if side == Bid {
				orderBook.Bids.RemoveOrder(headOrder)
			} else {
				orderBook.Asks.RemoveOrder(headOrder)
			}
			quantityToTrade = Zero()

		} else {
			tradedQuantity = CloneBigInt(headOrder.Item.Quantity)
			if side == Bid {
				orderBook.Bids.RemoveOrder(headOrder)
			} else {
				orderBook.Asks.RemoveOrderFromOrderList(headOrder, orderList)
			}
		}

		if verbose {
			fmt.Printf("TRADE: Timestamp - %d, Price - %s, Quantity - %s, TradeID - %d, Matching TradeID - %s\n",
				orderBook.Item.Timestamp, tradedPrice, tradedQuantity, headOrder.Key, quote["trade_id"])

		}

		transactionRecord := make(map[string]interface{})
		transactionRecord["timestamp"] = orderBook.Item.Timestamp
		transactionRecord["price"] = tradedPrice.String()
		transactionRecord["quantity"] = tradedQuantity.String()

		trades = append(trades, transactionRecord)
	}
	return quantityToTrade, trades
}

// CancelOrder : cancel the order, just need ID, side and price, of course order must belong
// to a price point as well
func (orderBook *OrderBook) CancelOrder(side string, orderID uint64, price *big.Int) error {
	orderBook.UpdateTime()
	key := GetKeyFromBig(big.NewInt(int64(orderID)))
	var err error
	if side == Bid {
		order := orderBook.Bids.GetOrder(key, price)
		if order != nil {
			_, err = orderBook.Bids.RemoveOrder(order)
		}

	} else {

		order := orderBook.Asks.GetOrder(key, price)
		if order != nil {
			_, err = orderBook.Asks.RemoveOrder(order)
		}

	}

	return err
}

func (orderBook *OrderBook) UpdateOrder(quoteUpdate map[string]interface{}) error {
	orderID := quoteUpdate["order_id"].(uint64)

	price, ok := new(big.Int).SetString(quoteUpdate["price"].(string), 10)
	if !ok {
		return fmt.Errorf("price is not correct :%s", quoteUpdate["price"])
	}

	return orderBook.ModifyOrder(quoteUpdate, orderID, price)

}

// ModifyOrder : modify the order
func (orderBook *OrderBook) ModifyOrder(quoteUpdate map[string]interface{}, orderID uint64, price *big.Int) error {
	orderBook.UpdateTime()

	side := quoteUpdate["side"]
	quoteUpdate["order_id"] = orderID
	quoteUpdate["timestamp"] = orderBook.Item.Timestamp
	key := []byte(quoteUpdate["order_id"].(string))
	if side == Bid {

		if orderBook.Bids.OrderExist(key, price) {
			return orderBook.Bids.UpdateOrder(quoteUpdate)
		}

	} else {

		if orderBook.Asks.OrderExist(key, price) {
			return orderBook.Asks.UpdateOrder(quoteUpdate)
		}
	}

	return nil
}

// VolumeAtPrice : get volume at the current price
func (orderBook *OrderBook) VolumeAtPrice(side string, price *big.Int) *big.Int {
	volume := Zero()
	if side == Bid {
		if orderBook.Bids.PriceExist(price) {
			orderList := orderBook.Bids.PriceList(price)
			// incase we use cache for PriceList
			volume = CloneBigInt(orderList.Item.Volume)
		}
	} else {
		// other case
		if orderBook.Asks.PriceExist(price) {
			orderList := orderBook.Asks.PriceList(price)
			volume = CloneBigInt(orderList.Item.Volume)
		}
	}

	return volume

}
