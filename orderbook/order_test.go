package orderbook

import (
	"bytes"
	"math/big"
	"testing"
)

func TestNewOrder(t *testing.T) {

	dummyOrder := make(map[string]interface{})
	dummyOrder["timestamp"] = testTimestamp
	dummyOrder["quantity"] = testQuanity.String()
	dummyOrder["price"] = testPrice.String()
	dummyOrder["order_id"] = testOrderID

	priceKey := GetKeyFromBig(testPrice)
	order := NewOrder(dummyOrder, priceKey)

	t.Logf("Order : %s", order)

	if !(order.Item.Timestamp == testTimestamp) {
		t.Errorf("Timesmape incorrect, got: %d, want: %d.", order.Item.Timestamp, testTimestamp)
	}

	if order.Item.Quantity.Cmp(testQuanity) != 0 {
		t.Errorf("quantity incorrect, got: %d, want: %d.", order.Item.Quantity, testQuanity)
	}

	if order.Item.Price.Cmp(testPrice) != 0 {
		t.Errorf("price incorrect, got: %d, want: %d.", order.Item.Price, testPrice)
	}

	if !bytes.Equal(order.Key, new(big.Int).SetUint64(dummyOrder["order_id"].(uint64)).Bytes()) {
		t.Errorf("order id incorrect, got: %x, want: %d.", order.Key, testOrderID)
	}

}

func TestOrder(t *testing.T) {
	orderList := NewOrderList(testPrice, testOrderTree)

	dummyOrder := make(map[string]interface{})
	dummyOrder["timestamp"] = testTimestamp
	dummyOrder["quantity"] = testQuanity.String()
	dummyOrder["price"] = testPrice.String()
	dummyOrder["order_id"] = testOrderID

	order := NewOrder(dummyOrder, orderList.Key)
	orderList.AppendOrder(order)
	order.UpdateQuantity(orderList, testQuanity1, testTimestamp1)

	if order.Item.Quantity.Cmp(testQuanity1) != 0 {
		t.Errorf("order id incorrect, got: %s, want: %d.", order.Key, testOrderID)
	}

	if !(order.Item.Timestamp == testTimestamp1) {
		t.Errorf("trade id incorrect, got: %d, want: %d.", order.Item.Timestamp, testTimestamp1)
	}

	// log in json format
	var i int64 = 4
	for ; i < 10; i++ {
		increment := big.NewInt(i)
		dummyOrder1 := make(map[string]interface{})
		dummyOrder1["timestamp"] = testTimestamp1
		dummyOrder1["quantity"] = testQuanity1.String()
		dummyOrder1["price"] = Add(testPrice1, increment).String()
		dummyOrder1["order_id"] = increment.Uint64()

		order1 := NewOrder(dummyOrder1, orderList.Key)
		orderList.AppendOrder(order1)
	}

	t.Logf("Order List : %s", orderList.String(0))
}
