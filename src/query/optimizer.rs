//! Query Optimizer - планирование запросов на основе стоимости

use crate::query::parser::ParsedSelect;
use anyhow::Result;

#[derive(Debug, Clone, Copy)]
pub enum IndexType {
    FullScan,
    BTreeIndex,
    HashIndex,
    BloomFilter,
}

#[derive(Debug, Clone)]
pub struct AccessPath {
    pub index_type: IndexType,
    pub estimated_rows: usize,
    pub estimated_cost: f64,
    pub column: String,
}

#[derive(Debug, Clone)]
pub struct JoinPlan {
    pub left_alias: String,
    pub right_alias: String,
    pub join_type: String,
    pub join_keys: Vec<(String, String)>,
    pub estimated_rows: usize,
}

#[derive(Debug, Clone)]
pub struct QueryPlan {
    /// Table access path (which index to use)
    pub access_path: AccessPath,
    
    /// Join execution plans (if any)
    pub joins: Vec<JoinPlan>,
    
    /// Filter selectivity (0.0 - 1.0)
    pub filter_selectivity: f64,
    
    /// GROUP BY optimization
    pub use_streaming_aggregation: bool,
    
    /// Total estimated rows
    pub estimated_rows: usize,
    
    /// Total estimated cost
    pub total_cost: f64,
}

pub struct QueryOptimizer;

impl QueryOptimizer {
    /// Optimize parsed SELECT query
    pub fn optimize(query: &ParsedSelect, table_stats: TableStatistics) -> Result<QueryPlan> {
        // 1. Choose best access path
        let access_path = Self::choose_access_path(query, &table_stats);
        
        // 2. Estimate filter selectivity
        let filter_selectivity = Self::estimate_filter_selectivity(&query.filters);
        
        // 3. Optimize JOINs (if any)
        let joins = Self::optimize_joins(&query.joins)?;
        
        // 4. Estimate result rows
        let base_rows = (table_stats.row_count as f64 * filter_selectivity) as usize;
        let mut estimated_rows = base_rows;
        
        for join in &joins {
            estimated_rows = (estimated_rows as f64 * 0.5) as usize; // rough estimate
        }
        
        let total_cost = Self::calculate_total_cost(
            &access_path,
            filter_selectivity,
            &joins,
            table_stats.row_count,
        );
        
        Ok(QueryPlan {
            access_path,
            joins,
            filter_selectivity,
            use_streaming_aggregation: !query.group_by.is_empty(),
            estimated_rows,
            total_cost,
        })
    }

    /// Choose best access path for table
    fn choose_access_path(query: &ParsedSelect, stats: &TableStatistics) -> AccessPath {
        // 1. Check if we can use index for filters
        if !query.filters.is_empty() {
            let first_filter = &query.filters[0];
            
            // If table has index on filtered column, use it
            if stats.has_index_on(&first_filter.column) {
                return AccessPath {
                    index_type: IndexType::BTreeIndex,
                    estimated_rows: (stats.row_count as f64 * 0.05) as usize,
                    estimated_cost: 10.0,
                    column: first_filter.column.clone(),
                };
            }
        }
        
        // 2. Check if we can use index for ORDER BY
        if !query.order_by.is_empty() {
            let (sort_col, _) = &query.order_by[0];
            if stats.has_index_on(sort_col) {
                return AccessPath {
                    index_type: IndexType::BTreeIndex,
                    estimated_rows: stats.row_count,
                    estimated_cost: 20.0,
                    column: sort_col.clone(),
                };
            }
        }
        
        // 3. Default: full table scan
        AccessPath {
            index_type: IndexType::FullScan,
            estimated_rows: stats.row_count,
            estimated_cost: stats.row_count as f64 * 1.0,
            column: String::new(),
        }
    }

    /// Estimate selectivity of WHERE filters (0.0 = empty result, 1.0 = all rows)
    fn estimate_filter_selectivity(filters: &[crate::query::parser::Filter]) -> f64 {
        if filters.is_empty() {
            return 1.0;
        }
        
        // Conservative estimate: each equality filter reduces selectivity by 10x
        // Range filters reduce by 100x
        let mut selectivity = 1.0;
        
        for filter in filters {
            selectivity *= match filter.op {
                crate::query::parser::FilterOp::Equal => 0.1,         // 10%
                crate::query::parser::FilterOp::Like => 0.2,          // 20%
                crate::query::parser::FilterOp::GreaterThan |
                crate::query::parser::FilterOp::LessThan => 0.33,     // 33%
                crate::query::parser::FilterOp::Between => 0.1,       // 10%
                _ => 0.95,                                             // minimal impact
            };
        }
        
        selectivity.max(0.001) // at least 0.1%
    }

    /// Optimize JOIN order and execution
    fn optimize_joins(joins: &[crate::query::parser::ParsedJoin]) -> Result<Vec<JoinPlan>> {
        let mut plans = Vec::new();
        
        for join in joins {
            // For now, simple join planning
            // Full implementation would consider:
            // - Result cardinality of previous joins
            // - Index availability on join keys
            // - Join selectivity statistics
            
            plans.push(JoinPlan {
                left_alias: "left".to_string(),
                right_alias: join.table.clone(),
                join_type: format!("{:?}", join.join_type),
                join_keys: vec![(
                    join.on_condition.column.clone(),
                    join.on_condition.value.clone(),
                )],
                estimated_rows: 1000, // placeholder
            });
        }
        
        Ok(plans)
    }

    /// Calculate total estimated cost
    fn calculate_total_cost(
        access_path: &AccessPath,
        filter_selectivity: f64,
        joins: &[JoinPlan],
        total_rows: usize,
    ) -> f64 {
        let mut cost = access_path.estimated_cost;
        
        // Add cost of applying filters
        cost += (total_rows as f64 * filter_selectivity) * 0.001;
        
        // Add cost of JOINs
        for join in joins {
            cost += join.estimated_rows as f64 * 0.01;
        }
        
        cost
    }

    /// Recommend best index for query
    pub fn recommend_indexes(query: &ParsedSelect) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        // Recommend index on filtered columns
        for filter in &query.filters {
            recommendations.push(format!(
                "CREATE INDEX idx_{} ON table({})",
                filter.column, filter.column
            ));
        }
        
        // Recommend index on ORDER BY columns
        for (col, _) in &query.order_by {
            recommendations.push(format!(
                "CREATE INDEX idx_{} ON table({})",
                col, col
            ));
        }
        
        // Recommend composite index for complex queries
        if !query.filters.is_empty() && !query.order_by.is_empty() {
            let mut cols = vec![];
            for filter in &query.filters {
                cols.push(filter.column.clone());
            }
            for (col, _) in &query.order_by {
                cols.push(col.clone());
            }
            let col_list = cols.join(", ");
            recommendations.push(format!(
                "CREATE INDEX idx_composite ON table({})",
                col_list
            ));
        }
        
        recommendations
    }
}

/// Table statistics for query planning
#[derive(Debug, Clone)]
pub struct TableStatistics {
    pub name: String,
    pub row_count: usize,
    pub column_stats: std::collections::HashMap<String, ColumnStats>,
    pub indexes: Vec<IndexInfo>,
}

#[derive(Debug, Clone)]
pub struct ColumnStats {
    pub name: String,
    pub data_type: String,
    pub null_count: usize,
    pub distinct_count: usize,
    pub max_length: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct IndexInfo {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
}

impl TableStatistics {
    pub fn new(name: String, row_count: usize) -> Self {
        TableStatistics {
            name,
            row_count,
            column_stats: std::collections::HashMap::new(),
            indexes: Vec::new(),
        }
    }

    pub fn add_column_stat(&mut self, stat: ColumnStats) {
        self.column_stats.insert(stat.name.clone(), stat);
    }

    pub fn add_index(&mut self, index: IndexInfo) {
        self.indexes.push(index);
    }

    pub fn has_index_on(&self, column: &str) -> bool {
        self.indexes.iter().any(|idx| idx.columns.contains(&column.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_selectivity() {
        let filters = vec![];
        assert_eq!(QueryOptimizer::estimate_filter_selectivity(&filters), 1.0);
    }

    #[test]
    fn test_access_path_selection() {
        let stats = TableStatistics::new("users".to_string(), 1_000_000);
        let query = ParsedSelect {
            table: "users".to_string(),
            columns: vec!["*".to_string()],
            filters: vec![],
            joins: vec![],
            group_by: vec![],
            order_by: vec![],
            limit: None,
            offset: None,
            distinct: false,
        };

        let path = QueryOptimizer::choose_access_path(&query, &stats);
        assert!(matches!(path.index_type, IndexType::FullScan));
    }

    #[test]
    fn test_index_recommendations() {
        let query = ParsedSelect {
            table: "users".to_string(),
            columns: vec!["*".to_string()],
            filters: vec![crate::query::parser::Filter {
                column: "age".to_string(),
                op: crate::query::parser::FilterOp::GreaterThan,
                value: "30".to_string(),
            }],
            joins: vec![],
            group_by: vec![],
            order_by: vec![("name".to_string(), true)],
            limit: None,
            offset: None,
            distinct: false,
        };

        let recommendations = QueryOptimizer::recommend_indexes(&query);
        assert!(!recommendations.is_empty());
    }
}
