//! Advanced Caching & Buffer Management - LRU, Кеш
/// LRU cache, query result caching, buffer pools
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::Mutex;

/// Cache entry with metadata
#[derive(Debug, Clone)]
struct CacheEntry<K, V> {
    key: K,
    value: V,
    access_count: u64,
    last_accessed_at: u64,
    insertion_time: u64,
}

/// LRU (Least Recently Used) Cache
#[derive(Debug)]
pub struct LruCache<K: Clone + PartialEq + Eq + std::hash::Hash, V: Clone> {
    capacity: usize,
    cache: HashMap<K, V>,
    lru_queue: VecDeque<K>,
}

impl<K: Clone + PartialEq + Eq + std::hash::Hash, V: Clone> LruCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        if capacity == 0 {
            panic!("Cache capacity must be > 0");
        }
        Self {
            capacity,
            cache: HashMap::new(),
            lru_queue: VecDeque::new(),
        }
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        if let Some(value) = self.cache.get(key) {
            // Move to front (most recently used)
            if let Some(pos) = self.lru_queue.iter().position(|k| k == key) {
                self.lru_queue.remove(pos);
            }
            self.lru_queue.push_back(key.clone());
            Some(value)
        } else {
            None
        }
    }

    pub fn put(&mut self, key: K, value: V) {
        if self.cache.contains_key(&key) {
            self.cache.insert(key.clone(), value);
            if let Some(pos) = self.lru_queue.iter().position(|k| k == &key) {
                self.lru_queue.remove(pos);
            }
            self.lru_queue.push_back(key);
        } else {
            if self.cache.len() >= self.capacity {
                if let Some(lru_key) = self.lru_queue.pop_front() {
                    self.cache.remove(&lru_key);
                }
            }
            self.cache.insert(key.clone(), value);
            self.lru_queue.push_back(key);
        }
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        if let Some(pos) = self.lru_queue.iter().position(|k| k == key) {
            self.lru_queue.remove(pos);
        }
        self.cache.remove(key)
    }

    pub fn size(&self) -> usize {
        self.cache.len()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.lru_queue.clear();
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.cache.contains_key(key)
    }
}

/// Buffer pool page
#[derive(Debug, Clone)]
pub struct BufferPoolPage {
    pub page_id: u32,
    pub data: Vec<u8>,
    pub pin_count: u32,
    pub dirty: bool,
    pub last_accessed: u64,
}

impl BufferPoolPage {
    pub fn new(page_id: u32, data: Vec<u8>) -> Self {
        Self {
            page_id,
            data,
            pin_count: 0,
            dirty: false,
            last_accessed: 0,
        }
    }

    pub fn pin(&mut self) {
        self.pin_count += 1;
    }

    pub fn unpin(&mut self) {
        if self.pin_count > 0 {
            self.pin_count -= 1;
        }
    }

    pub fn is_pinned(&self) -> bool {
        self.pin_count > 0
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }
}

/// Buffer pool manager
#[derive(Debug)]
pub struct BufferPool {
    pages: HashMap<u32, BufferPoolPage>,
    max_pages: u32,
    frame_count: u32,
}

impl BufferPool {
    pub fn new(max_pages: u32) -> Self {
        Self {
            pages: HashMap::new(),
            max_pages,
            frame_count: 0,
        }
    }

    pub fn pin_page(&mut self, page_id: u32) -> Option<&BufferPoolPage> {
        if let Some(page) = self.pages.get_mut(&page_id) {
            page.pin();
            Some(page)
        } else {
            None
        }
    }

    pub fn unpin_page(&mut self, page_id: u32) -> bool {
        if let Some(page) = self.pages.get_mut(&page_id) {
            page.unpin();
            true
        } else {
            false
        }
    }

    pub fn add_page(&mut self, page: BufferPoolPage) -> anyhow::Result<()> {
        if self.pages.len() as u32 >= self.max_pages {
            return Err(anyhow::anyhow!("Buffer pool full"));
        }
        self.pages.insert(page.page_id, page);
        self.frame_count += 1;
        Ok(())
    }

    pub fn get_page(&self, page_id: u32) -> Option<&BufferPoolPage> {
        self.pages.get(&page_id)
    }

    pub fn get_page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn flush_dirty_pages(&mut self) -> Vec<u32> {
        let mut flushed = Vec::new();
        for (id, page) in self.pages.iter_mut() {
            if page.dirty {
                page.mark_clean();
                flushed.push(*id);
            }
        }
        flushed
    }
}

/// Query result cache
#[derive(Debug, Clone)]
pub struct QueryResultCache {
    cache: Arc<Mutex<LruCache<String, Vec<Vec<String>>>>>,
}

impl QueryResultCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(capacity))),
        }
    }

    pub fn get(&self, query_hash: &str) -> Option<Vec<Vec<String>>> {
        let mut cache = self.cache.lock().unwrap();
        cache.get(&query_hash.to_string()).cloned()
    }

    pub fn put(&self, query_hash: String, results: Vec<Vec<String>>) {
        let mut cache = self.cache.lock().unwrap();
        cache.put(query_hash, results);
    }

    pub fn invalidate(&self, query_hash: &str) {
        let mut cache = self.cache.lock().unwrap();
        cache.remove(&query_hash.to_string());
    }

    pub fn size(&self) -> usize {
        let cache = self.cache.lock().unwrap();
        cache.size()
    }

    pub fn clear(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

impl CacheStats {
    pub fn new() -> Self {
        Self {
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }

    pub fn hit_ratio(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64) / (total as f64)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_cache_creation() {
        let cache: LruCache<String, i32> = LruCache::new(3);
        assert_eq!(cache.capacity(), 3);
        assert_eq!(cache.size(), 0);
    }

    #[test]
    fn test_lru_cache_put_get() {
        let mut cache: LruCache<String, i32> = LruCache::new(3);
        cache.put("key1".to_string(), 100);
        assert_eq!(cache.get(&"key1".to_string()), Some(&100));
    }

    #[test]
    fn test_lru_cache_eviction() {
        let mut cache: LruCache<String, i32> = LruCache::new(2);
        cache.put("key1".to_string(), 1);
        cache.put("key2".to_string(), 2);
        cache.put("key3".to_string(), 3);
        
        assert_eq!(cache.size(), 2);
        assert!(cache.get(&"key1".to_string()).is_none());
    }

    #[test]
    fn test_lru_cache_contains_key() {
        let mut cache: LruCache<String, String> = LruCache::new(1);
        cache.put("key1".to_string(), "value1".to_string());
        assert!(cache.contains_key(&"key1".to_string()));
    }

    #[test]
    fn test_lru_cache_remove() {
        let mut cache: LruCache<String, i32> = LruCache::new(3);
        cache.put("key1".to_string(), 100);
        let removed = cache.remove(&"key1".to_string());
        assert_eq!(removed, Some(100));
    }

    #[test]
    fn test_lru_cache_clear() {
        let mut cache: LruCache<String, i32> = LruCache::new(3);
        cache.put("key1".to_string(), 1);
        cache.put("key2".to_string(), 2);
        cache.clear();
        assert_eq!(cache.size(), 0);
    }

    #[test]
    fn test_buffer_pool_page_creation() {
        let page = BufferPoolPage::new(1, vec![1, 2, 3, 4]);
        assert_eq!(page.page_id, 1);
        assert!(!page.dirty);
    }

    #[test]
    fn test_buffer_pool_page_pin_unpin() {
        let mut page = BufferPoolPage::new(1, vec![]);
        assert!(!page.is_pinned());
        page.pin();
        assert!(page.is_pinned());
        page.unpin();
        assert!(!page.is_pinned());
    }

    #[test]
    fn test_buffer_pool_page_marks() {
        let mut page = BufferPoolPage::new(1, vec![]);
        assert!(!page.dirty);
        page.mark_dirty();
        assert!(page.dirty);
        page.mark_clean();
        assert!(!page.dirty);
    }

    #[test]
    fn test_buffer_pool_creation() {
        let pool = BufferPool::new(10);
        assert_eq!(pool.max_pages, 10);
        assert_eq!(pool.get_page_count(), 0);
    }

    #[test]
    fn test_buffer_pool_add_page() -> anyhow::Result<()> {
        let mut pool = BufferPool::new(10);
        let page = BufferPoolPage::new(1, vec![1, 2, 3]);
        pool.add_page(page)?;
        assert_eq!(pool.get_page_count(), 1);
        Ok(())
    }

    #[test]
    fn test_buffer_pool_pin_page() -> anyhow::Result<()> {
        let mut pool = BufferPool::new(10);
        let page = BufferPoolPage::new(1, vec![]);
        pool.add_page(page)?;
        pool.pin_page(1);
        assert!(pool.get_page(1).unwrap().is_pinned());
        Ok(())
    }

    #[test]
    fn test_buffer_pool_flush_dirty() -> anyhow::Result<()> {
        let mut pool = BufferPool::new(10);
        let mut page = BufferPoolPage::new(1, vec![]);
        page.mark_dirty();
        pool.add_page(page)?;
        
        let flushed = pool.flush_dirty_pages();
        assert_eq!(flushed.len(), 1);
        Ok(())
    }

    #[test]
    fn test_query_result_cache_creation() {
        let cache = QueryResultCache::new(5);
        assert_eq!(cache.size(), 0);
    }

    #[test]
    fn test_query_result_cache_put_get() {
        let cache = QueryResultCache::new(5);
        let results = vec![vec!["col1".to_string(), "col2".to_string()]];
        cache.put("query1".to_string(), results.clone());
        assert_eq!(cache.get("query1"), Some(results));
    }

    #[test]
    fn test_query_result_cache_invalidate() {
        let cache = QueryResultCache::new(5);
        let results = vec![vec!["col1".to_string()]];
        cache.put("query1".to_string(), results);
        cache.invalidate("query1");
        assert!(cache.get("query1").is_none());
    }

    #[test]
    fn test_cache_stats() {
        let mut stats = CacheStats::new();
        stats.hits = 75;
        stats.misses = 25;
        assert!((stats.hit_ratio() - 0.75).abs() < 0.001);
    }
}
