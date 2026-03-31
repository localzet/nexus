//! Оптимизация запросов на основе статистики и оценок

use std::collections::HashMap;
use crate::types::Value;

/// Column statistics for histogram-based estimation
#[derive(Debug, Clone)]
pub struct ColumnStatistics {
    pub column_name: String,
    pub data_type: String,
    pub null_count: i64,
    pub distinct_count: i64,
    pub min_value: Option<Value>,
    pub max_value: Option<Value>,
    pub average_length: f64,
    pub last_updated: i64,
}

impl ColumnStatistics {
    pub fn new(column_name: String, data_type: String) -> Self {
        Self {
            column_name,
            data_type,
            null_count: 0,
            distinct_count: 0,
            min_value: None,
            max_value: None,
            average_length: 0.0,
            last_updated: 0,
        }
    }

    pub fn update_bounds(&mut self, min: Option<Value>, max: Option<Value>) {
        self.min_value = min;
        self.max_value = max;
    }

    pub fn get_selectivity(&self, total_rows: i64) -> f64 {
        if total_rows == 0 {
            0.0
        } else {
            (self.distinct_count as f64) / (total_rows as f64)
        }
    }
}

/// Histogram bucket for distribution tracking
#[derive(Debug, Clone)]
pub struct HistogramBucket {
    pub lower_bound: Value,
    pub upper_bound: Value,
    pub row_count: i64,
    pub distinct_count: i64,
}

impl HistogramBucket {
    pub fn new(lower: Value, upper: Value, row_count: i64, distinct_count: i64) -> Self {
        Self {
            lower_bound: lower,
            upper_bound: upper,
            row_count,
            distinct_count,
        }
    }

    pub fn get_density(&self) -> f64 {
        if self.distinct_count == 0 {
            0.0
        } else {
            (self.row_count as f64) / (self.distinct_count as f64)
        }
    }
}

/// Histogram for column data distribution
#[derive(Debug, Clone)]
pub struct Histogram {
    pub column_name: String,
    pub buckets: Vec<HistogramBucket>,
    pub total_rows: i64,
}

impl Histogram {
    pub fn new(column_name: String, buckets: Vec<HistogramBucket>) -> Self {
        let total_rows: i64 = buckets.iter().map(|b| b.row_count).sum();
        Self {
            column_name,
            buckets,
            total_rows,
        }
    }

    pub fn estimate_cardinality(&self, value: &Value) -> i64 {
        for bucket in &self.buckets {
            // Simple range check
            let in_range = match (&bucket.lower_bound, &bucket.upper_bound, value) {
                (Value::Integer(l), Value::Integer(u), Value::Integer(v)) => v >= l && v <= u,
                (Value::String(l), Value::String(u), Value::String(v)) => v >= l && v <= u,
                _ => false,
            };

            if in_range {
                return bucket.row_count / std::cmp::max(1, bucket.distinct_count);
            }
        }
        0
    }

    pub fn get_bucket_count(&self) -> usize {
        self.buckets.len()
    }
}

/// Predicate characteristics for optimization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredicateType {
    Equality,
    Range,
    Like,
    In,
    Between,
    IsNull,
}

impl PredicateType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PredicateType::Equality => "Equality",
            PredicateType::Range => "Range",
            PredicateType::Like => "Like",
            PredicateType::In => "In",
            PredicateType::Between => "Between",
            PredicateType::IsNull => "IsNull",
        }
    }
}

/// Predicate for query optimization
#[derive(Debug, Clone)]
pub struct Predicate {
    pub column: String,
    pub predicate_type: PredicateType,
    pub values: Vec<Value>,
    pub selectivity: f64,
}

impl Predicate {
    pub fn new(column: String, predicate_type: PredicateType) -> Self {
        Self {
            column,
            predicate_type,
            values: Vec::new(),
            selectivity: 1.0,
        }
    }

    pub fn with_values(mut self, values: Vec<Value>) -> Self {
        self.values = values;
        self
    }

    pub fn with_selectivity(mut self, selectivity: f64) -> Self {
        self.selectivity = selectivity.max(0.0).min(1.0);
        self
    }
}

/// Statistics collector for schema
#[derive(Debug, Clone)]
pub struct StatisticsCollector {
    column_stats: HashMap<String, ColumnStatistics>,
    histograms: HashMap<String, Histogram>,
    table_row_count: i64,
}

impl StatisticsCollector {
    pub fn new() -> Self {
        Self {
            column_stats: HashMap::new(),
            histograms: HashMap::new(),
            table_row_count: 0,
        }
    }

    /// Register column statistics
    pub fn register_column_stats(&mut self, stats: ColumnStatistics) -> anyhow::Result<()> {
        if self.column_stats.contains_key(&stats.column_name) {
            return Err(anyhow::anyhow!("Stats for {} already exist", stats.column_name));
        }
        self.column_stats.insert(stats.column_name.clone(), stats);
        Ok(())
    }

    /// Get column statistics
    pub fn get_column_stats(&self, column: &str) -> Option<&ColumnStatistics> {
        self.column_stats.get(column)
    }

    /// Register histogram
    pub fn register_histogram(&mut self, histogram: Histogram) -> anyhow::Result<()> {
        if self.histograms.contains_key(&histogram.column_name) {
            return Err(anyhow::anyhow!(
                "Histogram for {} already exists",
                histogram.column_name
            ));
        }
        self.histograms.insert(histogram.column_name.clone(), histogram);
        Ok(())
    }

    /// Get histogram
    pub fn get_histogram(&self, column: &str) -> Option<&Histogram> {
        self.histograms.get(column)
    }

    /// Set table row count
    pub fn set_table_row_count(&mut self, count: i64) {
        self.table_row_count = count;
    }

    /// Estimate selectivity for a predicate
    pub fn estimate_selectivity(&self, predicate: &Predicate) -> f64 {
        match predicate.predicate_type {
            PredicateType::Equality => {
                if let Some(stats) = self.get_column_stats(&predicate.column) {
                    if stats.distinct_count > 0 {
                        return 1.0 / (stats.distinct_count as f64);
                    }
                }
                0.1
            }
            PredicateType::Range => {
                if let Some(hist) = self.get_histogram(&predicate.column) {
                    let total_buckets = hist.get_bucket_count() as f64;
                    return 1.0 / total_buckets;
                }
                0.33
            }
            PredicateType::Like => 0.1,
            PredicateType::In => {
                if let Some(stats) = self.get_column_stats(&predicate.column) {
                    if stats.distinct_count > 0 {
                        let in_list_size = predicate.values.len() as f64;
                        return (in_list_size) / (stats.distinct_count as f64);
                    }
                }
                0.1
            }
            PredicateType::Between => 0.05,
            PredicateType::IsNull => {
                if let Some(stats) = self.get_column_stats(&predicate.column) {
                    if self.table_row_count > 0 {
                        return (stats.null_count as f64) / (self.table_row_count as f64);
                    }
                }
                0.01
            }
        }
    }

    /// Get statistics count
    pub fn get_stats_count(&self) -> usize {
        self.column_stats.len()
    }

    /// Get histogram count
    pub fn get_histogram_count(&self) -> usize {
        self.histograms.len()
    }

    /// Get table row count
    pub fn get_table_row_count(&self) -> i64 {
        self.table_row_count
    }
}

/// Query optimizer using statistics
#[derive(Debug, Clone)]
pub struct QueryOptimizer {
    stats: StatisticsCollector,
}

impl QueryOptimizer {
    pub fn new(stats: StatisticsCollector) -> Self {
        Self { stats }
    }

    /// Estimate result cardinality for a set of predicates
    pub fn estimate_output_rows(&self, predicates: &[Predicate]) -> i64 {
        let mut selectivity = 1.0;

        for pred in predicates {
            selectivity *= self.stats.estimate_selectivity(pred);
        }

        ((self.stats.get_table_row_count() as f64) * selectivity).ceil() as i64
    }

    /// Order predicates by selectivity (most selective first)
    pub fn order_by_selectivity(&self, mut predicates: Vec<Predicate>) -> Vec<Predicate> {
        predicates.sort_by(|a, b| {
            let sel_a = self.stats.estimate_selectivity(a);
            let sel_b = self.stats.estimate_selectivity(b);
            sel_a.partial_cmp(&sel_b).unwrap_or(std::cmp::Ordering::Equal)
        });
        predicates
    }

    /// Check if column is indexed (mock implementation)
    pub fn is_column_indexed(&self, column: &str) -> bool {
        // In a real implementation, would check index metadata
        column == "id" || column.ends_with("_id")
    }

    /// Suggest index for a column based on statistics
    pub fn suggest_index(&self, column: &str) -> bool {
        if let Some(stats) = self.stats.get_column_stats(column) {
            // Suggest index if column has high distinct count
            stats.distinct_count > (self.stats.get_table_row_count() / 10)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_statistics_creation() {
        let stats = ColumnStatistics::new("age".to_string(), "integer".to_string());
        assert_eq!(stats.column_name, "age");
        assert_eq!(stats.data_type, "integer");
    }

    #[test]
    fn test_column_statistics_selectivity() {
        let mut stats = ColumnStatistics::new("id".to_string(), "integer".to_string());
        stats.distinct_count = 1000;
        
        let selectivity = stats.get_selectivity(10000);
        assert!((selectivity - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_histogram_bucket() {
        let bucket = HistogramBucket::new(
            Value::Integer(1),
            Value::Integer(100),
            1000,
            50,
        );
        assert_eq!(bucket.row_count, 1000);
        assert!(bucket.get_density() > 0.0);
    }

    #[test]
    fn test_histogram_creation() {
        let buckets = vec![
            HistogramBucket::new(Value::Integer(1), Value::Integer(50), 100, 50),
            HistogramBucket::new(Value::Integer(51), Value::Integer(100), 100, 50),
        ];
        let hist = Histogram::new("age".to_string(), buckets);
        assert_eq!(hist.total_rows, 200);
        assert_eq!(hist.get_bucket_count(), 2);
    }

    #[test]
    fn test_predicate_creation() {
        let pred = Predicate::new("salary".to_string(), PredicateType::Range);
        assert_eq!(pred.column, "salary");
        assert_eq!(pred.predicate_type, PredicateType::Range);
    }

    #[test]
    fn test_predicate_with_values() {
        let pred = Predicate::new("status".to_string(), PredicateType::In)
            .with_values(vec![
                Value::String("active".to_string()),
                Value::String("inactive".to_string()),
            ]);
        assert_eq!(pred.values.len(), 2);
    }

    #[test]
    fn test_predicate_type_string() {
        assert_eq!(PredicateType::Equality.as_str(), "Equality");
        assert_eq!(PredicateType::Range.as_str(), "Range");
        assert_eq!(PredicateType::IsNull.as_str(), "IsNull");
    }

    #[test]
    fn test_statistics_collector_creation() {
        let collector = StatisticsCollector::new();
        assert_eq!(collector.get_stats_count(), 0);
        assert_eq!(collector.get_table_row_count(), 0);
    }

    #[test]
    fn test_statistics_collector_register_stats() -> anyhow::Result<()> {
        let mut collector = StatisticsCollector::new();
        let stats = ColumnStatistics::new("id".to_string(), "integer".to_string());
        
        collector.register_column_stats(stats)?;
        assert_eq!(collector.get_stats_count(), 1);
        Ok(())
    }

    #[test]
    fn test_statistics_collector_register_histogram() -> anyhow::Result<()> {
        let mut collector = StatisticsCollector::new();
        let buckets = vec![
            HistogramBucket::new(Value::Integer(1), Value::Integer(50), 100, 50),
        ];
        let hist = Histogram::new("age".to_string(), buckets);
        
        collector.register_histogram(hist)?;
        assert_eq!(collector.get_histogram_count(), 1);
        Ok(())
    }

    #[test]
    fn test_statistics_collector_selectivity_estimation() {
        let mut collector = StatisticsCollector::new();
        let mut stats = ColumnStatistics::new("id".to_string(), "integer".to_string());
        stats.distinct_count = 1000;
        
        collector.register_column_stats(stats).ok();
        collector.set_table_row_count(10000);
        
        let pred = Predicate::new("id".to_string(), PredicateType::Equality);
        let selectivity = collector.estimate_selectivity(&pred);
        assert!((selectivity - 0.001).abs() < 0.0001);
    }

    #[test]
    fn test_query_optimizer_creation() {
        let collector = StatisticsCollector::new();
        let optimizer = QueryOptimizer::new(collector);
        assert_eq!(optimizer.stats.get_stats_count(), 0);
    }

    #[test]
    fn test_query_optimizer_estimate_cardinality() {
        let mut collector = StatisticsCollector::new();
        let mut stats = ColumnStatistics::new("status".to_string(), "varchar".to_string());
        stats.distinct_count = 10;
        
        collector.register_column_stats(stats).ok();
        collector.set_table_row_count(10000);
        
        let optimizer = QueryOptimizer::new(collector);
        let predicates = vec![
            Predicate::new("status".to_string(), PredicateType::Equality),
        ];
        
        let estimated = optimizer.estimate_output_rows(&predicates);
        assert!(estimated > 0);
    }

    #[test]
    fn test_query_optimizer_order_by_selectivity() {
        let collector = StatisticsCollector::new();
        let optimizer = QueryOptimizer::new(collector);
        
        let predicates = vec![
            Predicate::new("col1".to_string(), PredicateType::Range),
            Predicate::new("col2".to_string(), PredicateType::Like),
        ];
        
        let ordered = optimizer.order_by_selectivity(predicates);
        assert_eq!(ordered.len(), 2);
    }

    #[test]
    fn test_query_optimizer_index_suggestion() {
        let mut collector = StatisticsCollector::new();
        let mut stats = ColumnStatistics::new("user_id".to_string(), "integer".to_string());
        stats.distinct_count = 5000;
        
        collector.register_column_stats(stats).ok();
        collector.set_table_row_count(5000);
        
        let optimizer = QueryOptimizer::new(collector);
        let should_index = optimizer.suggest_index("user_id");
        assert!(should_index);
    }

    #[test]
    fn test_predicate_selectivity_with_in() {
        let mut collector = StatisticsCollector::new();
        let mut stats = ColumnStatistics::new("category".to_string(), "varchar".to_string());
        stats.distinct_count = 100;
        
        collector.register_column_stats(stats).ok();
        
        let pred = Predicate::new("category".to_string(), PredicateType::In)
            .with_values(vec![
                Value::String("A".to_string()),
                Value::String("B".to_string()),
                Value::String("C".to_string()),
            ]);
        
        let selectivity = collector.estimate_selectivity(&pred);
        assert!(selectivity > 0.0);
    }
}
