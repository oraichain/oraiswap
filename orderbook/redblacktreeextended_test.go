package orderbook

import (
	"math/big"
	"testing"
	"time"
)

func print(tree *RedBlackTreeExtended, t *testing.T) {
	max, _ := tree.GetMax()
	min, _ := tree.GetMin()
	t.Logf("Value for max key: %s \n", max)
	t.Logf("Value for min key: %s \n", min)
	t.Log(tree)
}

func getBig(value string) []byte {
	bigValue, _ := new(big.Int).SetString(value, 10)
	return GetKeyFromBig(bigValue)
}

func TestManipulateLevelDBTree(t *testing.T) {
	tree := NewRedBlackTreeExtended(testDB)
	testDB.Debug = true
	start := time.Now()
	tree.Put(getBig("1"), []byte("a")) // 1->a (in order)
	tree.Put(getBig("2"), []byte("b")) // 1->a, 2->b (in order)
	tree.Put(getBig("3"), []byte("c")) // 1->a, 2->b, 3->c (in order)
	tree.Put(getBig("4"), []byte("d")) // 1->a, 2->b, 3->c, 4->d (in order)
	tree.Put(getBig("5"), []byte("e")) // 1->a, 2->b, 3->c, 4->d, 5->e (in order)

	t.Logf("Done operation took: %v", time.Since(start))

	print(tree, t)

	// Value for max key: e
	// Value for min key: a
	// RedBlackTree
	// │       ┌── 5
	// │   ┌── 4
	// │   │   └── 3
	// └── 2
	//     └── 1

	tree.RemoveMin() // 2->b, 3->c, 4->d, 5->e (in order)
	tree.RemoveMax() // 2->b, 3->c, 4->d (in order)
	print(tree, t)

	tree.RemoveMin() // 3->c, 4->d (in order)
	print(tree, t)

	// Value for max key: d
	// Value for min key: c
	// RedBlackTree
	//	│   ┌── 4
	//  └── 3

	testDB.Commit()
}

func TestRestoreLevelDBTree(t *testing.T) {
	tree := NewRedBlackTreeExtended(testDB)

	tree.SetRootKey(getBig("3"), 3)

	tree.RemoveMax() // 3->c (in order)

	print(tree, t)

	// Value for max key: c
	// Value for min key: c
	// RedBlackTree
	// └── 3
}
