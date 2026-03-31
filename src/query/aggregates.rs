/// Aggregate functions and GROUP BY support
use crate::types::{Value, Row};
use anyhow::{Result, anyhow};
use std::collections::HashMap;

/// Supported aggregate functions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AggregateFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

impl AggregateFunction {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "COUNT" => Some(AggregateFunction::Count),
            "SUM" => Some(AggregateFunction::Sum),
            "AVG" => Some(AggregateFunction::Avg),
            "MIN" => Some(AggregateFunction::Min),
            "MAX" => Some(AggregateFunction::Max),
            _ => None,
        }
    }
}

/// Accumulates values for aggregate computation
#[derive(Debug, Clone)]
pub struct AggregateAccumulator {
    function: AggregateFunction,
    values: Vec<Value>,
}

impl AggregateAccumulator {
    /// Create new accumulator for a function
    pub fn new(function: AggregateFunction) -> Self {
        AggregateAccumulator {
            function,
            values: Vec::new(),
        }
    }

    /// Add a value to accumulate
    pub fn add(&mut self, value: Value) {
        self.values.push(value);
    }

    /// Compute final aggregate result
    pub fn compute(&self) -> Result<Value> {
        match self.function {
            AggregateFunction::Count => {
                Ok(Value::Integer(self.values.len() as i64))
            }
            AggregateFunction::Sum => self.compute_sum(),
            AggregateFunction::Avg => self.compute_avg(),
            AggregateFunction::Min => self.compute_min(),
            AggregateFunction::Max => self.compute_max(),
        }
    }

    fn compute_sum(&self) -> Result<Value> {
        if self.values.is_empty() {
            return Ok(Value::Null);
        }

        let mut sum_int: i64 = 0;
        let mut sum_float: f64 = 0.0;
        let mut is_float = false;

        for val in &self.values {
            match val {
                Value::Integer(i) => {
                    if is_float {
                        sum_float += *i as f64;
                    } else {
                        sum_int += i;
                    }
                }
                Value::Float(f) => {
                    if !is_float {
                        sum_float = sum_int as f64;
                        is_float = true;
                    }
                    sum_float += f;
                }
                _ => {}
            }
        }

        Ok(if is_float {
            Value::Float(sum_float)
        } else {
            Value::Integer(sum_int)
        })
    }

    fn compute_avg(&self) -> Result<Value> {
        if self.values.is_empty() {
            return Ok(Value::Null);
        }

        let mut sum: f64 = 0.0;
        let mut count = 0;

        for val in &self.values {
            match val {
                Value::Integer(i) => {
                    sum += *i as f64;
                    count += 1;
                }
                Value::Float(f) => {
                    sum += f;
                    count += 1;
                }
                _ => {}
            }
        }

        if count == 0 {
            Ok(Value::Null)
        } else {
            Ok(Value::Float(sum / count as f64))
        }
    }

    fn compute_min(&self) -> Result<Value> {
        if self.values.is_empty() {
            return Ok(Value::Null);
        }

        let mut min_val = &self.values[0];

        for val in &self.values[1..] {
            match (min_val, val) {
                (Value::Integer(a), Value::Integer(b)) => {
                    if b < a {
                        min_val = val;
                    }
                }
                (Value::Float(a), Value::Float(b)) => {
                    if b < a {
                        min_val = val;
                    }
                }
                (Value::String(a), Value::String(b)) => {
                    if b < a {
                        min_val = val;
                    }
                }
                _ => {}
            }
        }

        Ok(min_val.clone())
    }

    fn compute_max(&self) -> Result<Value> {
        if self.values.is_empty() {
            return Ok(Value::Null);
        }

        let mut max_val = &self.values[0];

        for val in &self.values[1..] {
            match (max_val, val) {
                (Value::Integer(a), Value::Integer(b)) => {
                    if b > a {
                        max_val = val;
                    }
                }
                (Value::Float(a), Value::Float(b)) => {
                    if b > a {
                        max_val = val;
                    }
                }
                (Value::String(a), Value::String(b)) => {
                    if b > a {
                        max_val = val;
                    }
                }
                _ => {}
            }
        }

        Ok(max_val.clone())
    }
}

/// Manages GROUP BY aggregation
#[derive(Debug)]
pub struct GroupByProcessor {
    group_keys: Vec<String>,
    groups: Vec<(Vec<Value>, HashMap<String, AggregateAccumulator>)>,
}

impl GroupByProcessor {
    /// Create processor for grouping by specified columns
    pub fn new(group_keys: Vec<String>) -> Self {
        GroupByProcessor {
            group_keys,
            groups: Vec::new(),
        }
    }

    /// Extract group key from row
    pub fn extract_group_key(&self, row: &Row) -> Result<Vec<Value>> {
        let mut key = Vec::new();
        for col in &self.group_keys {
            let val = row.values.get(col)
                .cloned()
                .ok_or_else(|| anyhow!("Column '{}' not found for grouping", col))?;
            key.push(val);
        }
        Ok(key)
    }

    /// Add row to group with aggregates
    pub fn add_row(&mut self, row: &Row, aggregates: Vec<(String, AggregateFunction, String)>) -> Result<()> {
        let key = self.extract_group_key(row)?;

        // Find or create group
        let mut found = false;
        for (group_key, group_aggs) in &mut self.groups {
            if group_key == &key {
                // Add to existing group
                for (agg_name, agg_func, column_name) in &aggregates {
                    let accumulator = group_aggs.entry(agg_name.clone()).or_insert_with(|| AggregateAccumulator::new(*agg_func));
                    
                    if let Some(val) = row.values.get(column_name) {
                        accumulator.add(val.clone());
                    }
                }
                found = true;
                break;
            }
        }

        // Create new group if not found
        if !found {
            let mut group_aggs = HashMap::new();
            for (agg_name, agg_func, column_name) in aggregates {
                let mut accumulator = AggregateAccumulator::new(agg_func);
                if let Some(val) = row.values.get(&column_name) {
                    accumulator.add(val.clone());
                }
                group_aggs.insert(agg_name, accumulator);
            }
            self.groups.push((key, group_aggs));
        }

        Ok(())
    }

    /// Get aggregated result rows
    pub fn get_results(&self) -> Result<Vec<Row>> {
        let mut results = Vec::new();

        for (group_key, aggregates) in &self.groups {
            let mut result_row = Row::new(crate::types::RecordId::new());

            // Add group key columns
            for (i, col_name) in self.group_keys.iter().enumerate() {
                if let Some(val) = group_key.get(i) {
                    result_row.insert(col_name.clone(), val.clone());
                }
            }

            // Add aggregate results
            for (agg_name, accumulator) in aggregates {
                let agg_result = accumulator.compute()?;
                result_row.insert(agg_name.clone(), agg_result);
            }

            results.push(result_row);
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_aggregate() {
        let mut acc = AggregateAccumulator::new(AggregateFunction::Count);
        acc.add(Value::Integer(1));
        acc.add(Value::Integer(2));
        acc.add(Value::Integer(3));
        
        let result = acc.compute().unwrap();
        assert_eq!(result, Value::Integer(3));
    }

    #[test]
    fn test_sum_aggregate() {
        let mut acc = AggregateAccumulator::new(AggregateFunction::Sum);
        acc.add(Value::Integer(10));
        acc.add(Value::Integer(20));
        acc.add(Value::Integer(30));
        
        let result = acc.compute().unwrap();
        assert_eq!(result, Value::Integer(60));
    }

    #[test]
    fn test_avg_aggregate() {
        let mut acc = AggregateAccumulator::new(AggregateFunction::Avg);
        acc.add(Value::Integer(10));
        acc.add(Value::Integer(20));
        acc.add(Value::Integer(30));
        
        let result = acc.compute().unwrap();
        match result {
            Value::Float(f) => assert!((f - 20.0).abs() < 0.01),
            _ => panic!("Expected float"),
        }
    }

    #[test]
    fn test_min_aggregate() {
        let mut acc = AggregateAccumulator::new(AggregateFunction::Min);
        acc.add(Value::Integer(30));
        acc.add(Value::Integer(10));
        acc.add(Value::Integer(20));
        
        let result = acc.compute().unwrap();
        assert_eq!(result, Value::Integer(10));
    }

    #[test]
    fn test_max_aggregate() {
        let mut acc = AggregateAccumulator::new(AggregateFunction::Max);
        acc.add(Value::Integer(30));
        acc.add(Value::Integer(10));
        acc.add(Value::Integer(20));
        
        let result = acc.compute().unwrap();
        assert_eq!(result, Value::Integer(30));
    }

    #[test]
    fn test_sum_with_floats() {
        let mut acc = AggregateAccumulator::new(AggregateFunction::Sum);
        acc.add(Value::Integer(10));
        acc.add(Value::Float(5.5));
        acc.add(Value::Integer(15));
        
        let result = acc.compute().unwrap();
        match result {
            Value::Float(f) => assert!((f - 30.5).abs() < 0.01),
            _ => panic!("Expected float"),
        }
    }

    #[test]
    fn test_empty_aggregate() {
        let acc = AggregateAccumulator::new(AggregateFunction::Count);
        let result = acc.compute().unwrap();
        assert_eq!(result, Value::Integer(0));
    }

    #[test]
    fn test_group_by_processor() {
        let mut processor = GroupByProcessor::new(vec!["category".to_string()]);
        
        let mut row1 = Row::new(crate::types::RecordId::new());
        row1.insert("category".to_string(), Value::String("A".to_string()));
        row1.insert("amount".to_string(), Value::Integer(10));
        
        let mut row2 = Row::new(crate::types::RecordId::new());
        row2.insert("category".to_string(), Value::String("A".to_string()));
        row2.insert("amount".to_string(), Value::Integer(20));
        
        processor.add_row(&row1, vec![("total".to_string(), AggregateFunction::Sum, "amount".to_string())]).unwrap();
        processor.add_row(&row2, vec![("total".to_string(), AggregateFunction::Sum, "amount".to_string())]).unwrap();
        
        let results = processor.get_results().unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_aggregate_function_from_str() {
        assert_eq!(AggregateFunction::from_str("COUNT"), Some(AggregateFunction::Count));
        assert_eq!(AggregateFunction::from_str("SUM"), Some(AggregateFunction::Sum));
        assert_eq!(AggregateFunction::from_str("AVG"), Some(AggregateFunction::Avg));
        assert_eq!(AggregateFunction::from_str("MIN"), Some(AggregateFunction::Min));
        assert_eq!(AggregateFunction::from_str("MAX"), Some(AggregateFunction::Max));
        assert_eq!(AggregateFunction::from_str("INVALID"), None);
    }

    #[test]
    fn test_min_aggregate_zero() {
        let mut acc = AggregateAccumulator::new(AggregateFunction::Min);
        acc.add(Value::Integer(30));
        acc.add(Value::Integer(0));
        acc.add(Value::Integer(20));
        
        let result = acc.compute().unwrap();
        assert_eq!(result, Value::Integer(0));
    }

    #[test]
    fn test_string_min_max() {
        let mut acc_min = AggregateAccumulator::new(AggregateFunction::Min);
        acc_min.add(Value::String("zebra".to_string()));
        acc_min.add(Value::String("apple".to_string()));
        acc_min.add(Value::String("mango".to_string()));
        
        let result_min = acc_min.compute().unwrap();
        assert_eq!(result_min, Value::String("apple".to_string()));

        let mut acc_max = AggregateAccumulator::new(AggregateFunction::Max);
        acc_max.add(Value::String("zebra".to_string()));
        acc_max.add(Value::String("apple".to_string()));
        acc_max.add(Value::String("mango".to_string()));
        
        let result_max = acc_max.compute().unwrap();
        assert_eq!(result_max, Value::String("zebra".to_string()));
    }
}
