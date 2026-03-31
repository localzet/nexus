//! MVCC (Multi-Version Concurrency Control) — система многоверсионности
//! для конкурентной обработки транзакций без глобальных блокировок.

use crate::types::{Value, Row};
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use chrono::{Utc, DateTime};

/// Версия строки с метаданными для MVCC
#[derive(Debug, Clone)]
pub struct Version {
    pub version_id: u64,
    pub data: Row,
    pub created_tx_id: u64,
    pub created_ts: DateTime<chrono::Utc>,
    pub committed: bool,
    pub deleted: bool,
}

impl Version {
    pub fn new(version_id: u64, tx_id: u64, data: Row) -> Self {
        Self {
            version_id,
            data,
            created_tx_id: tx_id,
            created_ts: Utc::now(),
            committed: false,
            deleted: false,
        }
    }

    pub fn commit(&mut self) {
        self.committed = true;
    }

    pub fn mark_deleted(&mut self) {
        self.deleted = true;
    }
}

/// Цепочка версий для одной строки таблицы
#[derive(Debug, Clone)]
pub struct VersionChain {
    pub row_id: u64,
    pub versions: Vec<Version>,
}

impl VersionChain {
    pub fn new(row_id: u64) -> Self {
        Self {
            row_id,
            versions: Vec::new(),
        }
    }

    pub fn add_version(&mut self, version: Version) {
        self.versions.push(version);
    }

    pub fn get_visible_version(&self, tx_id: u64) -> Option<&Version> {
        // Get the latest committed version visible to this transaction
        self.versions
            .iter()
            .rev()
            .find(|v| v.committed && v.created_tx_id <= tx_id && !v.deleted)
    }

    pub fn get_uncommitted_version(&self, tx_id: u64) -> Option<&Version> {
        // Get uncommitted version from same transaction
        self.versions
            .iter()
            .rev()
            .find(|v| v.created_tx_id == tx_id && !v.deleted)
    }
}

/// Lock types for concurrency control
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockType {
    Shared,    // Multiple readers
    Exclusive, // Exclusive writer
}

/// Lock entry
#[derive(Debug, Clone)]
pub struct Lock {
    pub lock_type: LockType,
    pub tx_id: u64,
    pub acquired_at: DateTime<chrono::Utc>,
}

/// Lock manager for row/page level locking
#[derive(Debug, Clone)]
pub struct LockManager {
    locks: HashMap<u64, Vec<Lock>>, // row_id -> locks
}

impl LockManager {
    pub fn new() -> Self {
        Self {
            locks: HashMap::new(),
        }
    }

    pub fn acquire_lock(&mut self, row_id: u64, lock_type: LockType, tx_id: u64) -> Result<()> {
        let locks = self.locks.entry(row_id).or_insert_with(Vec::new);

        // Check for conflicts
        for existing_lock in locks.iter() {
            if existing_lock.tx_id != tx_id {
                match (lock_type, existing_lock.lock_type) {
                    (LockType::Exclusive, _) | (_, LockType::Exclusive) => {
                        return Err(anyhow!("Lock conflict: exclusive lock held"));
                    }
                    _ => {}
                }
            }
        }

        // Add the lock
        locks.push(Lock {
            lock_type,
            tx_id,
            acquired_at: Utc::now(),
        });

        Ok(())
    }

    pub fn release_lock(&mut self, row_id: u64, tx_id: u64) {
        if let Some(locks) = self.locks.get_mut(&row_id) {
            locks.retain(|lock| lock.tx_id != tx_id);
        }
    }

    pub fn release_all_locks(&mut self, tx_id: u64) {
        for locks in self.locks.values_mut() {
            locks.retain(|lock| lock.tx_id != tx_id);
        }
    }

    pub fn has_lock(&self, row_id: u64, lock_type: LockType, tx_id: u64) -> bool {
        if let Some(locks) = self.locks.get(&row_id) {
            locks.iter().any(|lock| lock.tx_id == tx_id && lock.lock_type == lock_type)
        } else {
            false
        }
    }
}

/// Isolation levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

/// MVCC-based transaction
#[derive(Debug, Clone)]
pub struct MVCCTransaction {
    pub tx_id: u64,
    pub isolation_level: IsolationLevel,
    pub start_tx_id: u64, // For determining visibility
    pub read_set: Vec<u64>,
    pub write_set: Vec<u64>,
    pub active: bool,
}

impl MVCCTransaction {
    pub fn new(tx_id: u64, isolation_level: IsolationLevel) -> Self {
        Self {
            tx_id,
            isolation_level,
            start_tx_id: tx_id,
            read_set: Vec::new(),
            write_set: Vec::new(),
            active: true,
        }
    }

    pub fn add_read(&mut self, row_id: u64) {
        self.read_set.push(row_id);
    }

    pub fn add_write(&mut self, row_id: u64) {
        self.write_set.push(row_id);
    }

    pub fn is_conflict_possible(&self, other_tx: &MVCCTransaction) -> bool {
        // Check if read-write conflicts exist
        let has_read_write_conflict = self.read_set.iter().any(|id| other_tx.write_set.contains(id))
            || self.write_set.iter().any(|id| other_tx.read_set.contains(id));

        // Check if write-write conflicts exist
        let has_write_write_conflict = self
            .write_set
            .iter()
            .any(|id| other_tx.write_set.contains(id));

        has_read_write_conflict || has_write_write_conflict
    }

    pub fn commit(&mut self) -> Result<()> {
        if !self.active {
            return Err(anyhow!("Transaction already committed or aborted"));
        }
        self.active = false;
        Ok(())
    }

    pub fn abort(&mut self) -> Result<()> {
        if !self.active {
            return Err(anyhow!("Transaction already committed or aborted"));
        }
        self.active = false;
        Ok(())
    }
}

/// Serialization graph for conflict detection
#[derive(Debug, Clone)]
pub struct ConflictGraph {
    edges: HashMap<u64, Vec<u64>>, // tx_id -> conflicting tx_ids
}

impl ConflictGraph {
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
        }
    }

    pub fn add_edge(&mut self, from_tx: u64, to_tx: u64) {
        self.edges.entry(from_tx).or_insert_with(Vec::new).push(to_tx);
    }

    pub fn has_cycle(&self) -> bool {
        let mut visited = std::collections::HashSet::new();
        let mut rec_stack = std::collections::HashSet::new();

        for node in self.edges.keys() {
            if !visited.contains(node) {
                if self.has_cycle_dfs(*node, &mut visited, &mut rec_stack) {
                    return true;
                }
            }
        }
        false
    }

    fn has_cycle_dfs(
        &self,
        node: u64,
        visited: &mut std::collections::HashSet<u64>,
        rec_stack: &mut std::collections::HashSet<u64>,
    ) -> bool {
        visited.insert(node);
        rec_stack.insert(node);

        if let Some(neighbors) = self.edges.get(&node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if self.has_cycle_dfs(*neighbor, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(neighbor) {
                    return true;
                }
            }
        }

        rec_stack.remove(&node);
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_creation() {
        let row = Row::new(crate::types::RecordId::new());
        let version = Version::new(1, 100, row);
        
        assert_eq!(version.version_id, 1);
        assert_eq!(version.created_tx_id, 100);
        assert!(!version.committed);
        assert!(!version.deleted);
    }

    #[test]
    fn test_version_commit() {
        let row = Row::new(crate::types::RecordId::new());
        let mut version = Version::new(1, 100, row);
        
        version.commit();
        assert!(version.committed);
    }

    #[test]
    fn test_version_mark_deleted() {
        let row = Row::new(crate::types::RecordId::new());
        let mut version = Version::new(1, 100, row);
        
        version.mark_deleted();
        assert!(version.deleted);
    }

    #[test]
    fn test_version_chain_creation() {
        let chain = VersionChain::new(42);
        assert_eq!(chain.row_id, 42);
        assert_eq!(chain.versions.len(), 0);
    }

    #[test]
    fn test_version_chain_add_version() {
        let mut chain = VersionChain::new(42);
        let row = Row::new(crate::types::RecordId::new());
        let version = Version::new(1, 100, row);
        
        chain.add_version(version);
        assert_eq!(chain.versions.len(), 1);
    }

    #[test]
    fn test_get_visible_version() {
        let mut chain = VersionChain::new(42);
        let row = Row::new(crate::types::RecordId::new());
        let mut version = Version::new(1, 100, row);
        version.commit();
        chain.add_version(version);

        // Transaction 105 should see version created by tx 100
        assert!(chain.get_visible_version(105).is_some());
        
        // Transaction 50 should not see version created by tx 100
        assert!(chain.get_visible_version(50).is_none());
    }

    #[test]
    fn test_get_uncommitted_version() {
        let mut chain = VersionChain::new(42);
        let row = Row::new(crate::types::RecordId::new());
        let version = Version::new(1, 100, row);
        chain.add_version(version);

        // tx 100 should see its own uncommitted version
        assert!(chain.get_uncommitted_version(100).is_some());
        
        // tx 200 should not see version created by tx 100
        assert!(chain.get_uncommitted_version(200).is_none());
    }

    #[test]
    fn test_lock_manager_acquire_shared() -> Result<()> {
        let mut manager = LockManager::new();
        manager.acquire_lock(1, LockType::Shared, 100)?;
        assert!(manager.has_lock(1, LockType::Shared, 100));
        Ok(())
    }

    #[test]
    fn test_lock_manager_acquire_exclusive() -> Result<()> {
        let mut manager = LockManager::new();
        manager.acquire_lock(1, LockType::Exclusive, 100)?;
        assert!(manager.has_lock(1, LockType::Exclusive, 100));
        Ok(())
    }

    #[test]
    fn test_lock_manager_conflict_detection() -> Result<()> {
        let mut manager = LockManager::new();
        manager.acquire_lock(1, LockType::Exclusive, 100)?;
        
        // Another transaction trying to get any lock should fail
        let result = manager.acquire_lock(1, LockType::Shared, 200);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_lock_manager_release() {
        let mut manager = LockManager::new();
        manager.acquire_lock(1, LockType::Shared, 100).ok();
        assert!(manager.has_lock(1, LockType::Shared, 100));
        
        manager.release_lock(1, 100);
        assert!(!manager.has_lock(1, LockType::Shared, 100));
    }

    #[test]
    fn test_lock_manager_release_all() {
        let mut manager = LockManager::new();
        manager.acquire_lock(1, LockType::Shared, 100).ok();
        manager.acquire_lock(2, LockType::Exclusive, 100).ok();
        manager.acquire_lock(3, LockType::Shared, 100).ok();

        manager.release_all_locks(100);
        assert!(!manager.has_lock(1, LockType::Shared, 100));
        assert!(!manager.has_lock(2, LockType::Exclusive, 100));
        assert!(!manager.has_lock(3, LockType::Shared, 100));
    }

    #[test]
    fn test_mvcc_transaction_creation() {
        let tx = MVCCTransaction::new(1, IsolationLevel::RepeatableRead);
        assert_eq!(tx.tx_id, 1);
        assert_eq!(tx.isolation_level, IsolationLevel::RepeatableRead);
        assert!(tx.active);
    }

    #[test]
    fn test_mvcc_transaction_reads_writes() {
        let mut tx = MVCCTransaction::new(1, IsolationLevel::Serializable);
        tx.add_read(10);
        tx.add_write(20);
        
        assert_eq!(tx.read_set.len(), 1);
        assert_eq!(tx.write_set.len(), 1);
    }

    #[test]
    fn test_mvcc_conflict_detection() {
        let mut tx1 = MVCCTransaction::new(1, IsolationLevel::Serializable);
        let mut tx2 = MVCCTransaction::new(2, IsolationLevel::Serializable);
        
        tx1.add_read(10);
        tx2.add_write(10);
        
        assert!(tx1.is_conflict_possible(&tx2));
    }

    #[test]
    fn test_mvcc_transaction_commit() -> Result<()> {
        let mut tx = MVCCTransaction::new(1, IsolationLevel::Serializable);
        assert!(tx.active);
        
        tx.commit()?;
        assert!(!tx.active);
        Ok(())
    }

    #[test]
    fn test_mvcc_transaction_abort() -> Result<()> {
        let mut tx = MVCCTransaction::new(1, IsolationLevel::Serializable);
        assert!(tx.active);
        
        tx.abort()?;
        assert!(!tx.active);
        Ok(())
    }

    #[test]
    fn test_conflict_graph_acyclic() {
        let mut graph = ConflictGraph::new();
        graph.add_edge(1, 2);
        graph.add_edge(2, 3);
        graph.add_edge(3, 4);
        
        assert!(!graph.has_cycle());
    }

    #[test]
    fn test_conflict_graph_with_cycle() {
        let mut graph = ConflictGraph::new();
        graph.add_edge(1, 2);
        graph.add_edge(2, 3);
        graph.add_edge(3, 1); // Cycle!
        
        assert!(graph.has_cycle());
    }

    #[test]
    fn test_isolation_level_enum() {
        let levels = vec![
            IsolationLevel::ReadUncommitted,
            IsolationLevel::ReadCommitted,
            IsolationLevel::RepeatableRead,
            IsolationLevel::Serializable,
        ];
        assert_eq!(levels.len(), 4);
    }

    #[test]
    fn test_multiple_transactions_no_conflict() {
        let mut tx1 = MVCCTransaction::new(1, IsolationLevel::Serializable);
        let mut tx2 = MVCCTransaction::new(2, IsolationLevel::Serializable);
        
        tx1.add_read(10);
        tx2.add_read(20);
        
        assert!(!tx1.is_conflict_possible(&tx2));
    }

    #[test]
    fn test_write_write_conflict() {
        let mut tx1 = MVCCTransaction::new(1, IsolationLevel::Serializable);
        let mut tx2 = MVCCTransaction::new(2, IsolationLevel::Serializable);
        
        tx1.add_write(10);
        tx2.add_write(10);
        
        assert!(tx1.is_conflict_possible(&tx2));
    }
}
