//! Monitoring & Telemetry - производственная наблюдаемость
/// Query metrics, slow query log, performance monitoring, health checks
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Metric type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Timer,
}

impl MetricType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MetricType::Counter => "Counter",
            MetricType::Gauge => "Gauge",
            MetricType::Histogram => "Histogram",
            MetricType::Timer => "Timer",
        }
    }
}

/// Query execution metrics
#[derive(Debug, Clone)]
pub struct QueryMetrics {
    pub query_id: String,
    pub query_text: String,
    pub start_time: u64,
    pub end_time: u64,
    pub rows_scanned: i64,
    pub rows_returned: i64,
    pub execution_time_ms: u32,
    pub has_index: bool,
    pub full_scan: bool,
}

impl QueryMetrics {
    pub fn new(query_id: String, query_text: String) -> Self {
        Self {
            query_id,
            query_text,
            start_time: Self::current_time_ms(),
            end_time: 0,
            rows_scanned: 0,
            rows_returned: 0,
            execution_time_ms: 0,
            has_index: false,
            full_scan: false,
        }
    }

    pub fn complete(&mut self) {
        self.end_time = Self::current_time_ms();
        self.execution_time_ms = (self.end_time - self.start_time) as u32;
    }

    pub fn is_slow(&self, threshold_ms: u32) -> bool {
        self.execution_time_ms > threshold_ms
    }

    fn current_time_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

/// Slow query logger
#[derive(Debug, Clone)]
pub struct SlowQueryLog {
    pub threshold_ms: u32,
    pub capacity: usize,
    pub queries: Vec<QueryMetrics>,
}

impl SlowQueryLog {
    pub fn new(threshold_ms: u32, capacity: usize) -> Self {
        Self {
            threshold_ms,
            capacity,
            queries: Vec::new(),
        }
    }

    pub fn add_query(&mut self, metrics: QueryMetrics) {
        if metrics.is_slow(self.threshold_ms) {
            if self.queries.len() >= self.capacity {
                self.queries.remove(0);
            }
            self.queries.push(metrics);
        }
    }

    pub fn get_slow_queries(&self) -> &[QueryMetrics] {
        &self.queries
    }

    pub fn get_slowest_query(&self) -> Option<&QueryMetrics> {
        self.queries.iter().max_by_key(|q| q.execution_time_ms)
    }

    pub fn clear(&mut self) {
        self.queries.clear();
    }
}

/// Database health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

impl HealthStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            HealthStatus::Healthy => "Healthy",
            HealthStatus::Degraded => "Degraded",
            HealthStatus::Unhealthy => "Unhealthy",
        }
    }
}

/// Database health check result
#[derive(Debug, Clone)]
pub struct HealthCheck {
    pub timestamp: u64,
    pub status: HealthStatus,
    pub cpu_usage: f64,
    pub memory_usage_mb: u64,
    pub disk_usage_mb: u64,
    pub active_connections: u32,
    pub response_time_ms: u32,
    pub cache_hit_ratio: f64,
}

impl HealthCheck {
    pub fn new() -> Self {
        Self {
            timestamp: Self::current_time_ms(),
            status: HealthStatus::Healthy,
            cpu_usage: 0.0,
            memory_usage_mb: 0,
            disk_usage_mb: 0,
            active_connections: 0,
            response_time_ms: 0,
            cache_hit_ratio: 0.0,
        }
    }

    pub fn update_status(&mut self) {
        // Simple health status determination
        if self.cpu_usage > 90.0 || self.memory_usage_mb > 8000 {
            self.status = HealthStatus::Unhealthy;
        } else if self.cpu_usage > 70.0 || self.memory_usage_mb > 6000 {
            self.status = HealthStatus::Degraded;
        } else {
            self.status = HealthStatus::Healthy;
        }
    }

    fn current_time_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

/// Connection pool statistics
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub total_connections: u32,
    pub active_connections: u32,
    pub idle_connections: u32,
    pub waiting_connections: u32,
    pub peak_connections: u32,
}

impl ConnectionStats {
    pub fn new() -> Self {
        Self {
            total_connections: 0,
            active_connections: 0,
            idle_connections: 0,
            waiting_connections: 0,
            peak_connections: 0,
        }
    }

    pub fn utilization_ratio(&self) -> f64 {
        if self.total_connections == 0 {
            0.0
        } else {
            (self.active_connections as f64) / (self.total_connections as f64)
        }
    }
}

/// Performance counter aggregator
#[derive(Debug, Clone)]
pub struct PerformanceMonitor {
    pub total_queries: u64,
    pub total_errors: u64,
    pub total_rows_processed: i64,
    pub avg_query_time_ms: u32,
    pub slow_query_count: u64,
    pub connection_stats: ConnectionStats,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            total_queries: 0,
            total_errors: 0,
            total_rows_processed: 0,
            avg_query_time_ms: 0,
            slow_query_count: 0,
            connection_stats: ConnectionStats::new(),
        }
    }

    pub fn record_query(&mut self, metrics: &QueryMetrics) {
        self.total_queries += 1;
        self.total_rows_processed += metrics.rows_returned;
        
        if metrics.is_slow(500) {
            self.slow_query_count += 1;
        }
    }

    pub fn record_error(&mut self) {
        self.total_errors += 1;
    }

    pub fn error_rate(&self) -> f64 {
        if self.total_queries == 0 {
            0.0
        } else {
            (self.total_errors as f64) / (self.total_queries as f64)
        }
    }
}

/// Telemetry system
#[derive(Debug)]
pub struct TelemetryCollector {
    metrics: HashMap<String, Vec<QueryMetrics>>,
    slow_queries: SlowQueryLog,
    monitor: PerformanceMonitor,
    health_history: Vec<HealthCheck>,
}

impl TelemetryCollector {
    pub fn new(slow_query_threshold_ms: u32) -> Self {
        Self {
            metrics: HashMap::new(),
            slow_queries: SlowQueryLog::new(slow_query_threshold_ms, 100),
            monitor: PerformanceMonitor::new(),
            health_history: Vec::new(),
        }
    }

    pub fn collect_query_metrics(&mut self, mut metrics: QueryMetrics) {
        metrics.complete();
        
        self.slow_queries.add_query(metrics.clone());
        self.monitor.record_query(&metrics);
        
        self.metrics
            .entry(metrics.query_id.clone())
            .or_insert_with(Vec::new)
            .push(metrics);
    }

    pub fn collect_health_check(&mut self, mut health: HealthCheck) {
        health.update_status();
        self.health_history.push(health);
        
        // Keep last 1000 health checks
        if self.health_history.len() > 1000 {
            self.health_history.remove(0);
        }
    }

    pub fn get_query_metrics(&self, query_id: &str) -> Option<&Vec<QueryMetrics>> {
        self.metrics.get(query_id)
    }

    pub fn get_slow_queries(&self) -> &[QueryMetrics] {
        self.slow_queries.get_slow_queries()
    }

    pub fn get_performance_stats(&self) -> &PerformanceMonitor {
        &self.monitor
    }

    pub fn get_latest_health(&self) -> Option<&HealthCheck> {
        self.health_history.last()
    }

    pub fn get_health_history(&self) -> &[HealthCheck] {
        &self.health_history
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_metrics_creation() {
        let metrics = QueryMetrics::new("q1".to_string(), "SELECT * FROM users".to_string());
        assert_eq!(metrics.query_id, "q1");
        assert_eq!(metrics.execution_time_ms, 0);
    }

    #[test]
    fn test_query_metrics_is_slow() {
        let mut metrics = QueryMetrics::new("q1".to_string(), "SELECT *".to_string());
        metrics.execution_time_ms = 1000;
        assert!(metrics.is_slow(500));
        assert!(!metrics.is_slow(2000));
    }

    #[test]
    fn test_slow_query_log_creation() {
        let log = SlowQueryLog::new(500, 10);
        assert_eq!(log.threshold_ms, 500);
        assert_eq!(log.capacity, 10);
    }

    #[test]
    fn test_slow_query_log_add() {
        let mut log = SlowQueryLog::new(500, 10);
        let mut metrics = QueryMetrics::new("q1".to_string(), "SELECT *".to_string());
        metrics.execution_time_ms = 1000;
        
        log.add_query(metrics);
        assert_eq!(log.get_slow_queries().len(), 1);
    }

    #[test]
    fn test_slow_query_log_capacity() {
        let mut log = SlowQueryLog::new(0, 2);
        
        for i in 0..3 {
            let mut metrics = QueryMetrics::new(format!("q{}", i), "SELECT *".to_string());
            metrics.execution_time_ms = 100;
            log.add_query(metrics);
        }
        
        assert_eq!(log.get_slow_queries().len(), 2);
    }

    #[test]
    fn test_health_check_creation() {
        let health = HealthCheck::new();
        assert_eq!(health.status, HealthStatus::Healthy);
    }

    #[test]
    fn test_health_check_status_update() {
        let mut health = HealthCheck::new();
        health.cpu_usage = 95.0;
        health.update_status();
        assert_eq!(health.status, HealthStatus::Unhealthy);
    }

    #[test]
    fn test_connection_stats() {
        let stats = ConnectionStats::new();
        assert_eq!(stats.total_connections, 0);
        assert_eq!(stats.utilization_ratio(), 0.0);
    }

    #[test]
    fn test_connection_stats_utilization() {
        let mut stats = ConnectionStats::new();
        stats.total_connections = 100;
        stats.active_connections = 50;
        assert!((stats.utilization_ratio() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_performance_monitor_creation() {
        let monitor = PerformanceMonitor::new();
        assert_eq!(monitor.total_queries, 0);
        assert_eq!(monitor.error_rate(), 0.0);
    }

    #[test]
    fn test_performance_monitor_record_query() {
        let mut monitor = PerformanceMonitor::new();
        let metrics = QueryMetrics::new("q1".to_string(), "SELECT *".to_string());
        
        monitor.record_query(&metrics);
        assert_eq!(monitor.total_queries, 1);
    }

    #[test]
    fn test_performance_monitor_error_rate() {
        let mut monitor = PerformanceMonitor::new();
        monitor.total_queries = 100;
        monitor.total_errors = 10;
        assert!((monitor.error_rate() - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_telemetry_collector_creation() {
        let collector = TelemetryCollector::new(500);
        assert_eq!(collector.get_slow_queries().len(), 0);
    }

    #[test]
    fn test_telemetry_collector_collect_metrics() {
        let mut collector = TelemetryCollector::new(500);
        let metrics = QueryMetrics::new("q1".to_string(), "SELECT *".to_string());
        
        collector.collect_query_metrics(metrics);
        assert!(collector.get_query_metrics("q1").is_some());
    }

    #[test]
    fn test_telemetry_collector_health() {
        let mut collector = TelemetryCollector::new(500);
        let health = HealthCheck::new();
        
        collector.collect_health_check(health);
        assert!(collector.get_latest_health().is_some());
    }

    #[test]
    fn test_metric_type_string() {
        assert_eq!(MetricType::Counter.as_str(), "Counter");
        assert_eq!(MetricType::Gauge.as_str(), "Gauge");
        assert_eq!(MetricType::Timer.as_str(), "Timer");
    }

    #[test]
    fn test_health_status_string() {
        assert_eq!(HealthStatus::Healthy.as_str(), "Healthy");
        assert_eq!(HealthStatus::Degraded.as_str(), "Degraded");
        assert_eq!(HealthStatus::Unhealthy.as_str(), "Unhealthy");
    }
}
