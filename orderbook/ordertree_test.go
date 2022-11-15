package orderbook

import (
	"testing"
)

func TestNewOrderTree(t *testing.T) {
	orderTree := testOrderTree

	dummyOrder := make(map[string]interface{})
	dummyOrder["timestamp"] = testTimestamp
	dummyOrder["quantity"] = testQuanity.String()
	dummyOrder["price"] = testPrice.String()
	dummyOrder["order_id"] = testOrderID

	dummyOrder1 := make(map[string]interface{})
	dummyOrder1["timestamp"] = testTimestamp1
	dummyOrder1["quantity"] = testQuanity1.String()
	dummyOrder1["price"] = testPrice1.String()
	dummyOrder1["order_id"] = testOrderID1

	dummyOrder2 := make(map[string]interface{})
	dummyOrder2["timestamp"] = testTimestamp2
	dummyOrder2["quantity"] = testQuanity2.String()
	dummyOrder2["price"] = testPrice2.String()
	dummyOrder2["order_id"] = testOrderID2

	dummyOrder3 := make(map[string]interface{})
	dummyOrder3["timestamp"] = testTimestamp3
	dummyOrder3["quantity"] = testQuanity3.String()
	dummyOrder3["price"] = testPrice3.String()
	dummyOrder3["order_id"] = testOrderID3

	orderTree.InsertOrder(dummyOrder)
	orderTree.InsertOrder(dummyOrder1)

	orderTree.InsertOrder(dummyOrder2)
	orderTree.InsertOrder(dummyOrder3)

	maxPrice := orderTree.MaxPrice()
	minPrice := orderTree.MinPrice()
	if maxPrice.Cmp(testPrice3) != 0 {
		t.Errorf("orderTree.MaxPrice incorrect, got: %s, want: %s.", maxPrice, testPrice3)
	}

	if minPrice.Cmp(testPrice) != 0 {
		t.Errorf("orderTree.MinPrice incorrect, got: %s, want: %s.", minPrice, testPrice)
	}

	t.Logf("OrderTree : %s", orderTree.String(0))

}
