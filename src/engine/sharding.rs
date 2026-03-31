//! Sharding и нормализация данных - Consistent hashing
/// Consistent hashing, shard key management, range-based sharding
use std::collections::{HashMap, BTreeMap};
use anyhow::{Result, anyhow};

/// Shard ID type
pub type ShardKeyValue = String;

/// Shard range definition
#[derive(Debug, Clone)]
pub struct ShardRange {
    pub shard_id: u32,
    pub start_key: Option<ShardKeyValue>,
    pub end_key: Option<ShardKeyValue>,
}

impl ShardRange {
    pub fn new(shard_id: u32, start_key: Option<ShardKeyValue>, end_key: Option<ShardKeyValue>) -> Self {
        Self {
            shard_id,
            start_key,
            end_key,
        }
    }

    pub fn contains_key(&self, key: &str) -> bool {
        if let Some(ref start) = self.start_key {
            if key < start.as_str() {
                return false;
            }
        }
        if let Some(ref end) = self.end_key {
            if key >= end.as_str() {
                return false;
            }
        }
        true
    }
}

/// Sharding strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShardingStrategy {
    Range,
    Hash,
    List,
    Composite,
}

impl ShardingStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            ShardingStrategy::Range => "Range",
            ShardingStrategy::Hash => "Hash",
            ShardingStrategy::List => "List",
            ShardingStrategy::Composite => "Composite",
        }
    }
}

/// Consistent hash ring for distributing keys
#[derive(Debug, Clone)]
pub struct ConsistentHashRing {
    nodes: BTreeMap<u32, String>,
    virtual_nodes: u32,
}

impl ConsistentHashRing {
    pub fn new(virtual_nodes: u32) -> Self {
        Self {
            nodes: BTreeMap::new(),
            virtual_nodes,
        }
    }

    fn hash_key(key: &str) -> u32 {
        // Simple hash function
        let mut hash: u32 = 5381;
        for byte in key.bytes() {
            hash = ((hash << 5).wrapping_add(hash)).wrapping_add(byte as u32);
        }
        hash
    }

    pub fn add_node(&mut self, node_id: String) -> Result<()> {
        // Add virtual nodes to the ring
        for i in 0..self.virtual_nodes {
            let virtual_key = format!("{}:{}", node_id, i);
            let hash = Self::hash_key(&virtual_key);
            self.nodes.insert(hash, node_id.clone());
        }
        Ok(())
    }

    pub fn remove_node(&mut self, node_id: &str) -> Result<()> {
        let mut hashes_to_remove = Vec::new();
        
        for i in 0..self.virtual_nodes {
            let virtual_key = format!("{}:{}", node_id, i);
            let hash = Self::hash_key(&virtual_key);
            if self.nodes.get(&hash).map(|n| n == node_id).unwrap_or(false) {
                hashes_to_remove.push(hash);
            }
        }

        for hash in hashes_to_remove {
            self.nodes.remove(&hash);
        }

        Ok(())
    }

    pub fn get_node(&self, key: &str) -> Option<String> {
        if self.nodes.is_empty() {
            return None;
        }

        let hash = Self::hash_key(key);
        
        // Find the first node with hash >= key hash
        for (node_hash, node_id) in self.nodes.range(hash..).next() {
            return Some(node_id.clone());
        }

        // If not found, wrap around to the first node in the ring
        self.nodes.values().next().cloned()
    }

    pub fn get_node_count(&self) -> usize {
        let mut unique_nodes = std::collections::HashSet::new();
        for node_id in self.nodes.values() {
            unique_nodes.insert(node_id.clone());
        }
        unique_nodes.len()
    }
}

/// Shard configuration
#[derive(Debug, Clone)]
pub struct ShardConfig {
    pub shard_id: u32,
    pub node_id: String,
    pub primary: bool,
    pub replicas: Vec<String>,
}

impl ShardConfig {
    pub fn new(shard_id: u32, node_id: String) -> Self {
        Self {
            shard_id,
            node_id,
            primary: true,
            replicas: Vec::new(),
        }
    }

    pub fn add_replica(&mut self, node_id: String) {
        self.replicas.push(node_id);
    }

    pub fn get_all_nodes(&self) -> Vec<String> {
        let mut nodes = vec![self.node_id.clone()];
        nodes.extend(self.replicas.clone());
        nodes
    }
}

/// Shard manager for coordinating distributed data
#[derive(Debug, Clone)]
pub struct ShardManager {
    pub shards: HashMap<u32, ShardConfig>,
    pub strategy: ShardingStrategy,
    pub consistent_hash: ConsistentHashRing,
}

impl ShardManager {
    pub fn new(strategy: ShardingStrategy, virtual_nodes: u32) -> Self {
        Self {
            shards: HashMap::new(),
            strategy,
            consistent_hash: ConsistentHashRing::new(virtual_nodes),
        }
    }

    pub fn add_shard(&mut self, shard_config: ShardConfig) -> Result<()> {
        if self.shards.contains_key(&shard_config.shard_id) {
            return Err(anyhow!("Shard {} already exists", shard_config.shard_id));
        }
        
        self.consistent_hash.add_node(shard_config.node_id.clone())?;
        self.shards.insert(shard_config.shard_id, shard_config);
        Ok(())
    }

    pub fn remove_shard(&mut self, shard_id: u32) -> Result<()> {
        let shard = self.shards.remove(&shard_id)
            .ok_or_else(|| anyhow!("Shard {} not found", shard_id))?;
        
        self.consistent_hash.remove_node(&shard.node_id)?;
        Ok(())
    }

    pub fn get_shard_for_key(&self, key: &str) -> Result<u32> {
        let node_id = self.consistent_hash.get_node(key)
            .ok_or_else(|| anyhow!("No node available in hash ring"))?;
        
        // Find the shard that matches this node
        self.shards
            .values()
            .find(|s| s.node_id == node_id)
            .map(|s| s.shard_id)
            .ok_or_else(|| anyhow!("No shard found for node {}", node_id))
    }

    pub fn get_shard_config(&self, shard_id: u32) -> Option<&ShardConfig> {
        self.shards.get(&shard_id)
    }

    pub fn get_shard_count(&self) -> usize {
        self.shards.len()
    }

    pub fn get_replicas_for_shard(&self, shard_id: u32) -> Result<Vec<String>> {
        self.get_shard_config(shard_id)
            .map(|config| config.replicas.clone())
            .ok_or_else(|| anyhow!("Shard {} not found", shard_id))
    }
}

/// Rebalancing coordinator for moving data between shards
#[derive(Debug, Clone)]
pub struct RebalanceCoordinator {
    pub rebalance_in_progress: bool,
    pub moved_chunks: u64,
    pub total_chunks: u64,
}

impl RebalanceCoordinator {
    pub fn new() -> Self {
        Self {
            rebalance_in_progress: false,
            moved_chunks: 0,
            total_chunks: 0,
        }
    }

    pub fn start_rebalance(&mut self, total_chunks: u64) -> Result<()> {
        if self.rebalance_in_progress {
            return Err(anyhow!("Rebalance already in progress"));
        }
        self.rebalance_in_progress = true;
        self.total_chunks = total_chunks;
        self.moved_chunks = 0;
        Ok(())
    }

    pub fn mark_chunk_moved(&mut self) {
        self.moved_chunks += 1;
    }

    pub fn complete_rebalance(&mut self) -> Result<()> {
        if !self.rebalance_in_progress {
            return Err(anyhow!("No rebalance in progress"));
        }
        self.rebalance_in_progress = false;
        Ok(())
    }

    pub fn get_progress(&self) -> f64 {
        if self.total_chunks == 0 {
            0.0
        } else {
            (self.moved_chunks as f64) / (self.total_chunks as f64)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shard_range_creation() {
        let range = ShardRange::new(1, Some("a".to_string()), Some("m".to_string()));
        assert_eq!(range.shard_id, 1);
    }

    #[test]
    fn test_shard_range_contains_key() {
        let range = ShardRange::new(1, Some("a".to_string()), Some("m".to_string()));
        assert!(range.contains_key("b"));
        assert!(range.contains_key("l"));
        assert!(!range.contains_key("z"));
        assert!(!range.contains_key("0"));
    }

    #[test]
    fn test_sharding_strategy_string() {
        assert_eq!(ShardingStrategy::Range.as_str(), "Range");
        assert_eq!(ShardingStrategy::Hash.as_str(), "Hash");
        assert_eq!(ShardingStrategy::List.as_str(), "List");
    }

    #[test]
    fn test_consistent_hash_ring_creation() {
        let ring = ConsistentHashRing::new(160);
        assert_eq!(ring.get_node_count(), 0);
    }

    #[test]
    fn test_consistent_hash_ring_add_node() {
        let mut ring = ConsistentHashRing::new(16);
        ring.add_node("node1".to_string()).ok();
        assert_eq!(ring.get_node_count(), 1);
    }

    #[test]
    fn test_consistent_hash_ring_get_node() {
        let mut ring = ConsistentHashRing::new(16);
        ring.add_node("node1".to_string()).ok();
        let node = ring.get_node("key1");
        assert_eq!(node, Some("node1".to_string()));
    }

    #[test]
    fn test_consistent_hash_ring_multiple_nodes() {
        let mut ring = ConsistentHashRing::new(16);
        ring.add_node("node1".to_string()).ok();
        ring.add_node("node2".to_string()).ok();
        ring.add_node("node3".to_string()).ok();
        assert_eq!(ring.get_node_count(), 3);
    }

    #[test]
    fn test_consistent_hash_ring_remove_node() {
        let mut ring = ConsistentHashRing::new(16);
        ring.add_node("node1".to_string()).ok();
        ring.add_node("node2".to_string()).ok();
        ring.remove_node("node1").ok();
        assert_eq!(ring.get_node_count(), 1);
    }

    #[test]
    fn test_shard_config_creation() {
        let config = ShardConfig::new(1, "node1".to_string());
        assert_eq!(config.shard_id, 1);
        assert!(config.primary);
    }

    #[test]
    fn test_shard_config_add_replica() {
        let mut config = ShardConfig::new(1, "node1".to_string());
        config.add_replica("node2".to_string());
        assert_eq!(config.replicas.len(), 1);
    }

    #[test]
    fn test_shard_config_get_all_nodes() {
        let mut config = ShardConfig::new(1, "node1".to_string());
        config.add_replica("node2".to_string());
        config.add_replica("node3".to_string());
        let nodes = config.get_all_nodes();
        assert_eq!(nodes.len(), 3);
    }

    #[test]
    fn test_shard_manager_creation() {
        let manager = ShardManager::new(ShardingStrategy::Hash, 16);
        assert_eq!(manager.strategy, ShardingStrategy::Hash);
        assert_eq!(manager.get_shard_count(), 0);
    }

    #[test]
    fn test_shard_manager_add_shard() {
        let mut manager = ShardManager::new(ShardingStrategy::Hash, 16);
        let config = ShardConfig::new(1, "node1".to_string());
        manager.add_shard(config).ok();
        assert_eq!(manager.get_shard_count(), 1);
    }

    #[test]
    fn test_shard_manager_get_shard_for_key() -> Result<()> {
        let mut manager = ShardManager::new(ShardingStrategy::Hash, 16);
        let config = ShardConfig::new(1, "node1".to_string());
        manager.add_shard(config)?;
        
        let shard_id = manager.get_shard_for_key("test_key")?;
        assert_eq!(shard_id, 1);
        Ok(())
    }

    #[test]
    fn test_rebalance_coordinator_creation() {
        let coordinator = RebalanceCoordinator::new();
        assert!(!coordinator.rebalance_in_progress);
    }

    #[test]
    fn test_rebalance_coordinator_start() {
        let mut coordinator = RebalanceCoordinator::new();
        coordinator.start_rebalance(100).ok();
        assert!(coordinator.rebalance_in_progress);
        assert_eq!(coordinator.total_chunks, 100);
    }

    #[test]
    fn test_rebalance_coordinator_progress() {
        let mut coordinator = RebalanceCoordinator::new();
        coordinator.start_rebalance(100).ok();
        coordinator.mark_chunk_moved();
        coordinator.mark_chunk_moved();
        assert_eq!(coordinator.moved_chunks, 2);
        assert!((coordinator.get_progress() - 0.02).abs() < 0.001);
    }

    #[test]
    fn test_rebalance_coordinator_complete() {
        let mut coordinator = RebalanceCoordinator::new();
        coordinator.start_rebalance(100).ok();
        coordinator.complete_rebalance().ok();
        assert!(!coordinator.rebalance_in_progress);
    }

    #[test]
    fn test_shard_manager_multiple_shards() {
        let mut manager = ShardManager::new(ShardingStrategy::Range, 16);
        
        for i in 1..=4 {
            let config = ShardConfig::new(i, format!("node{}", i));
            manager.add_shard(config).ok();
        }
        
        assert_eq!(manager.get_shard_count(), 4);
    }

    #[test]
    fn test_shard_manager_replicas() -> Result<()> {
        let mut manager = ShardManager::new(ShardingStrategy::Hash, 16);
        let mut config = ShardConfig::new(1, "node1".to_string());
        config.add_replica("node2".to_string());
        config.add_replica("node3".to_string());
        
        manager.add_shard(config)?;
        let replicas = manager.get_replicas_for_shard(1)?;
        assert_eq!(replicas.len(), 2);
        Ok(())
    }

    #[test]
    fn test_hash_consistency() {
        let mut ring = ConsistentHashRing::new(16);
        ring.add_node("node1".to_string()).ok();
        
        let node_for_key = ring.get_node("test_key");
        assert!(node_for_key.is_some());
    }
}
