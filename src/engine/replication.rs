//! Raft консенсус для распределённой координации

use std::collections::HashMap;
use chrono::{Utc, DateTime};
use anyhow::{Result, anyhow};

/// Raft server state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaftState {
    Follower,
    Candidate,
    Leader,
}

impl RaftState {
    pub fn as_str(&self) -> &'static str {
        match self {
            RaftState::Follower => "Follower",
            RaftState::Candidate => "Candidate",
            RaftState::Leader => "Leader",
        }
    }
}

/// Log entry for replication
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub term: u64,
    pub index: u64,
    pub command: Vec<u8>,
    pub committed: bool,
}

impl LogEntry {
    pub fn new(term: u64, index: u64, command: Vec<u8>) -> Self {
        Self {
            term,
            index,
            command,
            committed: false,
        }
    }

    pub fn commit(&mut self) {
        self.committed = true;
    }
}

/// Replica tracking information
#[derive(Debug, Clone)]
pub struct ReplicaInfo {
    pub node_id: String,
    pub next_index: u64,
    pub match_index: u64,
    pub last_heartbeat: DateTime<chrono::Utc>,
    pub healthy: bool,
}

impl ReplicaInfo {
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            next_index: 1,
            match_index: 0,
            last_heartbeat: Utc::now(),
            healthy: true,
        }
    }

    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = Utc::now();
        self.healthy = true;
    }

    pub fn mark_unhealthy(&mut self) {
        self.healthy = false;
    }

    pub fn is_lagging(&self, leader_index: u64) -> bool {
        self.match_index < leader_index
    }
}

/// Raft node state machine
#[derive(Debug, Clone)]
pub struct RaftNode {
    pub node_id: String,
    pub current_term: u64,
    pub voted_for: Option<String>,
    pub state: RaftState,
    pub log: Vec<LogEntry>,
    pub commit_index: u64,
    pub last_applied: u64,
    pub leader_id: Option<String>,
}

impl RaftNode {
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            current_term: 0,
            voted_for: None,
            state: RaftState::Follower,
            log: Vec::new(),
            commit_index: 0,
            last_applied: 0,
            leader_id: None,
        }
    }

    pub fn append_entry(&mut self, entry: LogEntry) -> Result<()> {
        if entry.term < self.current_term {
            return Err(anyhow!("Entry term {} is older than current term {}", 
                entry.term, self.current_term));
        }
        self.log.push(entry);
        Ok(())
    }

    pub fn get_last_log_term(&self) -> u64 {
        self.log.last().map(|e| e.term).unwrap_or(0)
    }

    pub fn get_last_log_index(&self) -> u64 {
        self.log.len() as u64
    }

    pub fn advance_term(&mut self, new_term: u64) {
        if new_term > self.current_term {
            self.current_term = new_term;
            self.voted_for = None;
            if self.state != RaftState::Follower {
                self.state = RaftState::Follower;
            }
        }
    }

    pub fn become_leader(&mut self) {
        self.state = RaftState::Leader;
        self.leader_id = Some(self.node_id.clone());
    }

    pub fn become_candidate(&mut self) {
        self.state = RaftState::Candidate;
        self.current_term += 1;
        self.voted_for = Some(self.node_id.clone());
    }

    pub fn become_follower(&mut self) {
        self.state = RaftState::Follower;
        self.voted_for = None;
    }
}

/// Replication manager coordinates data replication across cluster
#[derive(Debug, Clone)]
pub struct ReplicationManager {
    pub replicas: HashMap<String, ReplicaInfo>,
    pub replication_factor: u64,
    pub min_replicas_for_quorum: u64,
}

impl ReplicationManager {
    pub fn new(replication_factor: u64) -> Self {
        Self {
            replicas: HashMap::new(),
            replication_factor,
            min_replicas_for_quorum: (replication_factor / 2) + 1,
        }
    }

    pub fn add_replica(&mut self, node_id: String) -> Result<()> {
        if self.replicas.len() as u64 >= self.replication_factor {
            return Err(anyhow!("Replication factor {} already reached", self.replication_factor));
        }
        self.replicas.insert(node_id.clone(), ReplicaInfo::new(node_id));
        Ok(())
    }

    pub fn remove_replica(&mut self, node_id: &str) -> Result<()> {
        self.replicas.remove(node_id)
            .ok_or_else(|| anyhow!("Replica {} not found", node_id))?;
        Ok(())
    }

    pub fn update_replica_heartbeat(&mut self, node_id: &str) {
        if let Some(replica) = self.replicas.get_mut(node_id) {
            replica.update_heartbeat();
        }
    }

    pub fn mark_replica_unhealthy(&mut self, node_id: &str) {
        if let Some(replica) = self.replicas.get_mut(node_id) {
            replica.mark_unhealthy();
        }
    }

    pub fn get_healthy_replicas(&self) -> usize {
        self.replicas.values().filter(|r| r.healthy).count()
    }

    pub fn is_quorum_available(&self) -> bool {
        self.get_healthy_replicas() as u64 >= self.min_replicas_for_quorum
    }

    pub fn get_replication_status(&self, leader_index: u64) -> String {
        let healthy = self.get_healthy_replicas();
        let lagging_replicas: Vec<_> = self.replicas
            .values()
            .filter(|r| r.is_lagging(leader_index))
            .map(|r| r.node_id.clone())
            .collect();
        
        format!(
            "Healthy: {}/{}, Lagging: {:?}",
            healthy,
            self.replicas.len(),
            lagging_replicas
        )
    }
}

/// Failover coordinator
#[derive(Debug, Clone)]
pub struct FailoverCoordinator {
    pub current_leader: Option<String>,
    pub failover_in_progress: bool,
    pub last_failover: Option<DateTime<chrono::Utc>>,
    pub failover_timeout_secs: u64,
}

impl FailoverCoordinator {
    pub fn new() -> Self {
        Self {
            current_leader: None,
            failover_in_progress: false,
            last_failover: None,
            failover_timeout_secs: 30,
        }
    }

    pub fn set_leader(&mut self, node_id: String) {
        self.current_leader = Some(node_id);
        self.failover_in_progress = false;
    }

    pub fn initiate_failover(&mut self) -> Result<()> {
        if self.failover_in_progress {
            return Err(anyhow!("Failover already in progress"));
        }
        
        // Check if enough time has passed since last failover
        if let Some(last) = self.last_failover {
            let elapsed = Utc::now().signed_duration_since(last).num_seconds() as u64;
            if elapsed < self.failover_timeout_secs {
                return Err(anyhow!(
                    "Failover cooldown active. {} seconds remaining",
                    self.failover_timeout_secs - elapsed
                ));
            }
        }

        self.failover_in_progress = true;
        self.last_failover = Some(Utc::now());
        self.current_leader = None;
        Ok(())
    }

    pub fn complete_failover(&mut self, new_leader: String) {
        self.current_leader = Some(new_leader);
        self.failover_in_progress = false;
    }

    pub fn is_leader_healthy(&self) -> bool {
        self.current_leader.is_some() && !self.failover_in_progress
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raft_state_string() {
        assert_eq!(RaftState::Follower.as_str(), "Follower");
        assert_eq!(RaftState::Candidate.as_str(), "Candidate");
        assert_eq!(RaftState::Leader.as_str(), "Leader");
    }

    #[test]
    fn test_log_entry_creation() {
        let entry = LogEntry::new(1, 1, vec![1, 2, 3]);
        assert_eq!(entry.term, 1);
        assert_eq!(entry.index, 1);
        assert!(!entry.committed);
    }

    #[test]
    fn test_log_entry_commit() {
        let mut entry = LogEntry::new(1, 1, vec![1, 2, 3]);
        entry.commit();
        assert!(entry.committed);
    }

    #[test]
    fn test_replica_info_creation() {
        let replica = ReplicaInfo::new("node1".to_string());
        assert_eq!(replica.node_id, "node1");
        assert_eq!(replica.match_index, 0);
        assert!(replica.healthy);
    }

    #[test]
    fn test_replica_info_lagging_check() {
        let replica = ReplicaInfo::new("node1".to_string());
        assert!( replica.is_lagging(10));
        assert!(!replica.is_lagging(0));
    }

    #[test]
    fn test_replica_info_mark_unhealthy() {
        let mut replica = ReplicaInfo::new("node1".to_string());
        replica.mark_unhealthy();
        assert!(!replica.healthy);
    }

    #[test]
    fn test_raft_node_creation() {
        let node = RaftNode::new("leader".to_string());
        assert_eq!(node.current_term, 0);
        assert_eq!(node.state, RaftState::Follower);
        assert!(node.voted_for.is_none());
    }

    #[test]
    fn test_raft_node_append_entry() -> Result<()> {
        let mut node = RaftNode::new("leader".to_string());
        let entry = LogEntry::new(1, 1, vec![1, 2, 3]);
        node.append_entry(entry)?;
        assert_eq!(node.log.len(), 1);
        Ok(())
    }

    #[test]
    fn test_raft_node_get_last_log_index() {
        let mut node = RaftNode::new("leader".to_string());
        node.log.push(LogEntry::new(1, 1, vec![]));
        node.log.push(LogEntry::new(1, 2, vec![]));
        assert_eq!(node.get_last_log_index(), 2);
    }

    #[test]
    fn test_raft_node_get_last_log_term() {
        let mut node = RaftNode::new("leader".to_string());
        node.log.push(LogEntry::new(1, 1, vec![]));
        node.log.push(LogEntry::new(2, 2, vec![]));
        assert_eq!(node.get_last_log_term(), 2);
    }

    #[test]
    fn test_raft_node_advance_term() {
        let mut node = RaftNode::new("follower".to_string());
        node.advance_term(5);
        assert_eq!(node.current_term, 5);
        assert_eq!(node.state, RaftState::Follower);
    }

    #[test]
    fn test_raft_node_become_leader() {
        let mut node = RaftNode::new("node1".to_string());
        node.become_leader();
        assert_eq!(node.state, RaftState::Leader);
    }

    #[test]
    fn test_raft_node_become_candidate() {
        let mut node = RaftNode::new("node1".to_string());
        let initial_term = node.current_term;
        node.become_candidate();
        assert_eq!(node.state, RaftState::Candidate);
        assert_eq!(node.current_term, initial_term + 1);
        assert_eq!(node.voted_for, Some("node1".to_string()));
    }

    #[test]
    fn test_replication_manager_creation() {
        let manager = ReplicationManager::new(3);
        assert_eq!(manager.replication_factor, 3);
        assert_eq!(manager.min_replicas_for_quorum, 2);
    }

    #[test]
    fn test_replication_manager_add_replica() -> Result<()> {
        let mut manager = ReplicationManager::new(3);
        manager.add_replica("node1".to_string())?;
        manager.add_replica("node2".to_string())?;
        assert_eq!(manager.replicas.len(), 2);
        Ok(())
    }

    #[test]
    fn test_replication_manager_replication_factor_limit() {
        let mut manager = ReplicationManager::new(1);
        manager.add_replica("node1".to_string()).ok();
        let result = manager.add_replica("node2".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_replication_manager_remove_replica() -> Result<()> {
        let mut manager = ReplicationManager::new(3);
        manager.add_replica("node1".to_string())?;
        manager.remove_replica("node1")?;
        assert_eq!(manager.replicas.len(), 0);
        Ok(())
    }

    #[test]
    fn test_replication_manager_quorum_check() {
        let mut manager = ReplicationManager::new(3);
        manager.add_replica("node1".to_string()).ok();
        manager.add_replica("node2".to_string()).ok();
        assert!(manager.is_quorum_available());
        
        manager.mark_replica_unhealthy("node1");
        assert!(!manager.is_quorum_available());
    }

    #[test]
    fn test_replication_manager_healthy_replicas() {
        let mut manager = ReplicationManager::new(3);
        manager.add_replica("node1".to_string()).ok();
        manager.add_replica("node2".to_string()).ok();
        manager.add_replica("node3".to_string()).ok();
        
        assert_eq!(manager.get_healthy_replicas(), 3);
        manager.mark_replica_unhealthy("node1");
        assert_eq!(manager.get_healthy_replicas(), 2);
    }

    #[test]
    fn test_failover_coordinator_creation() {
        let coordinator = FailoverCoordinator::new();
        assert!(coordinator.current_leader.is_none());
        assert!(!coordinator.failover_in_progress);
    }

    #[test]
    fn test_failover_coordinator_set_leader() {
        let mut coordinator = FailoverCoordinator::new();
        coordinator.set_leader("node1".to_string());
        assert_eq!(coordinator.current_leader, Some("node1".to_string()));
    }

    #[test]
    fn test_failover_coordinator_initiate_failover() -> Result<()> {
        let mut coordinator = FailoverCoordinator::new();
        coordinator.set_leader("node1".to_string());
        coordinator.initiate_failover()?;
        assert!(coordinator.failover_in_progress);
        assert!(coordinator.current_leader.is_none());
        Ok(())
    }

    #[test]
    fn test_failover_coordinator_complete_failover() {
        let mut coordinator = FailoverCoordinator::new();
        coordinator.set_leader("node1".to_string());
        coordinator.initiate_failover().ok();
        coordinator.complete_failover("node2".to_string());
        assert_eq!(coordinator.current_leader, Some("node2".to_string()));
        assert!(!coordinator.failover_in_progress);
    }

    #[test]
    fn test_failover_cooldown() {
        let mut coordinator = FailoverCoordinator::new();
        coordinator.set_leader("node1".to_string());
        coordinator.initiate_failover().ok();
        
        // Immediately try another failover - should fail
        let result = coordinator.initiate_failover();
        assert!(result.is_err());
    }

    #[test]
    fn test_failover_coordinator_leader_health() {
        let mut coordinator = FailoverCoordinator::new();
        assert!(!coordinator.is_leader_healthy());
        
        coordinator.set_leader("node1".to_string());
        assert!(coordinator.is_leader_healthy());
        
        coordinator.initiate_failover().ok();
        assert!(!coordinator.is_leader_healthy());
    }
}
