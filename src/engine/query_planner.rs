//! Оптимизация запросов и планы выполнения

use std::collections::HashMap;
use anyhow::Result;

/// Query operation type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationType {
    SeqScan,         // Sequential scan
    IndexScan,       // Index-based scan
    IndexSeek,       // Index seek (fastest)
    BitmapScan,      // Bitmap index scan
    HashJoin,        // Hash join
    NestedLoopJoin,  // Nested loop join
    MergeJoin,       // Merge join (sorted input)
    GroupAggregate,  // GROUP BY with aggregation
    Sort,            // Sort operation
    Limit,           // LIMIT operation
    Filter,          // WHERE clause filter
    Projection,      // SELECT columns
    Union,           // UNION operation
    Append,          // APPEND operation
}

/// Query plan node
#[derive(Debug, Clone)]
pub struct QueryPlanNode {
    pub node_id: usize,
    pub operation: OperationType,
    pub table: Option<String>,
    pub index: Option<String>,
    pub relation_name: String,
    pub startup_cost: f64,
    pub total_cost: f64,
    pub rows: usize,
    pub width: usize,
    pub filter: Option<String>,
    pub children: Vec<QueryPlanNode>,
    pub estimated_memory: usize,
}

/// Query execution plan
#[derive(Debug, Clone)]
pub struct QueryPlan {
    pub root: QueryPlanNode,
    pub total_cost: f64,
    pub planner_time: f64,
    pub execution_time: f64,
    pub planning_strategy: String,
}

/// Query optimizer
pub struct QueryOptimizer {
    cost_model: CostModel,
    statistics: QueryStatistics,
}

/// Cost model for query optimization
#[derive(Clone)]
struct CostModel {
    seq_scan_cost: f64,        // per row
    index_scan_cost: f64,       // per row
    index_seek_cost: f64,       // base cost
    hash_join_cost: f64,        // per row
    nested_loop_cost: f64,      // per row
    sort_cost_factor: f64,      // for n*log(n) sorting
}

impl Default for CostModel {
    fn default() -> Self {
        Self {
            seq_scan_cost: 1.0,
            index_scan_cost: 0.5,
            index_seek_cost: 1.0,
            hash_join_cost: 0.1,
            nested_loop_cost: 0.5,
            sort_cost_factor: 1.5,
        }
    }
}

/// Query statistics for optimization
#[derive(Debug, Clone, Default)]
pub struct QueryStatistics {
    pub table_sizes: HashMap<String, usize>,
    pub column_selectivity: HashMap<String, f64>,
    pub index_info: HashMap<String, IndexInfo>,
}

/// Index information
#[derive(Debug, Clone)]
pub struct IndexInfo {
    pub name: String,
    pub column: String,
    pub selectivity: f64,
    pub est_cost: f64,
}

impl QueryOptimizer {
    pub fn new() -> Self {
        Self {
            cost_model: CostModel::default(),
            statistics: QueryStatistics::default(),
        }
    }

    /// Register table statistics
    pub fn register_table_stats(&mut self, table: String, size: usize) {
        self.statistics.table_sizes.insert(table, size);
    }

    /// Register column selectivity
    pub fn register_column_selectivity(&mut self, column: String, selectivity: f64) {
        self.statistics.column_selectivity.insert(column, selectivity);
    }

    /// Register index information
    pub fn register_index(&mut self, index_info: IndexInfo) {
        self.statistics
            .index_info
            .insert(index_info.name.clone(), index_info);
    }

    /// Build query execution plan
    pub fn build_plan(
        &self,
        table_name: &str,
        filter: Option<&str>,
        has_index: bool,
    ) -> Result<QueryPlan> {
        let table_size = self.statistics.table_sizes.get(table_name).copied().unwrap_or(1000);

        let root = if has_index && filter.is_some() {
            // Index seek with filter
            let selectivity = self
                .statistics
                .column_selectivity
                .get(filter.unwrap_or(""))
                .copied()
                .unwrap_or(0.1);

            QueryPlanNode {
                node_id: 0,
                operation: OperationType::IndexSeek,
                table: Some(table_name.to_string()),
                index: Some(format!("{}_idx", table_name)),
                relation_name: table_name.to_string(),
                startup_cost: self.cost_model.index_seek_cost,
                total_cost: self.cost_model.index_seek_cost
                    + (table_size as f64 * selectivity * self.cost_model.index_scan_cost),
                rows: (table_size as f64 * selectivity) as usize,
                width: 100,
                filter: filter.map(|s| s.to_string()),
                children: vec![],
                estimated_memory: 512,
            }
        } else if has_index {
            // Full index scan
            QueryPlanNode {
                node_id: 0,
                operation: OperationType::IndexScan,
                table: Some(table_name.to_string()),
                index: Some(format!("{}_idx", table_name)),
                relation_name: table_name.to_string(),
                startup_cost: self.cost_model.index_seek_cost,
                total_cost: self.cost_model.index_seek_cost
                    + (table_size as f64 * self.cost_model.index_scan_cost),
                rows: table_size,
                width: 100,
                filter: None,
                children: vec![],
                estimated_memory: 1024,
            }
        } else {
            // Sequential scan
            QueryPlanNode {
                node_id: 0,
                operation: OperationType::SeqScan,
                table: Some(table_name.to_string()),
                index: None,
                relation_name: table_name.to_string(),
                startup_cost: 0.0,
                total_cost: table_size as f64 * self.cost_model.seq_scan_cost,
                rows: table_size,
                width: 100,
                filter: filter.map(|s| s.to_string()),
                children: vec![],
                estimated_memory: 2048,
            }
        };

        let total_cost = root.total_cost;

        Ok(QueryPlan {
            root,
            total_cost,
            planner_time: 0.5,
            execution_time: 0.0,
            planning_strategy: "cost-based".to_string(),
        })
    }

    /// Generate EXPLAIN output
    pub fn explain(&self, plan: &QueryPlan) -> String {
        let mut output = String::new();
        output.push_str(&self.explain_node(&plan.root, 0));
        output.push_str(&format!("\nPlan Total Cost: {:.2}\n", plan.total_cost));
        output.push_str(&format!("Planner Time: {:.2}ms\n", plan.planner_time));
        output
    }

    fn explain_node(&self, node: &QueryPlanNode, depth: usize) -> String {
        let indent = "  ".repeat(depth);
        let mut output = String::new();

        let op_name = match node.operation {
            OperationType::SeqScan => "Seq Scan",
            OperationType::IndexScan => "Index Scan",
            OperationType::IndexSeek => "Index Seek",
            OperationType::BitmapScan => "Bitmap Scan",
            OperationType::HashJoin => "Hash Join",
            OperationType::NestedLoopJoin => "Nested Loop",
            OperationType::MergeJoin => "Merge Join",
            OperationType::GroupAggregate => "Group Aggregate",
            OperationType::Sort => "Sort",
            OperationType::Limit => "Limit",
            OperationType::Filter => "Filter",
            OperationType::Projection => "Projection",
            OperationType::Union => "Union",
            OperationType::Append => "Append",
        };

        output.push_str(&format!(
            "{}{}  (cost={:.2}..{:.2} rows={} width={})\n",
            indent, op_name, node.startup_cost, node.total_cost, node.rows, node.width
        ));

        if let Some(table) = &node.table {
            output.push_str(&format!("{}  Relation: {}\n", indent, table));
        }

        if let Some(index) = &node.index {
            output.push_str(&format!("{}  Index: {}\n", indent, index));
        }

        if let Some(filter) = &node.filter {
            output.push_str(&format!("{}  Filter: {}\n", indent, filter));
        }

        for child in &node.children {
            output.push_str(&self.explain_node(child, depth + 1));
        }

        output
    }

    /// Estimate query cost
    pub fn estimate_cost(
        &self,
        table_size: usize,
        is_filtered: bool,
        has_index: bool,
    ) -> f64 {
        if has_index && is_filtered {
            self.cost_model.index_seek_cost + (table_size as f64 * 0.1 * self.cost_model.index_scan_cost)
        } else if has_index {
            self.cost_model.index_seek_cost + (table_size as f64 * self.cost_model.index_scan_cost)
        } else {
            table_size as f64 * self.cost_model.seq_scan_cost
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_optimizer() {
        let optimizer = QueryOptimizer::new();
        assert_eq!(optimizer.cost_model.seq_scan_cost, 1.0);
    }

    #[test]
    fn test_register_table_stats() {
        let mut optimizer = QueryOptimizer::new();
        optimizer.register_table_stats("users".to_string(), 10000);
        assert_eq!(optimizer.statistics.table_sizes.get("users"), Some(&10000));
    }

    #[test]
    fn test_register_column_selectivity() {
        let mut optimizer = QueryOptimizer::new();
        optimizer.register_column_selectivity("age".to_string(), 0.1);
        assert_eq!(
            optimizer.statistics.column_selectivity.get("age"),
            Some(&0.1)
        );
    }

    #[test]
    fn test_seq_scan_plan() {
        let mut optimizer = QueryOptimizer::new();
        optimizer.register_table_stats("users".to_string(), 1000);

        let plan = optimizer.build_plan("users", None, false).unwrap();
        assert_eq!(plan.root.operation, OperationType::SeqScan);
        assert_eq!(plan.root.rows, 1000);
    }

    #[test]
    fn test_index_scan_plan() {
        let mut optimizer = QueryOptimizer::new();
        optimizer.register_table_stats("users".to_string(), 1000);

        let plan = optimizer.build_plan("users", None, true).unwrap();
        assert_eq!(plan.root.operation, OperationType::IndexScan);
    }

    #[test]
    fn test_index_seek_plan() {
        let mut optimizer = QueryOptimizer::new();
        optimizer.register_table_stats("users".to_string(), 1000);
        optimizer.register_column_selectivity("id = 5".to_string(), 0.001);

        let plan = optimizer.build_plan("users", Some("id = 5"), true).unwrap();
        assert_eq!(plan.root.operation, OperationType::IndexSeek);
        assert!(plan.root.rows < 100); // Filtered
    }

    #[test]
    fn test_estimate_cost() {
        let optimizer = QueryOptimizer::new();

        let seq_cost = optimizer.estimate_cost(1000, false, false);
        let idx_cost = optimizer.estimate_cost(1000, false, true);
        let seek_cost = optimizer.estimate_cost(1000, true, true);

        assert!(seq_cost > idx_cost);
        assert!(idx_cost > seek_cost);
    }

    #[test]
    fn test_explain_output() {
        let mut optimizer = QueryOptimizer::new();
        optimizer.register_table_stats("users".to_string(), 1000);

        let plan = optimizer.build_plan("users", None, false).unwrap();
        let explain = optimizer.explain(&plan);

        assert!(explain.contains("Seq Scan"));
        assert!(explain.contains("Plan Total Cost"));
    }

    #[test]
    fn test_register_index_info() {
        let mut optimizer = QueryOptimizer::new();
        let idx_info = IndexInfo {
            name: "users_id_idx".to_string(),
            column: "id".to_string(),
            selectivity: 0.001,
            est_cost: 1.5,
        };

        optimizer.register_index(idx_info);
        assert!(optimizer
            .statistics
            .index_info
            .contains_key("users_id_idx"));
    }
}
