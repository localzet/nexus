//! NexusDB — многомодельная СУБД на Rust с поддержкой MVCC, Raft и распределённых вычислений.
//!
//! Основные компоненты:
//! - Движок хранения данных (таблицы, JSON, графы, векторы)
//! - SQL парсер и executor
//! - MVCC транзакции
//! - Raft консенсус и репликация
//! - Индексирование и оптимизация запросов
//! - Полнотекстовый поиск
//! - Мониторинг и телеметрия

pub mod types;
pub mod engine;
pub mod query;
pub mod protocol;
pub mod storage;
pub mod sql;
pub mod monitoring;

pub use types::*;
pub use engine::*;
pub use storage::*;
pub use sql::*;
pub use monitoring::*;

pub use engine::MultiModelEngine;
pub use query::QueryExecutor;
pub use storage::{TransactionManager, Transaction, IsolationLevel};
pub use storage::WalManager;
pub use sql::SqlParser;
