package orderbook

import (
	"encoding/json"
	"fmt"
	"log"
	"math/big"
	"strings"
)

// Engine : singleton orderbook for testing
type Engine struct {
	Orderbooks map[string]*OrderBook
	db         *BatchDatabase
	// pair and max volume ...
	allowedPairs map[string]*big.Int
}

func NewEngine(datadir string, allowedPairs map[string]*big.Int) *Engine {

	batchDB := NewBatchDatabaseWithEncode(datadir, 0, 0,
		json.Marshal, json.Unmarshal)

	fixAllowedPairs := make(map[string]*big.Int)
	for key, value := range allowedPairs {
		fixAllowedPairs[strings.ToLower(key)] = value
	}

	orderbooks := &Engine{
		Orderbooks:   make(map[string]*OrderBook),
		db:           batchDB,
		allowedPairs: fixAllowedPairs,
	}

	return orderbooks
}

func (engine *Engine) GetOrderBook(pairName string) (*OrderBook, error) {
	return engine.getAndCreateIfNotExisted(pairName)
}

func (engine *Engine) hasOrderBook(name string) bool {
	_, ok := engine.Orderbooks[name]
	return ok
}

// commit for all orderbooks
func (engine *Engine) Commit() error {
	return engine.db.Commit()
}

func (engine *Engine) getAndCreateIfNotExisted(pairName string) (*OrderBook, error) {

	name := strings.ToLower(pairName)

	if !engine.hasOrderBook(name) {
		// check allow pair
		if _, ok := engine.allowedPairs[name]; !ok {
			return nil, fmt.Errorf("orderbook not found for pair :%s", pairName)
		}

		// then create one
		ob := NewOrderBook(name, engine.db)
		if ob != nil {
			ob.Restore()
			engine.Orderbooks[name] = ob
		}
	}

	// return from map
	return engine.Orderbooks[name], nil
}

func (engine *Engine) GetOrder(pairName, orderID string) *Order {
	ob, _ := engine.getAndCreateIfNotExisted(pairName)
	if ob == nil {
		return nil
	}
	key := GetKeyFromString(orderID)
	return ob.GetOrder(key)
}

func (engine *Engine) ProcessOrder(quote map[string]interface{}) ([]map[string]interface{}, map[string]interface{}) {

	ob, _ := engine.getAndCreateIfNotExisted(quote["pair_name"].(string))
	var trades []map[string]interface{}
	var orderInBook map[string]interface{}

	if ob != nil {
		// get map as general input, we can set format later to make sure there is no problem
		// insert
		if quote["order_id"].(uint64) == 0 {
			log.Println("Process order")
			trades, orderInBook = ob.ProcessOrder(quote, true)
		} else {
			log.Println("Update order")
			err := ob.UpdateOrder(quote)
			if err != nil {
				log.Println("Update order failed", "quote", quote, "err", err)
			}
		}

	}

	return trades, orderInBook

}

func (engine *Engine) CancelOrder(quote map[string]interface{}) error {
	ob, err := engine.getAndCreateIfNotExisted(quote["pair_name"].(string))
	if ob != nil {
		orderID := quote["order_id"].(uint64)
		if err == nil {

			price, ok := new(big.Int).SetString(quote["price"].(string), 10)
			if !ok {
				return fmt.Errorf("price is not correct :%s", quote["price"])
			}

			return ob.CancelOrder(quote["side"].(string), orderID, price)
		}
	}

	return err
}
