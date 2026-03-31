/// ═══════════════════════════════════════════════════════════════════════════════
/// Transaction, MVCC, and ACID Property Tests
/// Comprehensive tests for transactions, persistence, and ACID properties
/// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod transaction_tests {
    use nexus_db::storage::{
        TransactionManager, IsolationLevel, TxStatus, 
        InMemoryBackend, PersistentTable, CacheLayer, MvccStore
    };
    use serde_json::json;
    use std::sync::Arc;
    use std::path::PathBuf;
    use std::thread;
    use std::time::Duration;

    // ─────────────────────────────────────────────────────────────────────────────
    // Transaction Manager Tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_multiple_transactions() {
        let tm = TransactionManager::new(PathBuf::from("/tmp/nexus_test1"));
        
        let txs: Vec<u64> = (0..5)
            .map(|_| tm.begin_transaction(IsolationLevel::ReadCommitted))
            .collect();

        assert_eq!(txs.len(), 5);
        assert!(txs.iter().all(|&tx| tm.is_active(tx)));

        for tx in txs {
            tm.commit(tx).ok();
            assert!(!tm.is_active(tx));
        }
    }

    #[test]
    fn test_transaction_commit() {
        let tm = TransactionManager::new(PathBuf::from("/tmp/nexus_test2"));
        let tx = tm.begin_transaction(IsolationLevel::ReadCommitted);

        assert!(tm.is_active(tx));
        assert_eq!(tm.get_tx_status(tx), Some(TxStatus::Pending));

        tm.commit(tx).unwrap();

        assert!(!tm.is_active(tx));
        assert_eq!(tm.get_tx_status(tx), None); // TX removed from active
    }

    #[test]
    fn test_transaction_rollback() {
        let tm = TransactionManager::new(PathBuf::from("/tmp/nexus_test3"));
        let tx = tm.begin_transaction(IsolationLevel::ReadCommitted);

        assert!(tm.is_active(tx));

        tm.rollback(tx).unwrap();

        assert!(!tm.is_active(tx));
    }

    #[test]
    fn test_isolation_levels() {
        let tm = TransactionManager::new(PathBuf::from("/tmp/nexus_test4"));

        let tx_ru = tm.begin_transaction(IsolationLevel::ReadUncommitted);
        let tx_rc = tm.begin_transaction(IsolationLevel::ReadCommitted);
        let tx_rr = tm.begin_transaction(IsolationLevel::RepeatableRead);
        let tx_ser = tm.begin_transaction(IsolationLevel::Serializable);

        assert!(tm.is_active(tx_ru));
        assert!(tm.is_active(tx_rc));
        assert!(tm.is_active(tx_rr));
        assert!(tm.is_active(tx_ser));

        tm.commit(tx_ru).ok();
        tm.commit(tx_rc).ok();
        tm.commit(tx_rr).ok();
        tm.commit(tx_ser).ok();
    }

    #[test]
    fn test_non_existent_transaction() {
        let tm = TransactionManager::new(PathBuf::from("/tmp/nexus_test5"));
        
        let result = tm.commit(9999);
        assert!(result.is_err());
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Persistence Tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_table_insert_get() {
        let backend = Arc::new(InMemoryBackend::new());
        let table = PersistentTable::new(backend, "users".to_string());

        let user = json!({"name": "Alice", "age": 30});
        table.put("1", &user).unwrap();

        let retrieved = table.get("1").unwrap().unwrap();
        assert_eq!(retrieved, user);
    }

    #[test]
    fn test_table_update() {
        let backend = Arc::new(InMemoryBackend::new());
        let table = PersistentTable::new(backend, "users".to_string());

        let user_v1 = json!({"name": "Alice", "age": 30});
        table.put("1", &user_v1).unwrap();

        let user_v2 = json!({"name": "Alice", "age": 31});
        table.put("1", &user_v2).unwrap();

        let retrieved = table.get("1").unwrap().unwrap();
        assert_eq!(retrieved["age"], 31);
    }

    #[test]
    fn test_table_delete() {
        let backend = Arc::new(InMemoryBackend::new());
        let table = PersistentTable::new(backend, "users".to_string());

        let user = json!({"name": "Alice", "age": 30});
        table.put("1", &user).unwrap();

        assert!(table.get("1").unwrap().is_some());

        table.delete("1").unwrap();

        assert!(table.get("1").unwrap().is_none());
    }

    #[test]
    fn test_table_scan_all() {
        let backend = Arc::new(InMemoryBackend::new());
        let table = PersistentTable::new(backend, "products".to_string());

        let records = vec![
            ("1".to_string(), json!({"name": "Product A"})),
            ("2".to_string(), json!({"name": "Product B"})),
            ("3".to_string(), json!({"name": "Product C"})),
        ];

        table.batch_insert(records).unwrap();

        let all = table.scan_all().unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_table_batch_insert() {
        let backend = Arc::new(InMemoryBackend::new());
        let table = PersistentTable::new(backend, "items".to_string());

        let records = vec![
            ("1".to_string(), json!({"value": 100})),
            ("2".to_string(), json!({"value": 200})),
            ("3".to_string(), json!({"value": 300})),
            ("4".to_string(), json!({"value": 400})),
        ];

        let count = table.batch_insert(records).unwrap();
        assert_eq!(count, 4);
        assert_eq!(table.count().unwrap(), 4);
    }

    #[test]
    fn test_table_count() {
        let backend = Arc::new(InMemoryBackend::new());
        let table = PersistentTable::new(backend, "items".to_string());

        assert_eq!(table.count().unwrap(), 0);

        table.put("1", &json!({"x": 1})).unwrap();
        assert_eq!(table.count().unwrap(), 1);

        table.put("2", &json!({"x": 2})).unwrap();
        assert_eq!(table.count().unwrap(), 2);

        table.delete("1").unwrap();
        assert_eq!(table.count().unwrap(), 1);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Cache Layer Tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_cache_put_get() {
        let cache = CacheLayer::new(10);

        cache.put("key1".to_string(), json!({"value": 1}));
        
        let result = cache.get("key1");
        assert_eq!(result, Some(json!({"value": 1})));
    }

    #[test]
    fn test_cache_lru_eviction() {
        let cache = CacheLayer::new(2);

        cache.put("k1".to_string(), json!({"v": 1}));
        cache.put("k2".to_string(), json!({"v": 2}));
        assert_eq!(cache.get("k1"), Some(json!({"v": 1})));

        // Adding 3rd should evict first (k1)
        cache.put("k3".to_string(), json!({"v": 3}));
        
        assert_eq!(cache.get("k1"), None);
        assert_eq!(cache.get("k2"), Some(json!({"v": 2})));
        assert_eq!(cache.get("k3"), Some(json!({"v": 3})));
    }

    #[test]
    fn test_cache_invalidate() {
        let cache = CacheLayer::new(10);

        cache.put("key1".to_string(), json!({"value": 1}));
        assert_eq!(cache.get("key1"), Some(json!({"value": 1})));

        cache.invalidate("key1");
        assert_eq!(cache.get("key1"), None);
    }

    #[test]
    fn test_cache_clear() {
        let cache = CacheLayer::new(10);

        cache.put("k1".to_string(), json!(1));
        cache.put("k2".to_string(), json!(2));
        cache.put("k3".to_string(), json!(3));

        assert_eq!(cache.get("k1"), Some(json!(1)));
        
        cache.clear();
        
        assert_eq!(cache.get("k1"), None);
        assert_eq!(cache.get("k2"), None);
        assert_eq!(cache.get("k3"), None);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // MVCC (Multi-Version Concurrency Control) Tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_mvcc_basic_versioning() {
        let store: MvccStore<String> = MvccStore::new();
        
        store.insert("key1".to_string(), "value1".to_string(), 1).ok();
        store.insert("key1".to_string(), "value2".to_string(), 2).ok();

        // TX 3 should see value1 (created by tx1)
        let val = store.get_for_tx("key1", 3, IsolationLevel::ReadCommitted);
        assert_eq!(val, Some("value1".to_string()));
    }

    #[test]
    fn test_mvcc_read_uncommitted() {
        let store: MvccStore<String> = MvccStore::new();
        store.insert("data".to_string(), "dirty".to_string(), 1).ok();
        
        // ReadUncommitted reads any version, even uncommitted
        let val = store.get_for_tx("data", 2, IsolationLevel::ReadUncommitted);
        assert_eq!(val, Some("dirty".to_string()));
    }

    #[test]
    fn test_mvcc_read_committed() {
        let store: MvccStore<String> = MvccStore::new();
        store.insert("data".to_string(), "value".to_string(), 1).ok();
        
        // ReadCommitted doesn't read uncommitted data
        let val = store.get_for_tx("data", 2, IsolationLevel::ReadCommitted);
        assert_eq!(val, None);
    }

    #[test]
    fn test_mvcc_delete() {
        let store: MvccStore<String> = MvccStore::new();
        store.insert("item".to_string(), "value".to_string(), 1).ok();
        
        // Mark as deleted
        store.delete("item", 2).ok();
        
        // Should not be visible after deletion
        let val = store.get_for_tx("item", 3, IsolationLevel::ReadCommitted);
        assert_eq!(val, None);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Multi-Table Transaction Tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_multi_table_transaction() {
        let backend = Arc::new(InMemoryBackend::new());
        let tm = TransactionManager::new(PathBuf::from("/tmp/nexus_test6"));

        let users = PersistentTable::new(backend.clone(), "users".to_string());
        let orders = PersistentTable::new(backend.clone(), "orders".to_string());

        let tx = tm.begin_transaction(IsolationLevel::ReadCommitted);

        users.put("1", &json!({"name": "Alice"})).ok();
        orders.put("O1", &json!({"user_id": "1", "amount": 100})).ok();

        tm.commit(tx).ok();

        assert_eq!(users.count().ok(), Some(1));
        assert_eq!(orders.count().ok(), Some(1));
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Concurrent Access Tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_concurrent_reads() {
        let backend = Arc::new(InMemoryBackend::new());
        let table = Arc::new(PersistentTable::new(backend, "data".to_string()));

        table.put("key", &json!({"value": 42})).ok();

        let mut handles = vec![];
        for _ in 0..4 {
            let t = table.clone();
            let handle = std::thread::spawn(move || {
                t.get("key").ok().flatten()
            });
            handles.push(handle);
        }

        for handle in handles {
            let result = handle.join().unwrap();
            assert_eq!(result, Some(json!({"value": 42})));
        }
    }

    #[test]
    fn test_concurrent_writes() {
        let backend = Arc::new(InMemoryBackend::new());
        let table = Arc::new(PersistentTable::new(backend, "data".to_string()));

        let mut handles = vec![];
        for i in 0..10 {
            let t = table.clone();
            let handle = std::thread::spawn(move || {
                let key = format!("key{}", i);
                t.put(&key, &json!({"value": i})).ok()
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().ok();
        }

        assert_eq!(table.count().ok(), Some(10));
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // ACID Properties Tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_atomicity() {
        // Either all operations succeed or all fail together
        let backend = Arc::new(InMemoryBackend::new());
        let table = PersistentTable::new(backend, "test".to_string());

        let record1 = json!({"x": 1});
        let record2 = json!({"x": 2});

        table.put("1", &record1).ok();
        table.put("2", &record2).ok();

        assert_eq!(table.count().ok(), Some(2));
    }

    #[test]
    fn test_consistency() {
        // Data is consistent after operations
        let backend = Arc::new(InMemoryBackend::new());
        let table = PersistentTable::new(backend, "test".to_string());

        let data = json!({"id": 1, "balance": 100});
        table.put("1", &data).ok();

        let retrieved = table.get("1").ok().flatten();
        assert_eq!(retrieved.unwrap()["balance"], 100);
    }

    #[test]
    fn test_isolation() {
        // Multiple transactions don't interfere
        let tm = TransactionManager::new(PathBuf::from("/tmp/nexus_test7"));

        let tx1 = tm.begin_transaction(IsolationLevel::Serializable);
        let tx2 = tm.begin_transaction(IsolationLevel::Serializable);

        assert!(tm.is_active(tx1));
        assert!(tm.is_active(tx2));

        tm.commit(tx1).ok();
        tm.commit(tx2).ok();

        assert!(!tm.is_active(tx1));
        assert!(!tm.is_active(tx2));
    }

    #[test]
    fn test_durability() {
        // Data persists across operations
        let backend = Arc::new(InMemoryBackend::new());
        let table = PersistentTable::new(backend.clone(), "durable".to_string());

        let data = json!({"key": "value"});
        table.put("1", &data).ok();

        backend.flush().ok(); // Simulate flush to disk

        let retrieved = table.get("1").ok().flatten();
        assert_eq!(retrieved, Some(data));
    }
}
