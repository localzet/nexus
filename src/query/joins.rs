//! JOIN Operations - реализация различных типов JOIN
/// Supports INNER, LEFT, RIGHT, and FULL JOINs with ON conditions
use crate::types::{Value, Row};
use anyhow::{Result, anyhow};
use std::collections::HashMap;

/// Supported JOIN types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

impl JoinType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "INNER" => Some(JoinType::Inner),
            "LEFT" => Some(JoinType::Left),
            "RIGHT" => Some(JoinType::Right),
            "FULL" => Some(JoinType::Full),
            _ => None,
        }
    }
}

/// JOIN condition comparing two columns
#[derive(Debug, Clone)]
pub struct JoinCondition {
    pub left_table: String,
    pub left_column: String,
    pub right_table: String,
    pub right_column: String,
}

impl JoinCondition {
    /// Parse condition like "users.id = orders.user_id"
    pub fn from_string(condition: &str) -> Result<Self> {
        let parts: Vec<&str> = condition.split('=').map(|s| s.trim()).collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid join condition format"));
        }

        let left_parts: Vec<&str> = parts[0].split('.').collect();
        let right_parts: Vec<&str> = parts[1].split('.').collect();

        if left_parts.len() != 2 || right_parts.len() != 2 {
            return Err(anyhow!("Join condition must use table.column format"));
        }

        Ok(JoinCondition {
            left_table: left_parts[0].to_lowercase(),
            left_column: left_parts[1].to_lowercase(),
            right_table: right_parts[0].to_lowercase(),
            right_column: right_parts[1].to_lowercase(),
        })
    }

    /// Check if row pair matches join condition
    pub fn matches(&self, left_row: &Row, right_row: &Row) -> Result<bool> {
        let left_val = left_row.values.get(&self.left_column)
            .cloned()
            .ok_or_else(|| anyhow!("Column '{}' not found in left table", self.left_column))?;
        
        let right_val = right_row.values.get(&self.right_column)
            .cloned()
            .ok_or_else(|| anyhow!("Column '{}' not found in right table", self.right_column))?;

        Ok(left_val == right_val)
    }
}

/// Executes JOIN operations between two result sets
pub struct JoinExecutor;

impl JoinExecutor {
    /// Execute JOIN between left_rows and right_rows
    pub fn join(
        join_type: JoinType,
        left_rows: Vec<Row>,
        right_rows: Vec<Row>,
        condition: &JoinCondition,
        left_prefix: &str,
        right_prefix: &str,
    ) -> Result<Vec<Row>> {
        match join_type {
            JoinType::Inner => Self::inner_join(left_rows, right_rows, condition, left_prefix, right_prefix),
            JoinType::Left => Self::left_join(left_rows, right_rows, condition, left_prefix, right_prefix),
            JoinType::Right => Self::right_join(left_rows, right_rows, condition, left_prefix, right_prefix),
            JoinType::Full => Self::full_join(left_rows, right_rows, condition, left_prefix, right_prefix),
        }
    }

    /// INNER JOIN: Only matching rows
    fn inner_join(
        left_rows: Vec<Row>,
        right_rows: Vec<Row>,
        condition: &JoinCondition,
        left_prefix: &str,
        right_prefix: &str,
    ) -> Result<Vec<Row>> {
        let mut results = Vec::new();

        for left_row in &left_rows {
            for right_row in &right_rows {
                if condition.matches(left_row, right_row)? {
                    let combined = Self::combine_rows(left_row, right_row, left_prefix, right_prefix);
                    results.push(combined);
                }
            }
        }

        Ok(results)
    }

    /// LEFT JOIN: All left rows + matching right rows
    fn left_join(
        left_rows: Vec<Row>,
        right_rows: Vec<Row>,
        condition: &JoinCondition,
        left_prefix: &str,
        right_prefix: &str,
    ) -> Result<Vec<Row>> {
        let mut results = Vec::new();
        let mut matched_right_indices = Vec::new();

        for (left_idx, left_row) in left_rows.iter().enumerate() {
            let mut found_match = false;

            for (right_idx, right_row) in right_rows.iter().enumerate() {
                if condition.matches(left_row, right_row)? {
                    let combined = Self::combine_rows(left_row, right_row, left_prefix, right_prefix);
                    results.push(combined);
                    matched_right_indices.push(right_idx);
                    found_match = true;
                }
            }

            // If no match found, add left row with NULL right columns
            if !found_match {
                let mut combined = Self::prefix_row(left_row, left_prefix);
                let template = if right_rows.is_empty() {
                    &Row::new(crate::types::RecordId::new())
                } else {
                    &right_rows[0]
                };
                let null_right = Self::create_null_row(template, right_prefix);
                combined.values.extend(null_right.values);
                results.push(combined);
            }
        }

        Ok(results)
    }

    /// RIGHT JOIN: Matching rows + all right rows
    fn right_join(
        left_rows: Vec<Row>,
        right_rows: Vec<Row>,
        condition: &JoinCondition,
        left_prefix: &str,
        right_prefix: &str,
    ) -> Result<Vec<Row>> {
        let mut results = Vec::new();
        let mut matched_right_indices = std::collections::HashSet::new();

        for left_row in &left_rows {
            for (right_idx, right_row) in right_rows.iter().enumerate() {
                if condition.matches(left_row, right_row)? {
                    let combined = Self::combine_rows(left_row, right_row, left_prefix, right_prefix);
                    results.push(combined);
                    matched_right_indices.insert(right_idx);
                }
            }
        }

        // Add unmatched right rows with NULL left columns
        for (right_idx, right_row) in right_rows.iter().enumerate() {
            if !matched_right_indices.contains(&right_idx) {
                let template = if left_rows.is_empty() {
                    &Row::new(crate::types::RecordId::new())
                } else {
                    &left_rows[0]
                };
                let null_left = Self::create_null_row(template, left_prefix);
                let mut combined = null_left;
                combined.values.extend(Self::prefix_row(right_row, right_prefix).values);
                results.push(combined);
            }
        }

        Ok(results)
    }

    /// FULL JOIN: All rows from both sides, with NULLs where no match
    fn full_join(
        left_rows: Vec<Row>,
        right_rows: Vec<Row>,
        condition: &JoinCondition,
        left_prefix: &str,
        right_prefix: &str,
    ) -> Result<Vec<Row>> {
        let mut results = Vec::new();
        let mut matched_left_indices = std::collections::HashSet::new();
        let mut matched_right_indices = std::collections::HashSet::new();

        // Add all matching pairs
        for (left_idx, left_row) in left_rows.iter().enumerate() {
            for (right_idx, right_row) in right_rows.iter().enumerate() {
                if condition.matches(left_row, right_row)? {
                    let combined = Self::combine_rows(left_row, right_row, left_prefix, right_prefix);
                    results.push(combined);
                    matched_left_indices.insert(left_idx);
                    matched_right_indices.insert(right_idx);
                }
            }
        }

        // Add unmatched left rows
        for (left_idx, left_row) in left_rows.iter().enumerate() {
            if !matched_left_indices.contains(&left_idx) {
                let mut combined = Self::prefix_row(left_row, left_prefix);
                let template = if right_rows.is_empty() {
                    &Row::new(crate::types::RecordId::new())
                } else {
                    &right_rows[0]
                };
                let null_right = Self::create_null_row(template, right_prefix);
                combined.values.extend(null_right.values);
                results.push(combined);
            }
        }

        // Add unmatched right rows
        for (right_idx, right_row) in right_rows.iter().enumerate() {
            if !matched_right_indices.contains(&right_idx) {
                let template = if left_rows.is_empty() {
                    &Row::new(crate::types::RecordId::new())
                } else {
                    &left_rows[0]
                };
                let null_left = Self::create_null_row(template, left_prefix);
                let mut combined = null_left;
                combined.values.extend(Self::prefix_row(right_row, right_prefix).values);
                results.push(combined);
            }
        }

        Ok(results)
    }

    /// Combine two rows with column prefixes
    fn combine_rows(left: &Row, right: &Row, left_prefix: &str, right_prefix: &str) -> Row {
        let mut combined = Self::prefix_row(left, left_prefix);
        combined.values.extend(Self::prefix_row(right, right_prefix).values);
        combined
    }

    /// Add prefix to row column names
    fn prefix_row(row: &Row, prefix: &str) -> Row {
        let mut prefixed = Row::new(row.id);
        for (col, val) in &row.values {
            prefixed.insert(format!("{}.{}", prefix, col), val.clone());
        }
        prefixed
    }

    /// Create a row with all NULL values for unmatched joins
    fn create_null_row(template: &Row, prefix: &str) -> Row {
        let mut null_row = Row::new(crate::types::RecordId::new());
        for col in template.values.keys() {
            null_row.insert(format!("{}.{}", prefix, col), Value::Null);
        }
        null_row
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_row(id: i64, name: &str) -> Row {
        let mut row = Row::new(crate::types::RecordId::new());
        row.insert("id".to_string(), Value::Integer(id));
        row.insert("name".to_string(), Value::String(name.to_string()));
        row
    }

    fn create_test_row_with_fk(id: i64, user_id: i64, total: i64) -> Row {
        let mut row = Row::new(crate::types::RecordId::new());
        row.insert("id".to_string(), Value::Integer(id));
        row.insert("user_id".to_string(), Value::Integer(user_id));
        row.insert("total".to_string(), Value::Integer(total));
        row
    }

    #[test]
    fn test_join_type_parsing() {
        assert_eq!(JoinType::from_str("INNER"), Some(JoinType::Inner));
        assert_eq!(JoinType::from_str("LEFT"), Some(JoinType::Left));
        assert_eq!(JoinType::from_str("RIGHT"), Some(JoinType::Right));
        assert_eq!(JoinType::from_str("FULL"), Some(JoinType::Full));
        assert_eq!(JoinType::from_str("INVALID"), None);
    }

    #[test]
    fn test_join_condition_parsing() -> Result<()> {
        let cond = JoinCondition::from_string("users.id = orders.user_id")?;
        assert_eq!(cond.left_table, "users");
        assert_eq!(cond.left_column, "id");
        assert_eq!(cond.right_table, "orders");
        assert_eq!(cond.right_column, "user_id");
        Ok(())
    }

    #[test]
    fn test_join_condition_matching() -> Result<()> {
        let cond = JoinCondition::from_string("users.id = orders.user_id")?;
        let user = create_test_row(1, "Alice");
        let order = create_test_row_with_fk(10, 1, 100);
        
        assert!(cond.matches(&user, &order)?);
        
        let order_no_match = create_test_row_with_fk(10, 2, 100);
        assert!(!cond.matches(&user, &order_no_match)?);
        
        Ok(())
    }

    #[test]
    fn test_inner_join() -> Result<()> {
        let left_rows = vec![
            create_test_row(1, "Alice"),
            create_test_row(2, "Bob"),
        ];
        
        let right_rows = vec![
            create_test_row_with_fk(10, 1, 100),
            create_test_row_with_fk(20, 1, 200),
            create_test_row_with_fk(30, 2, 150),
        ];
        
        let cond = JoinCondition::from_string("users.id = orders.user_id")?;
        let result = JoinExecutor::join(
            JoinType::Inner,
            left_rows,
            right_rows,
            &cond,
            "users",
            "orders"
        )?;
        
        assert_eq!(result.len(), 3);
        Ok(())
    }

    #[test]
    fn test_inner_join_no_matches() -> Result<()> {
        let left_rows = vec![create_test_row(1, "Alice")];
        let right_rows = vec![create_test_row_with_fk(10, 99, 100)];
        
        let cond = JoinCondition::from_string("users.id = orders.user_id")?;
        let result = JoinExecutor::join(
            JoinType::Inner,
            left_rows,
            right_rows,
            &cond,
            "users",
            "orders"
        )?;
        
        assert_eq!(result.len(), 0);
        Ok(())
    }

    #[test]
    fn test_left_join_all_left_preserved() -> Result<()> {
        let left_rows = vec![
            create_test_row(1, "Alice"),
            create_test_row(2, "Bob"),
            create_test_row(3, "Charlie"),
        ];
        
        let right_rows = vec![
            create_test_row_with_fk(10, 1, 100),
            create_test_row_with_fk(20, 2, 200),
        ];
        
        let cond = JoinCondition::from_string("users.id = orders.user_id")?;
        let result = JoinExecutor::join(
            JoinType::Left,
            left_rows,
            right_rows,
            &cond,
            "users",
            "orders"
        )?;
        
        assert_eq!(result.len(), 3);
        
        // Check that all left rows are present
        let ids: Vec<i64> = result.iter()
            .filter_map(|r| {
                if let Some(Value::Integer(id)) = r.values.get("users.id") {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
        
        assert_eq!(ids.len(), 3);
        Ok(())
    }

    #[test]
    fn test_right_join_all_right_preserved() -> Result<()> {
        let left_rows = vec![
            create_test_row(1, "Alice"),
        ];
        
        let right_rows = vec![
            create_test_row_with_fk(10, 1, 100),
            create_test_row_with_fk(20, 2, 200),
            create_test_row_with_fk(30, 3, 300),
        ];
        
        let cond = JoinCondition::from_string("users.id = orders.user_id")?;
        let result = JoinExecutor::join(
            JoinType::Right,
            left_rows,
            right_rows,
            &cond,
            "users",
            "orders"
        )?;
        
        assert_eq!(result.len(), 3);
        
        // Check that all right rows are present
        let order_ids: Vec<i64> = result.iter()
            .filter_map(|r| {
                if let Some(Value::Integer(id)) = r.values.get("orders.id") {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
        
        assert_eq!(order_ids.len(), 3);
        Ok(())
    }

    #[test]
    fn test_full_join_combines_all() -> Result<()> {
        let left_rows = vec![
            create_test_row(1, "Alice"),
            create_test_row(2, "Bob"),
            create_test_row(3, "Charlie"),
        ];
        
        let right_rows = vec![
            create_test_row_with_fk(10, 1, 100),
            create_test_row_with_fk(20, 4, 200),
        ];
        
        let cond = JoinCondition::from_string("users.id = orders.user_id")?;
        let result = JoinExecutor::join(
            JoinType::Full,
            left_rows,
            right_rows,
            &cond,
            "users",
            "orders"
        )?;
        
        // Should have: 1 match, 2 unmatched left, 1 unmatched right = 4 rows
        assert_eq!(result.len(), 4);
        Ok(())
    }

    #[test]
    fn test_full_join_empty_left() -> Result<()> {
        let left_rows = vec![];
        
        let right_rows = vec![
            create_test_row_with_fk(10, 1, 100),
            create_test_row_with_fk(20, 2, 200),
        ];
        
        let cond = JoinCondition::from_string("users.id = orders.user_id")?;
        let result = JoinExecutor::join(
            JoinType::Full,
            left_rows,
            right_rows,
            &cond,
            "users",
            "orders"
        )?;
        
        assert_eq!(result.len(), 2);
        Ok(())
    }

    #[test]
    fn test_full_join_empty_right() -> Result<()> {
        let left_rows = vec![
            create_test_row(1, "Alice"),
            create_test_row(2, "Bob"),
        ];
        
        let right_rows = vec![];
        
        let cond = JoinCondition::from_string("users.id = orders.user_id")?;
        let result = JoinExecutor::join(
            JoinType::Full,
            left_rows,
            right_rows,
            &cond,
            "users",
            "orders"
        )?;
        
        assert_eq!(result.len(), 2);
        Ok(())
    }

    #[test]
    fn test_multiple_matches_expanded() -> Result<()> {
        let left_rows = vec![
            create_test_row(1, "Alice"),
        ];
        
        let right_rows = vec![
            create_test_row_with_fk(10, 1, 100),
            create_test_row_with_fk(11, 1, 200),
            create_test_row_with_fk(12, 1, 300),
        ];
        
        let cond = JoinCondition::from_string("users.id = orders.user_id")?;
        let result = JoinExecutor::join(
            JoinType::Inner,
            left_rows,
            right_rows,
            &cond,
            "users",
            "orders"
        )?;
        
        // One user matches 3 orders = 3 result rows
        assert_eq!(result.len(), 3);
        Ok(())
    }
}
