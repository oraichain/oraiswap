package orderbook

import (
	"bytes"
	"encoding/json"
	"fmt"

	lru "github.com/hashicorp/golang-lru/v2"
	"github.com/syndtr/goleveldb/leveldb"
)

const (
	defaultCacheLimit = 1024
	defaultMaxPending = 1024
)

// BatchItem : currently we do not support deletion batch, so ignore it
type BatchItem struct {
	Value interface{}
	// Deleted bool
}

type BatchDatabase struct {
	db             *leveldb.DB
	itemCacheLimit int
	itemMaxPending int
	emptyKey       []byte
	pendingItems   map[string]*BatchItem
	cacheItems     *lru.Cache[string, interface{}] // Cache for reading
	Debug          bool

	EncodeToBytes EncodeToBytes
	DecodeBytes   DecodeBytes
}

// NewBatchDatabase use rlp as encoding
func NewBatchDatabase(datadir string, cacheLimit, maxPending int) *BatchDatabase {
	return NewBatchDatabaseWithEncode(datadir, cacheLimit, maxPending, json.Marshal, json.Unmarshal)
}

// batchdatabase is a fast cache db to retrieve in-mem object
func NewBatchDatabaseWithEncode(datadir string, cacheLimit, maxPending int, encode EncodeToBytes, decode DecodeBytes) *BatchDatabase {
	db, err := leveldb.OpenFile(datadir, nil)
	if err != nil {
		return nil
	}

	itemCacheLimit := defaultCacheLimit
	if cacheLimit > 0 {
		itemCacheLimit = cacheLimit
	}
	itemMaxPending := defaultMaxPending
	if maxPending > 0 {
		itemMaxPending = maxPending
	}

	cacheItems, _ := lru.New[string, interface{}](defaultCacheLimit)

	batchDB := &BatchDatabase{
		db:             db,
		EncodeToBytes:  encode,
		DecodeBytes:    decode,
		itemCacheLimit: itemCacheLimit,
		itemMaxPending: itemMaxPending,
		cacheItems:     cacheItems,
		emptyKey:       EmptyKey(), // pre alloc for comparison
		pendingItems:   make(map[string]*BatchItem),
	}

	return batchDB

}

func (db *BatchDatabase) Close() error {
	return db.db.Close()
}

func (db *BatchDatabase) IsEmptyKey(key []byte) bool {
	return len(key) == 0 || bytes.Equal(key, db.emptyKey)
}

func (db *BatchDatabase) Has(key []byte) (bool, error) {
	if db.IsEmptyKey(key) {
		return false, nil
	}
	cacheKey := string(key)

	// has in pending and is not deleted
	if _, ok := db.pendingItems[cacheKey]; ok { // && !pendingItem.Deleted {
		return true, nil
	}

	if db.cacheItems.Contains(cacheKey) {
		return true, nil
	}

	return db.db.Has(key, nil)
}

func (db *BatchDatabase) Get(key []byte, val interface{}) (interface{}, error) {

	if db.IsEmptyKey(key) {
		// return nil, fmt.Errorf("Key is invalid :%x", key)
		return nil, nil
	}

	cacheKey := string(key)

	if pendingItem, ok := db.pendingItems[cacheKey]; ok {
		// if pendingItem.Deleted {
		// 	return nil, nil
		// }
		// we get value from the pending item
		return pendingItem.Value, nil
	}

	if cached, ok := db.cacheItems.Get(cacheKey); ok {
		val = cached
		if db.Debug {
			fmt.Println("Cache hit :", cacheKey)
		}
	} else {

		// we can use lru for retrieving cache item, by default leveldb support get data from cache
		// but it is raw bytes
		bytes, err := db.db.Get(key, nil)
		if err != nil {
			// fmt.Println("DONE !!!!", cacheKey, err)
			if db.Debug {
				fmt.Printf("Key not found :%x\n", key)
			}
			return nil, err
		}

		err = db.DecodeBytes(bytes, val)

		// has problem here
		if err != nil {
			return nil, err
		}

		// update cache when reading
		db.cacheItems.Add(cacheKey, val)
		// fmt.Println("DONE !!!!", cacheKey, val, err)

	}

	return val, nil
}

func (db *BatchDatabase) Put(key []byte, val interface{}) error {

	cacheKey := string(key)

	// fmt.Println("PUT", cacheKey, val)
	db.pendingItems[cacheKey] = &BatchItem{Value: val}

	if len(db.pendingItems) >= db.itemMaxPending {
		return db.Commit()
	}

	return nil
}

func (db *BatchDatabase) Delete(key []byte, force bool) error {

	// by default, we force delete both db and cache,
	// for better performance, we can mark a Deleted flag, to do batch delete
	cacheKey := string(key)

	// force delete everything
	if force {
		delete(db.pendingItems, cacheKey)
		db.cacheItems.Remove(cacheKey)
	} else {
		if _, ok := db.pendingItems[cacheKey]; ok {
			// item.Deleted = true
			db.db.Delete(key, nil)

			// delete from pending Items
			delete(db.pendingItems, cacheKey)
			// remove cache key as well
			db.cacheItems.Remove(cacheKey)
			return nil
		}
	}

	// cache not found, or force delete, must delete from database
	return db.db.Delete(key, nil)
}

func (db *BatchDatabase) Commit() error {

	batch := new(leveldb.Batch)
	for key, item := range db.pendingItems {

		value, err := db.EncodeToBytes(item.Value)
		if err != nil {
			fmt.Println(err)
			return err
		}

		batch.Put([]byte(key), value)

		if db.Debug {
			fmt.Printf("Save %x, value :%s\n", key, ToJSON(item.Value))
		}
	}
	// commit pending items does not affect the cache
	db.pendingItems = make(map[string]*BatchItem)
	// db.cacheItems.Purge()
	return db.db.Write(batch, nil)
}
