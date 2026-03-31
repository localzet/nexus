//! Query Planning & Optimization - планирование и выражение реляционных на ЭРА
/// Query plan generation, cost estimation, and execution planning
use crate::types::Table;
use anyhow::{Result, anyhow};
use std::collections::HashMap;

/// Query execution plan type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanType {
    FullTableScan,
    IndexScan,
    IndexSeek,
    NestedLoop,
    HashJoin,
    SortMergeJoin,
}

impl PlanType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlanType::FullTableScan => "FullTableScan",
            PlanType::IndexScan => "IndexScan",
            PlanType::IndexSeek => "IndexSeek",
            PlanType::NestedLoop => "NestedLoopJoin",
            PlanType::HashJoin => "HashJoin",
            PlanType::SortMergeJoin => "SortMergeJoin",
        }
    }
}

/// Query cost estimation
#[derive(Debug, Clone)]
pub struct QueryCost {
    pub cpu_cost: f64,
    pub io_cost: f64,
    pub memory_cost: f64,
    pub total_cost: f64,
}

impl QueryCost {
    pub fn new(cpu: f64, io: f64, memory: f64) -> Self {
        Self {
            cpu_cost: cpu,
            io_cost: io,
            memory_cost: memory,
            total_cost: (cpu * 1.0) + (io * 10.0) + (memory * 0.1),
        }
    }

    pub fn estimate_rows(&self) -> u64 {
        (self.total_cost * 10.0) as u64
    }
}

/// Column statistics for selectivity estimation
#[derive(Debug, Clone)]
pub struct ColumnStats {
    pub column_name: String,
    pub null_count: u64,
    pub distinct_count: u64,
    pub min_value: Option<String>,
    pub max_value: Option<String>,
    pub avg_length: u64,
}

impl ColumnStats {
    pub fn new(name: String) -> Self {
        Self {
            column_name: name,
            null_count: 0,
            distinct_count: 1,
            min_value: None,
            max_value: None,
            avg_length: 10,
        }
    }

    pub fn selectivity(&self, total_rows: u64) -> f64 {
        if self.distinct_count == 0 {
            0.0
        } else {
            1.0 / (self.distinct_count as f64).max(1.0)
        }
    }
}

/// Table statistics for cost estimation
#[derive(Debug, Clone)]
pub struct TableStats {
    pub table_name: String,
    pub row_count: u64,
    pub avg_row_size: u64,
    pub columns: HashMap<String, ColumnStats>,
}

impl TableStats {
    pub fn new(name: String, row_count: u64) -> Self {
        Self {
            table_name: name,
            row_count,
            avg_row_size: 100,
            columns: HashMap::new(),
        }
    }

    pub fn add_column_stats(&mut self, stats: ColumnStats) {
        self.columns.insert(stats.column_name.clone(), stats);
    }

    pub fn get_column_selectivity(&self, col: &str) -> f64 {
        self.columns
            .get(col)
            .map(|s| s.selectivity(self.row_count))
            .unwrap_or(1.0)
    }
}

/// Query plan node in the execution tree
#[derive(Debug, Clone)]
pub struct PlanNode {
    pub plan_type: PlanType,
    pub cost: QueryCost,
    pub estimated_rows: u64,
    pub description: String,
    pub children: Vec<Box<PlanNode>>,
}

impl PlanNode {
    pub fn new(plan_type: PlanType, description: String) -> Self {
        Self {
            plan_type,
            cost: QueryCost::new(0.0, 0.0, 0.0),
            estimated_rows: 0,
            description,
            children: Vec::new(),
        }
    }

    pub fn with_cost(mut self, cpu: f64, io: f64, memory: f64) -> Self {
        self.cost = QueryCost::new(cpu, io, memory);
        self.estimated_rows = self.cost.estimate_rows();
        self
    }

    pub fn add_child(&mut self, child: PlanNode) {
        self.children.push(Box::new(child));
    }
}

/// Query planner for generating execution plans
pub struct QueryPlanner {
    table_stats: HashMap<String, TableStats>,
}

impl QueryPlanner {
    pub fn new() -> Self {
        Self {
            table_stats: HashMap::new(),
        }
    }

    pub fn register_table_stats(&mut self, stats: TableStats) {
        self.table_stats.insert(stats.table_name.clone(), stats);
    }

    pub fn register_column_selectivity(&mut self, table: &str, column: &str, selectivity: f64) -> Result<()> {
        let stats = self.table_stats.get_mut(table)
            .ok_or_else(|| anyhow!("Table '{}' not found", table))?;
        
        if let Some(col_stat) = stats.columns.get_mut(column) {
            // Estimate distinct count from selectivity
            col_stat.distinct_count = ((stats.row_count as f64) * selectivity) as u64;
        }
        
        Ok(())
    }

    pub fn register_index_info(&mut self, table: &str, column: &str, is_clustered: bool) -> Result<()> {
        let stats = self.table_stats.get_mut(table)
            .ok_or_else(|| anyhow!("Table '{}' not found", table))?;
        
        if let Some(col_stat) = stats.columns.get_mut(column) {
            // Mark indexed column with metadata
            col_stat.column_name = format!("{}(indexed{})", col_stat.column_name, if is_clustered { ":clustered" } else { "" });
        }
        
        Ok(())
    }

    /// Plan a simple SELECT query with potential WHERE clause
    pub fn plan_select(&self, table: &str, has_where: bool) -> Result<PlanNode> {
        let table_stats = self.table_stats.get(table)
            .ok_or_else(|| anyhow!("Table '{}' not found", table))?;

        if has_where {
            // Estimate with WHERE clause (assume 10% selectivity by default)
            let estimated_rows = (table_stats.row_count as f64 * 0.1) as u64;
            let cpu_cost = estimated_rows as f64 * 0.001;
            let io_cost = (table_stats.row_count as f64 * 0.01) as f64;
            
            let mut plan = PlanNode::new(
                PlanType::FullTableScan,
                format!("Scan table '{}' with WHERE filter", table)
            ).with_cost(cpu_cost, io_cost, 5.0);
            
            plan.estimated_rows = estimated_rows;
            Ok(plan)
        } else {
            // Full table scan
            let cpu_cost = table_stats.row_count as f64 * 0.001;
            let io_cost = (table_stats.row_count / 100) as f64;
            
            let mut plan = PlanNode::new(
                PlanType::FullTableScan,
                format!("Scan table '{}'", table)
            ).with_cost(cpu_cost, io_cost, 0.0);
            
            plan.estimated_rows = table_stats.row_count;
            Ok(plan)
        }
    }

    /// Plan a JOIN operation
    pub fn plan_join(&self, left_table: &str, right_table: &str, join_type: &str) -> Result<PlanNode> {
        let left_stats = self.table_stats.get(left_table)
            .ok_or_else(|| anyhow!("Table '{}' not found", left_table))?;
        let right_stats = self.table_stats.get(right_table)
            .ok_or_else(|| anyhow!("Table '{}' not found", right_table))?;

        // Estimate JOIN result rows
        let estimate_bytes = (left_stats.row_count * right_stats.row_count * left_stats.avg_row_size) / 100;
        
        // Choose JOIN algorithm based on table sizes
        let plan_type = if estimate_bytes < 10_000_000 {
            PlanType::HashJoin
        } else {
            PlanType::NestedLoop
        };

        let cpu_cost = (left_stats.row_count + right_stats.row_count) as f64 * 0.01;
        let io_cost = ((left_stats.row_count / 100) + (right_stats.row_count / 100)) as f64;

        let mut plan = PlanNode::new(
            plan_type,
            format!("{} JOIN '{}' and '{}'", join_type.to_uppercase(), left_table, right_table)
        ).with_cost(cpu_cost, io_cost, estimate_bytes as f64 / 1_000_000.0);

        plan.estimated_rows = (left_stats.row_count * right_stats.row_count) / 100;
        Ok(plan)
    }

    /// Plan GROUP BY aggregation
    pub fn plan_aggregation(&self, table: &str, group_columns: usize) -> Result<PlanNode> {
        let table_stats = self.table_stats.get(table)
            .ok_or_else(|| anyhow!("Table '{}' not found", table))?;

        let estimated_groups = ((table_stats.row_count as f64).sqrt() / group_columns.max(1) as f64) as u64;
        let cpu_cost = table_stats.row_count as f64 * 0.01;
        let io_cost = (estimated_groups as f64) * 0.1;
        let memory_cost = estimated_groups as f64 * 0.001;

        let mut plan = PlanNode::new(
            PlanType::FullTableScan,
            format!("Stream Aggregate on '{}' with {} group(s)", table, group_columns)
        ).with_cost(cpu_cost, io_cost, memory_cost);

        plan.estimated_rows = estimated_groups;
        Ok(plan)
    }
}

/// EXPLAIN output formatter
pub struct ExplainFormatter;

impl ExplainFormatter {
    pub fn format_plan(plan: &PlanNode, depth: usize) -> String {
        let mut result = String::new();
        let indent = "  ".repeat(depth);
        
        result.push_str(&format!(
            "{}{} (Cost: CPU={:.2}, IO={:.2}, Total={:.2}; Rows={})\n",
            indent,
            plan.plan_type.as_str(),
            plan.cost.cpu_cost,
            plan.cost.io_cost,
            plan.cost.total_cost,
            plan.estimated_rows
        ));
        result.push_str(&format!("{}  Description: {}\n", indent, plan.description));

        for child in &plan.children {
            result.push_str(&Self::format_plan(child, depth + 1));
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_cost_calculation() {
        let cost = QueryCost::new(10.0, 5.0, 2.0);
        assert!(cost.total_cost > 0.0);
        assert_eq!(cost.cpu_cost, 10.0);
        assert_eq!(cost.io_cost, 5.0);
    }

    #[test]
    fn test_plan_type_string() {
        assert_eq!(PlanType::FullTableScan.as_str(), "FullTableScan");
        assert_eq!(PlanType::IndexSeek.as_str(), "IndexSeek");
        assert_eq!(PlanType::HashJoin.as_str(), "HashJoin");
    }

    #[test]
    fn test_column_stats_creation() {
        let stats = ColumnStats::new("id".to_string());
        assert_eq!(stats.column_name, "id");
        assert_eq!(stats.distinct_count, 1);
    }

    #[test]
    fn test_column_selectivity() {
        let mut stats = ColumnStats::new("status".to_string());
        stats.distinct_count = 5; // 5 distinct values
        let selectivity = stats.selectivity(100); // 100 rows
        assert!(selectivity > 0.0 && selectivity <= 1.0);
    }

    #[test]
    fn test_table_stats_creation() {
        let stats = TableStats::new("users".to_string(), 10000);
        assert_eq!(stats.table_name, "users");
        assert_eq!(stats.row_count, 10000);
        assert_eq!(stats.columns.len(), 0);
    }

    #[test]
    fn test_table_stats_column_addition() {
        let mut stats = TableStats::new("users".to_string(), 10000);
        let col_stats = ColumnStats::new("id".to_string());
        stats.add_column_stats(col_stats);
        assert_eq!(stats.columns.len(), 1);
    }

    #[test]
    fn test_plan_node_creation() -> Result<()> {
        let plan = PlanNode::new(PlanType::FullTableScan, "Test scan".to_string());
        assert_eq!(plan.plan_type, PlanType::FullTableScan);
        assert_eq!(plan.children.len(), 0);
        Ok(())
    }

    #[test]
    fn test_plan_node_with_cost() {
        let plan = PlanNode::new(PlanType::FullTableScan, "Test".to_string())
            .with_cost(10.0, 5.0, 1.0);
        assert!(plan.cost.total_cost > 0.0);
        assert!(plan.estimated_rows > 0);
    }

    #[test]
    fn test_plan_node_hierarchy() {
        let mut parent = PlanNode::new(PlanType::HashJoin, "JOIN".to_string());
        let child = PlanNode::new(PlanType::FullTableScan, "SCAN".to_string());
        parent.add_child(child);
        assert_eq!(parent.children.len(), 1);
    }

    #[test]
    fn test_query_planner_creation() {
        let planner = QueryPlanner::new();
        assert_eq!(planner.table_stats.len(), 0);
    }

    #[test]
    fn test_query_planner_register_table() {
        let mut planner = QueryPlanner::new();
        let stats = TableStats::new("products".to_string(), 5000);
        planner.register_table_stats(stats);
        assert_eq!(planner.table_stats.len(), 1);
    }

    #[test]
    fn test_plan_full_table_scan() -> Result<()> {
        let mut planner = QueryPlanner::new();
        planner.register_table_stats(TableStats::new("users".to_string(), 10000));
        
        let plan = planner.plan_select("users", false)?;
        assert_eq!(plan.plan_type, PlanType::FullTableScan);
        assert_eq!(plan.estimated_rows, 10000);
        Ok(())
    }

    #[test]
    fn test_plan_with_where_clause() -> Result<()> {
        let mut planner = QueryPlanner::new();
        planner.register_table_stats(TableStats::new("users".to_string(), 10000));
        
        let plan = planner.plan_select("users", true)?;
        assert_eq!(plan.plan_type, PlanType::FullTableScan);
        assert!(plan.estimated_rows < 10000); // WHERE reduces estimated rows
        Ok(())
    }

    #[test]
    fn test_plan_join_operation() -> Result<()> {
        let mut planner = QueryPlanner::new();
        planner.register_table_stats(TableStats::new("users".to_string(), 5000));
        planner.register_table_stats(TableStats::new("orders".to_string(), 20000));
        
        let plan = planner.plan_join("users", "orders", "INNER")?;
        assert!(matches!(plan.plan_type, PlanType::HashJoin | PlanType::NestedLoop));
        Ok(())
    }

    #[test]
    fn test_plan_aggregation() -> Result<()> {
        let mut planner = QueryPlanner::new();
        planner.register_table_stats(TableStats::new("sales".to_string(), 100000));
        
        let plan = planner.plan_aggregation("sales", 2)?;
        assert!(plan.estimated_rows < 100000);
        Ok(())
    }

    #[test]
    fn test_explain_format_simple_plan() -> Result<()> {
        let plan = PlanNode::new(PlanType::FullTableScan, "Simple scan".to_string())
            .with_cost(5.0, 2.0, 1.0);
        
        let explanation = ExplainFormatter::format_plan(&plan, 0);
        assert!(explanation.contains("FullTableScan"));
        assert!(explanation.contains("Simple scan"));
        Ok(())
    }

    #[test]
    fn test_explain_format_nested_plan() {
        let mut parent = PlanNode::new(PlanType::HashJoin, "JOIN operation".to_string())
            .with_cost(15.0, 8.0, 5.0);
        
        let child = PlanNode::new(PlanType::FullTableScan, "Left scan".to_string())
            .with_cost(5.0, 2.0, 1.0);
        
        parent.add_child(child);
        let explanation = ExplainFormatter::format_plan(&parent, 0);
        
        assert!(explanation.contains("HashJoin"));
        assert!(explanation.contains("FullTableScan"));
    }

    #[test]
    fn test_query_planner_register_selectivity() -> Result<()> {
        let mut planner = QueryPlanner::new();
        let mut stats = TableStats::new("products".to_string(), 1000);
        let mut col_stat = ColumnStats::new("category".to_string());
        col_stat.distinct_count = 20;
        stats.add_column_stats(col_stat);
        planner.register_table_stats(stats);
        
        planner.register_column_selectivity("products", "category", 0.05)?;
        Ok(())
    }

    #[test]
    fn test_query_planner_register_index() -> Result<()> {
        let mut planner = QueryPlanner::new();
        let mut stats = TableStats::new("users".to_string(), 50000);
        let col_stat = ColumnStats::new("id".to_string());
        stats.add_column_stats(col_stat);
        planner.register_table_stats(stats);
        
        planner.register_index_info("users", "id", true)?;
        Ok(())
    }

    #[test]
    fn test_plan_cost_comparison() -> Result<()> {
        let mut planner = QueryPlanner::new();
        planner.register_table_stats(TableStats::new("large_table".to_string(), 1_000_000));
        
        let plan1 = planner.plan_select("large_table", false)?;
        let plan2 = planner.plan_select("large_table", true)?;
        
        // Plan with WHERE should have lower cost
        assert!(plan2.cost.total_cost < plan1.cost.total_cost);
        Ok(())
    }
}
