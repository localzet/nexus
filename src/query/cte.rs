//! Common Table Expressions и Subqueries

use std::collections::HashMap;
use crate::types::{Value, Row, RecordId};

/// CTE definition
#[derive(Debug, Clone)]
pub struct CommonTableExpression {
    pub name: String,
    pub columns: Vec<String>,
    pub query: String, // Simplified: store the query as string
    pub is_recursive: bool,
}

impl CommonTableExpression {
    pub fn new(name: String, columns: Vec<String>, query: String, is_recursive: bool) -> Self {
        Self {
            name,
            columns,
            query,
            is_recursive,
        }
    }

    pub fn get_column_count(&self) -> usize {
        self.columns.len()
    }

    pub fn get_columns(&self) -> &[String] {
        &self.columns
    }
}

/// Subquery type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubqueryType {
    Scalar,      // Returns single value
    Correlated,  // References outer query
    Uncorrelated, // Independent
    Exists,      // EXISTS clause
    In,          // IN clause subquery
}

impl SubqueryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SubqueryType::Scalar => "Scalar",
            SubqueryType::Correlated => "Correlated",
            SubqueryType::Uncorrelated => "Uncorrelated",
            SubqueryType::Exists => "Exists",
            SubqueryType::In => "In",
        }
    }
}

/// Subquery definition
#[derive(Debug, Clone)]
pub struct Subquery {
    pub query: String,
    pub subquery_type: SubqueryType,
    pub correlation_columns: Vec<String>,
}

impl Subquery {
    pub fn new(query: String, subquery_type: SubqueryType) -> Self {
        Self {
            query,
            subquery_type,
            correlation_columns: Vec::new(),
        }
    }

    pub fn with_correlation(mut self, columns: Vec<String>) -> Self {
        self.correlation_columns = columns;
        self
    }

    pub fn is_correlated(&self) -> bool {
        !self.correlation_columns.is_empty()
    }
}

/// CTE result cache
#[derive(Debug, Clone)]
struct CteCacheEntry {
    name: String,
    rows: Vec<Row>,
    columns: Vec<String>,
}

/// CTE processor for managing CTEs
#[derive(Debug, Clone)]
pub struct CteMaterializer {
    ctes: HashMap<String, CommonTableExpression>,
    cache: HashMap<String, Vec<Row>>,
    execution_order: Vec<String>,
}

impl CteMaterializer {
    pub fn new() -> Self {
        Self {
            ctes: HashMap::new(),
            cache: HashMap::new(),
            execution_order: Vec::new(),
        }
    }

    /// Register a CTE
    pub fn register_cte(&mut self, cte: CommonTableExpression) -> anyhow::Result<()> {
        if self.ctes.contains_key(&cte.name) {
            return Err(anyhow::anyhow!("CTE {} already registered", cte.name));
        }
        self.ctes.insert(cte.name.clone(), cte);
        Ok(())
    }

    /// Get registered CTE
    pub fn get_cte(&self, name: &str) -> Option<&CommonTableExpression> {
        self.ctes.get(name)
    }

    /// Add rows to CTE cache
    pub fn cache_results(&mut self, name: String, rows: Vec<Row>) -> anyhow::Result<()> {
        self.cache.insert(name, rows);
        Ok(())
    }

    /// Get cached rows for a CTE
    pub fn get_cached_rows(&self, name: &str) -> Option<&Vec<Row>> {
        self.cache.get(name)
    }

    /// Clear cache for a specific CTE or all CTEs
    pub fn clear_cache(&mut self, cte_name: Option<&str>) {
        if let Some(name) = cte_name {
            self.cache.remove(name);
        } else {
            self.cache.clear();
        }
    }

    /// Get CTE count
    pub fn get_cte_count(&self) -> usize {
        self.ctes.len()
    }

    /// Get cache size
    pub fn get_cache_size(&self) -> usize {
        self.cache.len()
    }

    /// Build execution order for recursive CTEs
    pub fn build_execution_order(&mut self) -> anyhow::Result<Vec<String>> {
        let mut order = Vec::new();
        let mut visited = std::collections::HashSet::new();

        for (name, _) in &self.ctes {
            if !visited.contains(name) {
                self.topological_sort(name, &mut visited, &mut order)?;
            }
        }

        self.execution_order = order.clone();
        Ok(order)
    }

    fn topological_sort(
        &self,
        name: &str,
        visited: &mut std::collections::HashSet<String>,
        order: &mut Vec<String>,
    ) -> anyhow::Result<()> {
        visited.insert(name.to_string());
        
        // In a real implementation, we would parse the CTE query
        // and identify dependencies on other CTEs
        
        order.push(name.to_string());
        Ok(())
    }

    /// List all CTE names
    pub fn list_cte_names(&self) -> Vec<String> {
        self.ctes.keys().cloned().collect()
    }
}

/// Subquery processor
#[derive(Debug, Clone)]
pub struct SubqueryExecutor {
    subqueries: HashMap<String, Subquery>,
}

impl SubqueryExecutor {
    pub fn new() -> Self {
        Self {
            subqueries: HashMap::new(),
        }
    }

    /// Add a subquery
    pub fn add_subquery(&mut self, id: String, subquery: Subquery) -> anyhow::Result<()> {
        if self.subqueries.contains_key(&id) {
            return Err(anyhow::anyhow!("Subquery {} already exists", id));
        }
        self.subqueries.insert(id, subquery);
        Ok(())
    }

    /// Get a subquery
    pub fn get_subquery(&self, id: &str) -> Option<&Subquery> {
        self.subqueries.get(id)
    }

    /// Evaluate EXISTS subquery (returns true if has rows)
    pub fn eval_exists(&self, rows: &[Row]) -> bool {
        !rows.is_empty()
    }

    /// Evaluate IN subquery (returns set of values)
    pub fn eval_in(&self, rows: &[Row], column: &str) -> anyhow::Result<Vec<Value>> {
        let mut values = Vec::new();
        for row in rows {
            if let Some(val) = row.values.get(column) {
                values.push(val.clone());
            }
        }
        Ok(values)
    }

    /// Evaluate scalar subquery (returns single value or null)
    pub fn eval_scalar(&self, rows: &[Row], column: &str) -> anyhow::Result<Value> {
        if rows.is_empty() {
            return Ok(Value::Null);
        }
        
        if rows.len() > 1 {
            return Err(anyhow::anyhow!("Scalar subquery returned more than one row"));
        }

        Ok(rows[0]
            .values
            .get(column)
            .cloned()
            .unwrap_or(Value::Null))
    }

    /// Get subquery count
    pub fn get_subquery_count(&self) -> usize {
        self.subqueries.len()
    }

    /// List all subquery IDs
    pub fn list_subquery_ids(&self) -> Vec<String> {
        self.subqueries.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cte_creation() {
        let cte = CommonTableExpression::new(
            "sales_data".to_string(),
            vec!["id".to_string(), "amount".to_string()],
            "SELECT id, amount FROM sales".to_string(),
            false,
        );

        assert_eq!(cte.name, "sales_data");
        assert_eq!(cte.get_column_count(), 2);
        assert!(!cte.is_recursive);
    }

    #[test]
    fn test_cte_columns() {
        let cte = CommonTableExpression::new(
            "test".to_string(),
            vec!["col1".to_string(), "col2".to_string(), "col3".to_string()],
            "SELECT 1, 2, 3".to_string(),
            false,
        );

        assert_eq!(cte.get_column_count(), 3);
        assert_eq!(cte.get_columns().len(), 3);
    }

    #[test]
    fn test_subquery_type_string() {
        assert_eq!(SubqueryType::Scalar.as_str(), "Scalar");
        assert_eq!(SubqueryType::Correlated.as_str(), "Correlated");
        assert_eq!(SubqueryType::Exists.as_str(), "Exists");
    }

    #[test]
    fn test_subquery_creation() {
        let subquery = Subquery::new(
            "SELECT * FROM employees WHERE salary > 50000".to_string(),
            SubqueryType::Uncorrelated,
        );

        assert_eq!(subquery.subquery_type, SubqueryType::Uncorrelated);
        assert!(!subquery.is_correlated());
    }

    #[test]
    fn test_subquery_correlation() {
        let subquery = Subquery::new(
            "SELECT COUNT(*) FROM orders WHERE customer_id = outer.id".to_string(),
            SubqueryType::Correlated,
        )
        .with_correlation(vec!["customer_id".to_string()]);

        assert!(subquery.is_correlated());
        assert_eq!(subquery.correlation_columns.len(), 1);
    }

    #[test]
    fn test_cte_materializer_creation() {
        let materializer = CteMaterializer::new();
        assert_eq!(materializer.get_cte_count(), 0);
        assert_eq!(materializer.get_cache_size(), 0);
    }

    #[test]
    fn test_cte_materializer_register() -> anyhow::Result<()> {
        let mut materializer = CteMaterializer::new();
        let cte = CommonTableExpression::new(
            "users".to_string(),
            vec!["id".to_string(), "name".to_string()],
            "SELECT id, name FROM users".to_string(),
            false,
        );

        materializer.register_cte(cte)?;
        assert_eq!(materializer.get_cte_count(), 1);
        Ok(())
    }

    #[test]
    fn test_cte_materializer_get_cte() -> anyhow::Result<()> {
        let mut materializer = CteMaterializer::new();
        let cte = CommonTableExpression::new(
            "test_cte".to_string(),
            vec!["col1".to_string()],
            "SELECT 1".to_string(),
            false,
        );

        materializer.register_cte(cte.clone())?;
        
        let retrieved = materializer.get_cte("test_cte");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test_cte");
        Ok(())
    }

    #[test]
    fn test_cte_materializer_duplicate_error() -> anyhow::Result<()> {
        let mut materializer = CteMaterializer::new();
        let cte1 = CommonTableExpression::new(
            "dup_cte".to_string(),
            vec!["id".to_string()],
            "SELECT 1".to_string(),
            false,
        );
        let cte2 = CommonTableExpression::new(
            "dup_cte".to_string(),
            vec!["id".to_string()],
            "SELECT 2".to_string(),
            false,
        );

        materializer.register_cte(cte1)?;
        assert!(materializer.register_cte(cte2).is_err());
        Ok(())
    }

    #[test]
    fn test_cte_materializer_cache() -> anyhow::Result<()> {
        let mut materializer = CteMaterializer::new();
        
        let mut row = Row::new(RecordId::new());
        row.insert("id".to_string(), Value::Integer(1));
        
        materializer.cache_results("test".to_string(), vec![row])?;
        assert_eq!(materializer.get_cache_size(), 1);
        assert!(materializer.get_cached_rows("test").is_some());
        Ok(())
    }

    #[test]
    fn test_cte_materializer_clear_cache() -> anyhow::Result<()> {
        let mut materializer = CteMaterializer::new();
        
        let mut row = Row::new(RecordId::new());
        row.insert("id".to_string(), Value::Integer(1));
        
        materializer.cache_results("test1".to_string(), vec![row.clone()])?;
        materializer.cache_results("test2".to_string(), vec![row])?;
        
        materializer.clear_cache(Some("test1"));
        assert_eq!(materializer.get_cache_size(), 1);
        
        materializer.clear_cache(None);
        assert_eq!(materializer.get_cache_size(), 0);
        Ok(())
    }

    #[test]
    fn test_cte_list_names() -> anyhow::Result<()> {
        let mut materializer = CteMaterializer::new();
        
        for i in 0..3 {
            let cte = CommonTableExpression::new(
                format!("cte_{}", i),
                vec!["col".to_string()],
                "SELECT 1".to_string(),
                false,
            );
            materializer.register_cte(cte)?;
        }
        
        let names = materializer.list_cte_names();
        assert_eq!(names.len(), 3);
        Ok(())
    }

    #[test]
    fn test_subquery_executor_creation() {
        let executor = SubqueryExecutor::new();
        assert_eq!(executor.get_subquery_count(), 0);
    }

    #[test]
    fn test_subquery_executor_add() -> anyhow::Result<()> {
        let mut executor = SubqueryExecutor::new();
        let subquery = Subquery::new(
            "SELECT * FROM t".to_string(),
            SubqueryType::Uncorrelated,
        );

        executor.add_subquery("sq1".to_string(), subquery)?;
        assert_eq!(executor.get_subquery_count(), 1);
        Ok(())
    }

    #[test]
    fn test_subquery_executor_exists() -> anyhow::Result<()> {
        let executor = SubqueryExecutor::new();
        
        // Non-empty result
        let mut row = Row::new(RecordId::new());
        row.insert("id".to_string(), Value::Integer(1));
        assert!(executor.eval_exists(&vec![row]));
        
        // Empty result
        assert!(!executor.eval_exists(&vec![]));
        Ok(())
    }

    #[test]
    fn test_subquery_executor_scalar() -> anyhow::Result<()> {
        let executor = SubqueryExecutor::new();
        
        let mut row = Row::new(RecordId::new());
        row.insert("value".to_string(), Value::Integer(42));
        
        let result = executor.eval_scalar(&vec![row], "value")?;
        match result {
            Value::Integer(n) => assert_eq!(n, 42),
            _ => panic!("Expected integer"),
        }
        Ok(())
    }

    #[test]
    fn test_subquery_executor_scalar_empty() -> anyhow::Result<()> {
        let executor = SubqueryExecutor::new();
        let result = executor.eval_scalar(&vec![], "value")?;
        assert_eq!(result, Value::Null);
        Ok(())
    }

    #[test]
    fn test_subquery_executor_in() -> anyhow::Result<()> {
        let executor = SubqueryExecutor::new();
        
        let mut rows = Vec::new();
        for i in 1..=3 {
            let mut row = Row::new(RecordId::new());
            row.insert("id".to_string(), Value::Integer(i));
            rows.push(row);
        }
        
        let result = executor.eval_in(&rows, "id")?;
        assert_eq!(result.len(), 3);
        Ok(())
    }

    #[test]
    fn test_subquery_executor_list_ids() -> anyhow::Result<()> {
        let mut executor = SubqueryExecutor::new();
        
        for i in 0..3 {
            let subquery = Subquery::new(
                format!("SELECT {}", i),
                SubqueryType::Scalar,
            );
            executor.add_subquery(format!("sq_{}", i), subquery)?;
        }
        
        let ids = executor.list_subquery_ids();
        assert_eq!(ids.len(), 3);
        Ok(())
    }
}
