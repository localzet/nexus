/// Engine module - Core storage implementation
pub mod multimodel;
pub mod indexing;
pub mod constraints;
pub mod auth;
pub mod query_planner;
pub mod distributed;
pub mod analytics;
pub mod replication;
pub mod sharding;

pub use multimodel::{
    TableStore, DocumentStore, GraphStore, VectorStore,
    MultiModelEngine, VectorRecord, cosine_distance
};
pub use indexing::{IndexingManager, BTreeIndex, HashIndex, BloomFilter, QueryCache};
pub use constraints::{Constraint, ConstraintType, ConstraintMetadata, ForeignKeyRef};
pub use auth::{AuthManager, User, Role, Permission, RoleDefinition};
pub use query_planner::{QueryOptimizer, QueryPlan, QueryPlanNode, OperationType, QueryStatistics};
pub use distributed::{DistributedQueryExecutor, CoordinationStrategy, ShardId, ShardLocation, QueryFragment};
pub use analytics::{MetricsCollector, MetricType, MetricValue, QueryStats, HealthStatus, HealthCheck};
pub use replication::{RaftNode, RaftState, ReplicationManager, FailoverCoordinator, LogEntry};
pub use sharding::{ShardManager, ShardConfig, ShardRange, ShardingStrategy, ConsistentHashRing, RebalanceCoordinator};
