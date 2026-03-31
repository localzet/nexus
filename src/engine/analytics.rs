//! Аналитика и мониторинг - сбор метрик и производственная наблюдаемость

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use chrono::{DateTime, Utc};
use anyhow::Result;

/// Performance metric type
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum MetricType {
    QueryCount,
    QueryLatency,
    InsertCount,
    DeleteCount,
    UpdateCount,
    CacheHitRate,
    MemoryUsage,
    DiskUsage,
    ConnectionCount,
    TransactionCount,
    LockWaits,
    NetworkLatency,
}

/// Performance metric value
#[derive(Debug, Clone)]
pub struct MetricValue {
    pub metric_type: MetricType,
    pub value: f64,
    pub timestamp: DateTime<Utc>,
    pub dimension: String, // e.g., table name, query type
}

/// Performance statistics
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub min_value: f64,
    pub max_value: f64,
    pub avg_value: f64,
    pub count: usize,
    pub percentile_95: f64,
    pub percentile_99: f64,
}

/// Query execution statistics
#[derive(Debug, Clone)]
pub struct QueryStats {
    pub query_id: String,
    pub query_text: String,
    pub execution_count: u64,
    pub total_time_ms: u64,
    pub avg_time_ms: f64,
    pub min_time_ms: u64,
    pub max_time_ms: u64,
    pub rows_scanned: u64,
    pub rows_returned: u64,
}

/// Database health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Down,
}

/// Health check result
#[derive(Debug, Clone)]
pub struct HealthCheck {
    pub status: HealthStatus,
    pub checks: HashMap<String, bool>,
    pub timestamp: DateTime<Utc>,
    pub message: String,
}

/// Metrics collector and analyzer
pub struct MetricsCollector {
    metrics: Vec<MetricValue>,
    query_stats: HashMap<String, QueryStats>,
    counters: HashMap<String, Arc<AtomicU64>>,
    gauges: HashMap<String, f64>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Vec::new(),
            query_stats: HashMap::new(),
            counters: HashMap::new(),
            gauges: HashMap::new(),
        }
    }

    /// Record a metric value
    pub fn record_metric(&mut self, metric: MetricValue) {
        self.metrics.push(metric);
    }

    /// Increment counter
    pub fn increment_counter(&mut self, name: &str, amount: u64) {
        let counter = self
            .counters
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(AtomicU64::new(0)));

        counter.fetch_add(amount, Ordering::SeqCst);
    }

    /// Set gauge value
    pub fn set_gauge(&mut self, name: &str, value: f64) {
        self.gauges.insert(name.to_string(), value);
    }

    /// Get counter value
    pub fn get_counter(&self, name: &str) -> Option<u64> {
        self.counters.get(name).map(|c| c.load(Ordering::SeqCst))
    }

    /// Get gauge value
    pub fn get_gauge(&self, name: &str) -> Option<f64> {
        self.gauges.get(name).copied()
    }

    /// Record query statistics
    pub fn record_query_stats(&mut self, stats: QueryStats) {
        self.query_stats.insert(stats.query_id.clone(), stats);
    }

    /// Get query statistics
    pub fn get_query_stats(&self, query_id: &str) -> Option<&QueryStats> {
        self.query_stats.get(query_id)
    }

    /// Get top N slow queries
    pub fn get_slow_queries(&self, limit: usize) -> Vec<QueryStats> {
        let mut queries: Vec<_> = self.query_stats.values().cloned().collect();
        queries.sort_by(|a, b| b.total_time_ms.cmp(&a.total_time_ms));
        queries.truncate(limit);
        queries
    }

    /// Calculate statistics for a metric type
    pub fn get_metric_statistics(&self, metric_type: MetricType) -> Option<PerformanceStats> {
        let values: Vec<f64> = self
            .metrics
            .iter()
            .filter(|m| m.metric_type == metric_type)
            .map(|m| m.value)
            .collect();

        if values.is_empty() {
            return None;
        }

        let mut sorted_values = values.clone();
        sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let min = sorted_values[0];
        let max = sorted_values[sorted_values.len() - 1];
        let sum: f64 = sorted_values.iter().sum();
        let avg = sum / sorted_values.len() as f64;

        let p95_idx = (sorted_values.len() as f64 * 0.95) as usize;
        let p99_idx = (sorted_values.len() as f64 * 0.99) as usize;

        let percentile_95 = sorted_values.get(p95_idx).copied().unwrap_or(max);
        let percentile_99 = sorted_values.get(p99_idx).copied().unwrap_or(max);

        Some(PerformanceStats {
            min_value: min,
            max_value: max,
            avg_value: avg,
            count: sorted_values.len(),
            percentile_95,
            percentile_99,
        })
    }

    /// Perform health check
    pub fn health_check(&self) -> HealthCheck {
        let mut checks = HashMap::new();
        let mut all_healthy = true;

        // Check memory usage
        let memory_ok = self.get_gauge("memory_usage").map(|m| m < 80.0).unwrap_or(true);
        checks.insert("memory".to_string(), memory_ok);
        if !memory_ok {
            all_healthy = false;
        }

        // Check connection count
        let conn_ok = self.get_counter("connection_count").map(|c| c < 1000).unwrap_or(true);
        checks.insert("connections".to_string(), conn_ok);
        if !conn_ok {
            all_healthy = false;
        }

        // Check query latency
        let query_latency_ok = self
            .get_metric_statistics(MetricType::QueryLatency)
            .map(|s| s.avg_value < 1000.0)
            .unwrap_or(true);
        checks.insert("query_latency".to_string(), query_latency_ok);
        if !query_latency_ok {
            all_healthy = false;
        }

        let status = if all_healthy {
            HealthStatus::Healthy
        } else {
            HealthStatus::Warning
        };

        HealthCheck {
            status,
            checks,
            timestamp: Utc::now(),
            message: format!("Health check completed with status: {:?}", status),
        }
    }

    /// Get metrics summary
    pub fn get_summary(&self) -> String {
        let mut summary = String::new();

        summary.push_str(&format!("Total metrics recorded: {}\n", self.metrics.len()));
        summary.push_str(&format!("Total queries tracked: {}\n", self.query_stats.len()));

        if let Some(stats) = self.get_metric_statistics(MetricType::QueryLatency) {
            summary.push_str(&format!(
                "Query Latency - Avg: {:.2}ms, P95: {:.2}ms, P99: {:.2}ms\n",
                stats.avg_value, stats.percentile_95, stats.percentile_99
            ));
        }

        if let Some(count) = self.get_counter("total_queries") {
            summary.push_str(&format!("Total queries executed: {}\n", count));
        }

        summary
    }

    /// Clear old metrics (older than hours)
    pub fn cleanup_old_metrics(&mut self, hours: i64) {
        let cutoff = Utc::now() - chrono::Duration::hours(hours);
        self.metrics.retain(|m| m.timestamp > cutoff);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_collector() {
        let collector = MetricsCollector::new();
        assert_eq!(collector.metrics.len(), 0);
    }

    #[test]
    fn test_record_metric() {
        let mut collector = MetricsCollector::new();

        let metric = MetricValue {
            metric_type: MetricType::QueryLatency,
            value: 42.5,
            timestamp: Utc::now(),
            dimension: "SELECT".to_string(),
        };

        collector.record_metric(metric);
        assert_eq!(collector.metrics.len(), 1);
    }

    #[test]
    fn test_increment_counter() {
        let mut collector = MetricsCollector::new();

        collector.increment_counter("total_queries", 1);
        collector.increment_counter("total_queries", 1);

        assert_eq!(collector.get_counter("total_queries"), Some(2));
    }

    #[test]
    fn test_set_gauge() {
        let mut collector = MetricsCollector::new();

        collector.set_gauge("memory_usage", 65.5);
        assert_eq!(collector.get_gauge("memory_usage"), Some(65.5));
    }

    #[test]
    fn test_record_query_stats() {
        let mut collector = MetricsCollector::new();

        let stats = QueryStats {
            query_id: "q1".to_string(),
            query_text: "SELECT * FROM users".to_string(),
            execution_count: 5,
            total_time_ms: 500,
            avg_time_ms: 100.0,
            min_time_ms: 80,
            max_time_ms: 150,
            rows_scanned: 1000,
            rows_returned: 100,
        };

        collector.record_query_stats(stats);
        assert!(collector.get_query_stats("q1").is_some());
    }

    #[test]
    fn test_get_slow_queries() {
        let mut collector = MetricsCollector::new();

        for i in 1..=5 {
            let stats = QueryStats {
                query_id: format!("q{}", i),
                query_text: format!("SELECT {}", i),
                execution_count: 1,
                total_time_ms: (i * 100) as u64,
                avg_time_ms: (i * 100) as f64,
                min_time_ms: (i * 100) as u64,
                max_time_ms: (i * 100) as u64,
                rows_scanned: 100,
                rows_returned: 10,
            };

            collector.record_query_stats(stats);
        }

        let slow = collector.get_slow_queries(2);
        assert_eq!(slow.len(), 2);
        assert_eq!(slow[0].total_time_ms, 500); // q5
        assert_eq!(slow[1].total_time_ms, 400); // q4
    }

    #[test]
    fn test_metric_statistics() {
        let mut collector = MetricsCollector::new();

        for i in 1..=10 {
            let metric = MetricValue {
                metric_type: MetricType::QueryLatency,
                value: (i * 10) as f64,
                timestamp: Utc::now(),
                dimension: "SELECT".to_string(),
            };

            collector.record_metric(metric);
        }

        let stats = collector.get_metric_statistics(MetricType::QueryLatency).unwrap();
        assert_eq!(stats.min_value, 10.0);
        assert_eq!(stats.max_value, 100.0);
        assert_eq!(stats.count, 10);
    }

    #[test]
    fn test_health_check() {
        let mut collector = MetricsCollector::new();

        collector.set_gauge("memory_usage", 50.0);
        collector.increment_counter("connection_count", 100);

        let health = collector.health_check();
        assert_eq!(health.status, HealthStatus::Healthy);
    }

    #[test]
    fn test_get_summary() {
        let mut collector = MetricsCollector::new();

        collector.increment_counter("total_queries", 42);
        collector.set_gauge("memory_usage", 65.5);

        let summary = collector.get_summary();
        assert!(summary.contains("Total metrics"));
        assert!(summary.contains("Total queries"));
    }
}
