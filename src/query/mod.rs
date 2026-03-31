/// Query processing module - SQL parsing and execution
pub mod executor;
pub mod aggregates;
pub mod joins;
pub mod dml;
pub mod planning;
pub mod window_functions;
pub mod cte;
pub mod optimization;
pub mod fulltext;
// Advanced modules disabled - compatibility issues
// Will be re-enabled with proper sqlparser integration
// pub mod parser;
// pub mod functions;
// pub mod optimizer;

pub use executor::QueryExecutor;
pub use aggregates::{AggregateFunction, AggregateAccumulator, GroupByProcessor};
pub use joins::{JoinType, JoinCondition, JoinExecutor};
pub use dml::{InsertStatement, UpdateStatement, DeleteStatement};
pub use planning::{QueryPlanner, PlanNode, PlanType, QueryCost, ExplainFormatter};
pub use window_functions::{WindowFunctionProcessor, WindowFunctionType, WindowFunctionContext, WindowSpec};
pub use cte::{CommonTableExpression, Subquery, CteMaterializer, SubqueryExecutor, SubqueryType};
pub use optimization::{ColumnStatistics, Histogram, Predicate, StatisticsCollector, QueryOptimizer, PredicateType};
pub use fulltext::{Token, Document, InvertedIndex, Tokenizer, BM25Scorer, Posting};
