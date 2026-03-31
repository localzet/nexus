//! Обработка индексов и оптимизация запросов

use crate::types::*;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use std::collections::{HashMap, BTreeMap, VecDeque};
use anyhow::{Result, anyhow};

/// ───────────────────────────────────────────────────────────────────────────────
/// B-TREE INDEX - Эффективный поиск по диапазонам
/// ───────────────────────────────────────────────────────────────────────────────
#[derive(Clone)]
pub struct BTreeIndex {
    // Индекс: column_value -> Vec<RecordId>
    tree: Arc<RwLock<BTreeMap<String, Vec<RecordId>>>>,
    column_name: String,
}

impl BTreeIndex {
    pub fn new(column_name: String) -> Self {
        Self {
            tree: Arc::new(RwLock::new(BTreeMap::new())),
            column_name,
        }
    }

    /// Добавление записи в индекс
    pub fn insert(&self, key: String, record_id: RecordId) {
        let mut tree = self.tree.write();
        tree.entry(key)
            .or_insert_with(Vec::new)
            .push(record_id);
    }

    /// Поиск по точному значению
    pub fn lookup(&self, key: &str) -> Vec<RecordId> {
        let tree = self.tree.read();
        tree.get(key).cloned().unwrap_or_default()
    }

    /// Range query: находит все ключи в диапазоне [start, end]
    pub fn range_query(&self, start: &str, end: &str) -> Vec<RecordId> {
        let tree = self.tree.read();
        let mut result = Vec::new();

        for (_, record_ids) in tree.range(start.to_string()..=end.to_string()) {
            result.extend(record_ids.iter().copied());
        }

        result
    }

    /// Получение всех записей
    pub fn all_records(&self) -> Vec<RecordId> {
        let tree = self.tree.read();
        tree.values().flat_map(|ids| ids.iter().copied()).collect()
    }
}

/// ───────────────────────────────────────────────────────────────────────────────
/// HASH INDEX - Быстрый lookup O(1)
/// ───────────────────────────────────────────────────────────────────────────────
#[derive(Clone)]
pub struct HashIndex {
    index: Arc<DashMap<String, Vec<RecordId>>>,
    column_name: String,
}

impl HashIndex {
    pub fn new(column_name: String) -> Self {
        Self {
            index: Arc::new(DashMap::new()),
            column_name,
        }
    }

    /// Быстрый поиск по ключу
    pub fn lookup(&self, key: &str) -> Vec<RecordId> {
        self.index
            .get(key)
            .map(|entry| entry.clone())
            .unwrap_or_default()
    }

    /// Добавление записи
    pub fn insert(&self, key: String, record_id: RecordId) {
        self.index
            .entry(key)
            .or_insert_with(Vec::new)
            .push(record_id);
    }

    /// Удаление записи
    pub fn remove(&self, key: &str, record_id: RecordId) -> bool {
        if let Some(mut entry) = self.index.get_mut(key) {
            entry.retain(|&id| id != record_id);
            entry.is_empty()
        } else {
            false
        }
    }
}

/// ───────────────────────────────────────────────────────────────────────────────
/// BLOOM FILTER - Вероятностная структура для быстрого отрицания
/// ───────────────────────────────────────────────────────────────────────────────
#[derive(Clone)]
pub struct BloomFilter {
    bits: Arc<RwLock<Vec<bool>>>,
    hash_functions: usize,
    size: usize,
}

impl BloomFilter {
    pub fn new(estimated_items: usize, false_positive_rate: f64) -> Self {
        let optimal_size = Self::optimal_size(estimated_items, false_positive_rate);
        let hash_functions = Self::optimal_hash_functions(optimal_size, estimated_items);

        Self {
            bits: Arc::new(RwLock::new(vec![false; optimal_size])),
            hash_functions,
            size: optimal_size,
        }
    }

    /// Добавление элемента
    pub fn insert(&self, item: &str) {
        let mut bits = self.bits.write();
        for i in 0..self.hash_functions {
            let hash = self.hash(item, i as u32) % self.size;
            bits[hash] = true;
        }
    }

    /// Проверка может ли быть элемент (может быть false positive)
    pub fn might_contain(&self, item: &str) -> bool {
        let bits = self.bits.read();
        for i in 0..self.hash_functions {
            let hash = self.hash(item, i as u32) % self.size;
            if !bits[hash] {
                return false;  // Точно нету
            }
        }
        true  // Может быть, но не уверены
    }

    fn hash(&self, item: &str, seed: u32) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        item.hash(&mut hasher);
        seed.hash(&mut hasher);
        hasher.finish() as usize
    }

    fn optimal_size(items: usize, fp_rate: f64) -> usize {
        let ln2_squared = 0.4804530139592104;
        let size = ((items as f64 * fp_rate.ln().abs()) / ln2_squared) as usize;
        size.max(8)
    }

    fn optimal_hash_functions(size: usize, items: usize) -> usize {
        ((size as f64 / items as f64) * 2.0_f64.ln()).ceil() as usize
    }
}

/// ───────────────────────────────────────────────────────────────────────────────
/// LRU QUERY CACHE - Кеширование результатов запросов
/// ───────────────────────────────────────────────────────────────────────────────
#[derive(Clone)]
struct CacheEntry {
    result: QueryResult,
    timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct QueryCache {
    cache: Arc<RwLock<VecDeque<(String, CacheEntry)>>>,
    max_entries: usize,
    ttl_seconds: u64,
}

impl QueryCache {
    pub fn new(max_entries: usize, ttl_seconds: u64) -> Self {
        Self {
            cache: Arc::new(RwLock::new(VecDeque::new())),
            max_entries,
            ttl_seconds,
        }
    }

    /// Получить результат из кеша
    pub fn get(&self, query: &str) -> Option<QueryResult> {
        let mut cache = self.cache.write();
        let now = chrono::Utc::now();

        // Ищем и проверяем TTL
        for (cached_query, entry) in cache.iter() {
            if cached_query == query {
                let age = (now - entry.timestamp).num_seconds() as u64;
                if age < self.ttl_seconds {
                    return Some(entry.result.clone());
                } else {
                    // Удаляем старый результат
                    cache.retain(|(q, _)| q != query);
                    break;
                }
            }
        }

        None
    }

    /// Сохранить результат в кеш
    pub fn put(&self, query: String, result: QueryResult) {
        let mut cache = self.cache.write();

        // Удаляем старые entries если кеш полон
        while cache.len() >= self.max_entries {
            cache.pop_front();
        }

        cache.push_back((query, CacheEntry {
            result,
            timestamp: chrono::Utc::now(),
        }));
    }

    /// Очистка кеша
    pub fn clear(&self) {
        self.cache.write().clear();
    }

    /// Статистика кеша
    pub fn stats(&self) -> (usize, usize) {
        let cache = self.cache.read();
        (cache.len(), self.max_entries)
    }
}

/// ───────────────────────────────────────────────────────────────────────────────
/// INDEXING MANAGER - Управляет всеми индексами
/// ───────────────────────────────────────────────────────────────────────────────
pub struct IndexingManager {
    // table_name -> column_name -> index
    btree_indices: Arc<DashMap<String, Arc<DashMap<String, BTreeIndex>>>>,
    hash_indices: Arc<DashMap<String, Arc<DashMap<String, HashIndex>>>>,
    bloom_filters: Arc<DashMap<String, Arc<DashMap<String, BloomFilter>>>>,
    query_cache: Arc<QueryCache>,
}

impl IndexingManager {
    pub fn new() -> Self {
        Self {
            btree_indices: Arc::new(DashMap::new()),
            hash_indices: Arc::new(DashMap::new()),
            bloom_filters: Arc::new(DashMap::new()),
            query_cache: Arc::new(QueryCache::new(1000, 300)), // 300 секунд TTL
        }
    }

    /// Создание B-tree индекса для таблицы
    pub fn create_btree_index(&self, table_name: String, column_name: String) -> Result<()> {
        let table_indices = self
            .btree_indices
            .entry(table_name.clone())
            .or_insert_with(|| Arc::new(DashMap::new()))
            .clone();

        table_indices.insert(column_name.clone(), BTreeIndex::new(column_name));
        Ok(())
    }

    /// Создание Hash индекса для таблицы (обычно для primary key)
    pub fn create_hash_index(&self, table_name: String, column_name: String) -> Result<()> {
        let table_indices = self
            .hash_indices
            .entry(table_name.clone())
            .or_insert_with(|| Arc::new(DashMap::new()))
            .clone();

        table_indices.insert(column_name.clone(), HashIndex::new(column_name));
        Ok(())
    }

    /// Создание Bloom filter для быстрого отрицания
    pub fn create_bloom_filter(
        &self,
        table_name: String,
        column_name: String,
        estimated_items: usize,
    ) -> Result<()> {
        let table_filters = self
            .bloom_filters
            .entry(table_name.clone())
            .or_insert_with(|| Arc::new(DashMap::new()))
            .clone();

        table_filters.insert(
            column_name,
            BloomFilter::new(estimated_items, 0.01), // 1% false positive
        );
        Ok(())
    }

    /// Получить B-tree индекс
    pub fn get_btree_index(&self, table_name: &str, column_name: &str) -> Option<BTreeIndex> {
        self.btree_indices
            .get(table_name)
            .and_then(|indices| indices.get(column_name).map(|idx| idx.clone()))
    }

    /// Получить Hash индекс
    pub fn get_hash_index(&self, table_name: &str, column_name: &str) -> Option<HashIndex> {
        self.hash_indices
            .get(table_name)
            .and_then(|indices| indices.get(column_name).map(|idx| idx.clone()))
    }

    /// Получить Bloom filter
    pub fn get_bloom_filter(&self, table_name: &str, column_name: &str) -> Option<BloomFilter> {
        self.bloom_filters
            .get(table_name)
            .and_then(|filters| filters.get(column_name).map(|bf| bf.clone()))
    }

    /// Кешировать результат запроса
    pub fn cache_query_result(&self, query: String, result: QueryResult) {
        self.query_cache.put(query, result);
    }

    /// Получить результат из кеша
    pub fn get_cached_result(&self, query: &str) -> Option<QueryResult> {
        self.query_cache.get(query)
    }

    /// Очистить весь кеш
    pub fn clear_cache(&self) {
        self.query_cache.clear();
    }

    /// Получить статистику кеша
    pub fn cache_stats(&self) -> (usize, usize) {
        self.query_cache.stats()
    }

    /// Список всех индексов таблицы
    pub fn list_indices(&self, table_name: &str) -> Vec<(String, String)> {
        let mut indices = Vec::new();

        if let Some(btree) = self.btree_indices.get(table_name) {
            for entry in btree.iter() {
                indices.push((entry.key().clone(), "btree".to_string()));
            }
        }

        if let Some(hash) = self.hash_indices.get(table_name) {
            for entry in hash.iter() {
                indices.push((entry.key().clone(), "hash".to_string()));
            }
        }

        indices
    }
}

impl Default for IndexingManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_btree_index() {
        let idx = BTreeIndex::new("age".to_string());
        
        idx.insert("25".to_string(), RecordId::new());
        idx.insert("30".to_string(), RecordId::new());
        idx.insert("35".to_string(), RecordId::new());

        let results = idx.lookup("30");
        assert_eq!(results.len(), 1);

        let range = idx.range_query("25", "35");
        assert_eq!(range.len(), 3);
    }

    #[test]
    fn test_hash_index() {
        let idx = HashIndex::new("email".to_string());
        let id = RecordId::new();

        idx.insert("user@example.com".to_string(), id);
        let results = idx.lookup("user@example.com");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], id);
    }

    #[test]
    fn test_bloom_filter() {
        let bf = BloomFilter::new(100, 0.01);

        bf.insert("apple");
        bf.insert("banana");
        bf.insert("cherry");

        assert!(bf.might_contain("apple"));
        assert!(bf.might_contain("banana"));
        assert!(bf.might_contain("cherry"));
        // "grape" может быть false positive, но обычно нет
        // assert!(!bf.might_contain("grape")); // Not guaranteed
    }

    #[test]
    fn test_query_cache() {
        let cache = QueryCache::new(10, 60);
        let mut result = QueryResult::new();
        result.row_count = 5;

        cache.put("SELECT * FROM users".to_string(), result.clone());
        let cached = cache.get("SELECT * FROM users");

        assert!(cached.is_some());
        assert_eq!(cached.unwrap().row_count, 5);
    }

    #[test]
    fn test_indexing_manager() {
        let manager = IndexingManager::new();

        manager.create_btree_index("users".to_string(), "name".to_string()).unwrap();
        manager.create_hash_index("users".to_string(), "id".to_string()).unwrap();

        let indices = manager.list_indices("users");
        assert_eq!(indices.len(), 2);
    }
}
