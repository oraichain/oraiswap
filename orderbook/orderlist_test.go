package orderbook

import (
	"math/big"
	"testing"
)

func TestNewOrderList(t *testing.T) {
	orderList := NewOrderList(testPrice, testOrderTree)

	if !(orderList.Item.Length == 0) {
		t.Errorf("Orderlist length incorrect, got: %d, want: %d.", orderList.Item.Length, 0)
	}

	if orderList.Item.Price.Cmp(testPrice) != 0 {
		t.Errorf("Orderlist price incorrect, got: %d, want: %d.", orderList.Item.Price, testPrice)
	}

	if orderList.Item.Volume.Cmp(Zero()) != 0 {
		t.Errorf("Orderlist volume incorrect, got: %d, want: %d.", orderList.Item.Volume, 0)
	}
}

func TestOrderList(t *testing.T) {
	orderList := NewOrderList(testPrice, testOrderTree)
	testOrderTree.orderDB.Debug = true

	dummyOrder := make(map[string]interface{})
	dummyOrder["timestamp"] = testTimestamp
	dummyOrder["quantity"] = testQuanity.String()
	dummyOrder["price"] = testPrice.String()
	dummyOrder["order_id"] = testOrderID

	order := NewOrder(dummyOrder, orderList.Key)
	orderList.AppendOrder(order)

	if !(orderList.Item.Length == 1) {
		t.Errorf("Orderlist Length incorrect, got: %d, want: %d.", orderList.Item.Length, 1)
	}

	if orderList.Item.Price.Cmp(testPrice) != 0 {
		t.Errorf("Orderlist price incorrect, got: %d, want: %d.", orderList.Item.Price, testPrice)
	}

	if orderList.Item.Volume.Cmp(order.Item.Quantity) != 0 {
		t.Errorf("Orderlist volume incorrect, got: %d, want: %d.", orderList.Item.Volume, order.Item.Quantity)
	}

	dummyOrder1 := make(map[string]interface{})
	dummyOrder1["timestamp"] = testTimestamp1
	dummyOrder1["quantity"] = testQuanity1.String()
	dummyOrder1["price"] = testPrice1.String()
	dummyOrder1["order_id"] = testOrderID1

	order1 := NewOrder(dummyOrder1, orderList.Key)
	orderList.AppendOrder(order1)

	if !(orderList.Item.Length == 2) {
		t.Errorf("Orderlist Length incorrect, got: %d, want: %d.", orderList.Item.Length, 2)
	}

	orderListQuantity := Add(order.Item.Quantity, order1.Item.Quantity)
	if orderList.Item.Volume.Cmp(orderListQuantity) != 0 {
		t.Errorf("Orderlist Length incorrect, got: %d, want: %d.", orderList.Item.Volume, orderListQuantity)
	}

	headOrder := orderList.GetOrder(orderList.Item.HeadOrder)

	if !IsEqual(new(big.Int).SetBytes(headOrder.Key), big.NewInt(1)) {
		t.Errorf("headorder id incorrect, got: %x, want: %d.", headOrder.Key, big.NewInt(1))
	}

	nextOrder := orderList.GetOrder(headOrder.Item.NextOrder)

	if !IsEqual(new(big.Int).SetBytes(nextOrder.Key), big.NewInt(2)) {
		t.Errorf("Next headorder id incorrect, got: %x, want: %d.", nextOrder.Key, big.NewInt(2))
	}

	t.Logf("Order List : %s", orderList.String(0))
}
