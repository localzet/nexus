//! Window функции для аналитических запросов

use crate::types::{Value, Row};
use std::collections::HashMap;
use std::cmp::Ordering;

/// Window frame specification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameType {
    Rows,
    Range,
}

/// Window frame boundary
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameBound {
    UnboundedPreceding,
    Preceding(i64),
    CurrentRow,
    Following(i64),
    UnboundedFollowing,
}

/// Window frame definition
#[derive(Debug, Clone)]
pub struct WindowFrame {
    pub frame_type: FrameType,
    pub start: FrameBound,
    pub end: FrameBound,
}

impl WindowFrame {
    pub fn new(frame_type: FrameType, start: FrameBound, end: FrameBound) -> Self {
        Self {
            frame_type,
            start,
            end,
        }
    }

    pub fn default_range() -> Self {
        Self {
            frame_type: FrameType::Range,
            start: FrameBound::UnboundedPreceding,
            end: FrameBound::CurrentRow,
        }
    }
}

/// Window function type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowFunctionType {
    RowNumber,
    Rank,
    DenseRank,
    Lag,
    Lead,
    FirstValue,
    LastValue,
    Count,
    Sum,
    Avg,
}

impl WindowFunctionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            WindowFunctionType::RowNumber => "ROW_NUMBER",
            WindowFunctionType::Rank => "RANK",
            WindowFunctionType::DenseRank => "DENSE_RANK",
            WindowFunctionType::Lag => "LAG",
            WindowFunctionType::Lead => "LEAD",
            WindowFunctionType::FirstValue => "FIRST_VALUE",
            WindowFunctionType::LastValue => "LAST_VALUE",
            WindowFunctionType::Count => "COUNT",
            WindowFunctionType::Sum => "SUM",
            WindowFunctionType::Avg => "AVG",
        }
    }
}

/// Partition index entry
#[derive(Debug, Clone)]
struct PartitionEntry {
    rows: Vec<Row>,
    indices: Vec<usize>,
}

/// Window specification
#[derive(Debug, Clone)]
pub struct WindowSpec {
    pub partition_by: Vec<String>,
    pub order_by: Vec<(String, bool)>, // (column, is_asc)
    pub frame: WindowFrame,
}

impl WindowSpec {
    pub fn new(partition_by: Vec<String>, order_by: Vec<(String, bool)>) -> Self {
        Self {
            partition_by,
            order_by,
            frame: WindowFrame::default_range(),
        }
    }

    pub fn with_frame(mut self, frame: WindowFrame) -> Self {
        self.frame = frame;
        self
    }
}

/// Window function context
#[derive(Debug, Clone)]
pub struct WindowFunctionContext {
    pub function: WindowFunctionType,
    pub target_column: Option<String>,
    pub window_spec: WindowSpec,
    pub offset: Option<i64>,
}

impl WindowFunctionContext {
    pub fn new(
        function: WindowFunctionType,
        target_column: Option<String>,
        window_spec: WindowSpec,
    ) -> Self {
        Self {
            function,
            target_column,
            window_spec,
            offset: None,
        }
    }

    pub fn with_offset(mut self, offset: i64) -> Self {
        self.offset = Some(offset);
        self
    }
}

/// Window function processor
#[derive(Debug, Clone)]
pub struct WindowFunctionProcessor {
    rows: Vec<Row>,
    partitions: HashMap<String, PartitionEntry>,
}

impl WindowFunctionProcessor {
    pub fn new(rows: Vec<Row>) -> Self {
        Self {
            rows,
            partitions: HashMap::new(),
        }
    }

    /// Build partitions based on the window spec
    fn build_partitions(&mut self, partition_by: &[String]) -> anyhow::Result<()> {
        self.partitions.clear();

        for (idx, row) in self.rows.iter().enumerate() {
            let partition_key = if partition_by.is_empty() {
                "__default__".to_string()
            } else {
                let keys: Vec<String> = partition_by
                    .iter()
                    .filter_map(|col| {
                        row.values
                            .get(col)
                            .map(|v| format!("{:?}", v))
                    })
                    .collect();
                keys.join("|")
            };

            self.partitions
                .entry(partition_key)
                .or_insert_with(|| PartitionEntry {
                    rows: Vec::new(),
                    indices: Vec::new(),
                })
                .indices
                .push(idx);
        }

        // Copy rows into partition entries
        for entry in self.partitions.values_mut() {
            for &idx in &entry.indices {
                if idx < self.rows.len() {
                    entry.rows.push(self.rows[idx].clone());
                }
            }
        }

        Ok(())
    }

    /// Sort partition by order by columns
    fn sort_partition(rows: &mut Vec<Row>, order_by: &[(String, bool)]) {
        rows.sort_by(|a, b| {
            for (col, is_asc) in order_by {
                let val_a = a.values.get(col).cloned();
                let val_b = b.values.get(col).cloned();

                let cmp = match (val_a, val_b) {
                    (Some(Value::Integer(x)), Some(Value::Integer(y))) => x.cmp(&y),
                    (Some(Value::String(x)), Some(Value::String(y))) => x.cmp(&y),
                    (Some(Value::Integer(x)), Some(Value::String(y))) => {
                        x.to_string().cmp(&y)
                    }
                    (Some(Value::String(x)), Some(Value::Integer(y))) => {
                        x.cmp(&y.to_string())
                    }
                    _ => Ordering::Equal,
                };

                let cmp = if *is_asc { cmp } else { cmp.reverse() };
                if cmp != Ordering::Equal {
                    return cmp;
                }
            }
            Ordering::Equal
        });
    }

    /// Apply ROW_NUMBER function
    fn apply_row_number(&self, partition_rows: &[Row]) -> Vec<Value> {
        (1..=partition_rows.len())
            .map(|n| Value::Integer(n as i64))
            .collect()
    }

    /// Apply RANK function
    fn apply_rank(&self, partition_rows: &[Row], order_by: &[(String, bool)]) -> Vec<Value> {
        let mut ranks = vec![Value::Integer(1)];
        let mut current_rank = 1i64;

        for i in 1..partition_rows.len() {
            let mut same_as_prev = true;

            for (col, _) in order_by {
                let val_cur = &partition_rows[i].values.get(col).cloned();
                let val_prev = &partition_rows[i - 1].values.get(col).cloned();

                if val_cur != val_prev {
                    same_as_prev = false;
                    break;
                }
            }

            if same_as_prev {
                ranks.push(Value::Integer(current_rank));
            } else {
                current_rank = (i + 1) as i64;
                ranks.push(Value::Integer(current_rank));
            }
        }

        ranks
    }

    /// Apply DENSE_RANK function
    fn apply_dense_rank(&self, partition_rows: &[Row], order_by: &[(String, bool)]) -> Vec<Value> {
        let mut ranks = vec![Value::Integer(1)];
        let mut current_rank = 1i64;

        for i in 1..partition_rows.len() {
            let mut same_as_prev = true;

            for (col, _) in order_by {
                let val_cur = &partition_rows[i].values.get(col).cloned();
                let val_prev = &partition_rows[i - 1].values.get(col).cloned();

                if val_cur != val_prev {
                    same_as_prev = false;
                    break;
                }
            }

            if same_as_prev {
                ranks.push(Value::Integer(current_rank));
            } else {
                current_rank += 1;
                ranks.push(Value::Integer(current_rank));
            }
        }

        ranks
    }

    /// Apply LAG function
    fn apply_lag(
        &self,
        partition_rows: &[Row],
        col: &str,
        offset: i64,
    ) -> Vec<Value> {
        let offset = offset as usize;
        let mut results = Vec::new();

        for i in 0..partition_rows.len() {
            if i < offset {
                results.push(Value::Null);
            } else {
                results.push(
                    partition_rows[i - offset]
                        .values
                        .get(col)
                        .cloned()
                        .unwrap_or(Value::Null),
                );
            }
        }

        results
    }

    /// Apply LEAD function
    fn apply_lead(
        &self,
        partition_rows: &[Row],
        col: &str,
        offset: i64,
    ) -> Vec<Value> {
        let offset = offset as usize;
        let mut results = Vec::new();

        for i in 0..partition_rows.len() {
            if i + offset >= partition_rows.len() {
                results.push(Value::Null);
            } else {
                results.push(
                    partition_rows[i + offset]
                        .values
                        .get(col)
                        .cloned()
                        .unwrap_or(Value::Null),
                );
            }
        }

        results
    }

    /// Apply FIRST_VALUE function
    fn apply_first_value(&self, partition_rows: &[Row], col: &str) -> Vec<Value> {
        let first_val = partition_rows
            .first()
            .and_then(|r| r.values.get(col).cloned())
            .unwrap_or(Value::Null);

        vec![first_val; partition_rows.len()]
    }

    /// Apply LAST_VALUE function
    fn apply_last_value(&self, partition_rows: &[Row], col: &str) -> Vec<Value> {
        let last_val = partition_rows
            .last()
            .and_then(|r| r.values.get(col).cloned())
            .unwrap_or(Value::Null);

        vec![last_val; partition_rows.len()]
    }

    /// Process window function
    pub fn process(
        &mut self,
        context: &WindowFunctionContext,
    ) -> anyhow::Result<Vec<Value>> {
        self.build_partitions(&context.window_spec.partition_by)?;

        let mut result = Vec::new();

        for (_, entry) in &self.partitions {
            let mut partition_rows = entry.rows.clone();

            // Sort partition
            if !context.window_spec.order_by.is_empty() {
                Self::sort_partition(&mut partition_rows, &context.window_spec.order_by);
            }

            // Apply function
            let values = match context.function {
                WindowFunctionType::RowNumber => self.apply_row_number(&partition_rows),
                WindowFunctionType::Rank => {
                    self.apply_rank(&partition_rows, &context.window_spec.order_by)
                }
                WindowFunctionType::DenseRank => {
                    self.apply_dense_rank(&partition_rows, &context.window_spec.order_by)
                }
                WindowFunctionType::Lag => {
                    let col = context
                        .target_column
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("LAG requires a target column"))?;
                    let offset = context.offset.unwrap_or(1);
                    self.apply_lag(&partition_rows, col, offset)
                }
                WindowFunctionType::Lead => {
                    let col = context
                        .target_column
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("LEAD requires a target column"))?;
                    let offset = context.offset.unwrap_or(1);
                    self.apply_lead(&partition_rows, col, offset)
                }
                WindowFunctionType::FirstValue => {
                    let col = context
                        .target_column
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("FIRST_VALUE requires a target column"))?;
                    self.apply_first_value(&partition_rows, col)
                }
                WindowFunctionType::LastValue => {
                    let col = context
                        .target_column
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("LAST_VALUE requires a target column"))?;
                    self.apply_last_value(&partition_rows, col)
                }
                _ => Vec::new(),
            };

            result.extend(values);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RecordId;

    fn create_test_rows() -> Vec<Row> {
        let mut rows = Vec::new();

        // Create test data: department and salary
        let depts = vec!["Sales", "Sales", "IT", "IT", "HR"];
        let salaries = vec![50000, 60000, 70000, 80000, 55000];
        let names = vec!["Alice", "Bob", "Charlie", "David", "Eve"];

        for i in 0..5 {
            let mut row = Row::new(RecordId::new());
            row.insert("dept".to_string(), Value::String(depts[i].to_string()));
            row.insert("salary".to_string(), Value::Integer(salaries[i]));
            row.insert("name".to_string(), Value::String(names[i].to_string()));
            rows.push(row);
        }

        rows
    }

    #[test]
    fn test_window_frame_creation() {
        let frame = WindowFrame::new(
            FrameType::Rows,
            FrameBound::UnboundedPreceding,
            FrameBound::CurrentRow,
        );
        assert_eq!(frame.frame_type, FrameType::Rows);
    }

    #[test]
    fn test_window_frame_default() {
        let frame = WindowFrame::default_range();
        assert_eq!(frame.frame_type, FrameType::Range);
    }

    #[test]
    fn test_window_spec_creation() {
        let spec = WindowSpec::new(vec!["dept".to_string()], vec![("salary".to_string(), true)]);
        assert_eq!(spec.partition_by.len(), 1);
        assert_eq!(spec.order_by.len(), 1);
    }

    #[test]
    fn test_window_function_type_string() {
        assert_eq!(WindowFunctionType::RowNumber.as_str(), "ROW_NUMBER");
        assert_eq!(WindowFunctionType::Rank.as_str(), "RANK");
        assert_eq!(WindowFunctionType::Lead.as_str(), "LEAD");
    }

    #[test]
    fn test_window_function_context_creation() {
        let spec = WindowSpec::new(vec![], vec![]);
        let ctx = WindowFunctionContext::new(WindowFunctionType::RowNumber, None, spec);
        assert_eq!(ctx.function, WindowFunctionType::RowNumber);
    }

    #[test]
    fn test_row_number_function() -> anyhow::Result<()> {
        let rows = create_test_rows();
        let mut processor = WindowFunctionProcessor::new(rows);

        let spec = WindowSpec::new(vec!["dept".to_string()], vec![]);
        let ctx = WindowFunctionContext::new(WindowFunctionType::RowNumber, None, spec);

        let result = processor.process(&ctx)?;
        assert_eq!(result.len(), 5);

        Ok(())
    }

    #[test]
    fn test_rank_function() -> anyhow::Result<()> {
        let rows = create_test_rows();
        let mut processor = WindowFunctionProcessor::new(rows);

        let spec = WindowSpec::new(
            vec!["dept".to_string()],
            vec![("salary".to_string(), true)],
        );
        let ctx = WindowFunctionContext::new(WindowFunctionType::Rank, None, spec);

        let result = processor.process(&ctx)?;
        assert_eq!(result.len(), 5);

        Ok(())
    }

    #[test]
    fn test_dense_rank_function() -> anyhow::Result<()> {
        let rows = create_test_rows();
        let mut processor = WindowFunctionProcessor::new(rows);

        let spec = WindowSpec::new(
            vec!["dept".to_string()],
            vec![("salary".to_string(), true)],
        );
        let ctx = WindowFunctionContext::new(WindowFunctionType::DenseRank, None, spec);

        let result = processor.process(&ctx)?;
        assert_eq!(result.len(), 5);

        Ok(())
    }

    #[test]
    fn test_lag_function() -> anyhow::Result<()> {
        let rows = create_test_rows();
        let mut processor = WindowFunctionProcessor::new(rows);

        let spec = WindowSpec::new(
            vec!["dept".to_string()],
            vec![("salary".to_string(), true)],
        );
        let ctx = WindowFunctionContext::new(
            WindowFunctionType::Lag,
            Some("salary".to_string()),
            spec,
        )
        .with_offset(1);

        let result = processor.process(&ctx)?;
        assert_eq!(result.len(), 5);

        Ok(())
    }

    #[test]
    fn test_lead_function() -> anyhow::Result<()> {
        let rows = create_test_rows();
        let mut processor = WindowFunctionProcessor::new(rows);

        let spec = WindowSpec::new(
            vec!["dept".to_string()],
            vec![("salary".to_string(), true)],
        );
        let ctx = WindowFunctionContext::new(
            WindowFunctionType::Lead,
            Some("salary".to_string()),
            spec,
        )
        .with_offset(1);

        let result = processor.process(&ctx)?;
        assert_eq!(result.len(), 5);

        Ok(())
    }

    #[test]
    fn test_first_value_function() -> anyhow::Result<()> {
        let rows = create_test_rows();
        let mut processor = WindowFunctionProcessor::new(rows);

        let spec = WindowSpec::new(
            vec!["dept".to_string()],
            vec![("salary".to_string(), true)],
        );
        let ctx = WindowFunctionContext::new(
            WindowFunctionType::FirstValue,
            Some("salary".to_string()),
            spec,
        );

        let result = processor.process(&ctx)?;
        assert_eq!(result.len(), 5);

        Ok(())
    }

    #[test]
    fn test_last_value_function() -> anyhow::Result<()> {
        let rows = create_test_rows();
        let mut processor = WindowFunctionProcessor::new(rows);

        let spec = WindowSpec::new(
            vec!["dept".to_string()],
            vec![("salary".to_string(), true)],
        );
        let ctx = WindowFunctionContext::new(
            WindowFunctionType::LastValue,
            Some("salary".to_string()),
            spec,
        );

        let result = processor.process(&ctx)?;
        assert_eq!(result.len(), 5);

        Ok(())
    }

    #[test]
    fn test_window_processor_no_partition() -> anyhow::Result<()> {
        let rows = create_test_rows();
        let mut processor = WindowFunctionProcessor::new(rows);

        let spec = WindowSpec::new(vec![], vec![]);
        let ctx = WindowFunctionContext::new(WindowFunctionType::RowNumber, None, spec);

        let result = processor.process(&ctx)?;
        assert_eq!(result.len(), 5);

        Ok(())
    }

    #[test]
    fn test_lag_with_offset() -> anyhow::Result<()> {
        let rows = create_test_rows();
        let mut processor = WindowFunctionProcessor::new(rows);

        let spec = WindowSpec::new(
            vec!["dept".to_string()],
            vec![("salary".to_string(), true)],
        );
        let ctx = WindowFunctionContext::new(
            WindowFunctionType::Lag,
            Some("salary".to_string()),
            spec,
        )
        .with_offset(2);

        let result = processor.process(&ctx)?;
        assert_eq!(result.len(), 5);

        Ok(())
    }

    #[test]
    fn test_window_frame_bound_types() {
        assert_eq!(FrameBound::CurrentRow, FrameBound::CurrentRow);
        assert_eq!(FrameBound::UnboundedPreceding, FrameBound::UnboundedPreceding);
    }
}
