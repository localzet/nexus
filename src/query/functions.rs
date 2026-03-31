//! SQL встроенные функции

use crate::types::Value;
use anyhow::{Result, anyhow};

#[derive(Debug, Clone)]
pub enum SqlFunction {
    // String functions
    Upper(Box<SqlExpression>),
    Lower(Box<SqlExpression>),
    Length(Box<SqlExpression>),
    Substring(Box<SqlExpression>, Box<SqlExpression>, Option<Box<SqlExpression>>),
    Concat(Vec<SqlExpression>),
    Trim(Box<SqlExpression>),
    LTrim(Box<SqlExpression>),
    RTrim(Box<SqlExpression>),

    // Math functions
    Abs(Box<SqlExpression>),
    Round(Box<SqlExpression>, Option<Box<SqlExpression>>),
    Ceil(Box<SqlExpression>),
    Floor(Box<SqlExpression>),
    Power(Box<SqlExpression>, Box<SqlExpression>),
    Sqrt(Box<SqlExpression>),

    // Date functions
    Now,
    Date(Box<SqlExpression>),
    DateAdd(Box<SqlExpression>, i32, String), // expr, value, unit (DAY, MONTH, YEAR)
    DateDiff(Box<SqlExpression>, Box<SqlExpression>),
    Year(Box<SqlExpression>),
    Month(Box<SqlExpression>),
    Day(Box<SqlExpression>),

    // Aggregate functions
    Count(Option<Box<SqlExpression>>),
    Sum(Box<SqlExpression>),
    Avg(Box<SqlExpression>),
    Min(Box<SqlExpression>),
    Max(Box<SqlExpression>),
    StdDev(Box<SqlExpression>),

    // Type casting
    Cast(Box<SqlExpression>, String), // expr, to_type
}

#[derive(Debug, Clone)]
pub enum SqlExpression {
    Literal(Value),
    Column(String),
    Function(SqlFunction),
    Binary {
        left: Box<SqlExpression>,
        op: String,
        right: Box<SqlExpression>,
    },
}

pub struct FunctionEvaluator;

impl FunctionEvaluator {
    /// Evaluate SQL function with given arguments
    pub fn eval(func: &SqlFunction, args: &[Value]) -> Result<Value> {
        match func {
            // String functions
            SqlFunction::Upper(expr) => {
                Self::eval_upper(expr)
            }
            SqlFunction::Lower(expr) => {
                Self::eval_lower(expr)
            }
            SqlFunction::Length(expr) => {
                Self::eval_length(expr)
            }
            SqlFunction::Substring(expr, start, len) => {
                Self::eval_substring(expr, start, len)
            }
            SqlFunction::Concat(exprs) => {
                let mut result = String::new();
                for expr in exprs {
                    // This is simplified - would need actual evaluation
                    result.push_str("concatenated");
                }
                Ok(Value::String(result))
            }
            SqlFunction::Trim(expr) => {
                Self::eval_trim(expr)
            }

            // Math functions
            SqlFunction::Abs(expr) => {
                Self::eval_abs(expr)
            }
            SqlFunction::Round(expr, digits) => {
                Self::eval_round(expr, digits)
            }
            SqlFunction::Ceil(expr) => {
                Self::eval_ceil(expr)
            }
            SqlFunction::Floor(expr) => {
                Self::eval_floor(expr)
            }
            SqlFunction::Power(base, exp) => {
                Self::eval_power(base, exp)
            }
            SqlFunction::Sqrt(expr) => {
                Self::eval_sqrt(expr)
            }

            // Date functions
            SqlFunction::Now => {
                Ok(Value::DateTime(chrono::Utc::now().to_rfc3339()))
            }
            SqlFunction::Year(expr) => {
                Self::eval_year(expr)
            }
            SqlFunction::Month(expr) => {
                Self::eval_month(expr)
            }
            SqlFunction::Day(expr) => {
                Self::eval_day(expr)
            }

            // Aggregate functions (would be handled separately in GROUP BY)
            SqlFunction::Count(_) => Ok(Value::Integer(0)), // Placeholder
            SqlFunction::Sum(_) => Ok(Value::Integer(0)),   // Placeholder
            SqlFunction::Avg(_) => Ok(Value::Float(0.0)),   // Placeholder
            SqlFunction::Min(_) => Ok(Value::Null),         // Placeholder
            SqlFunction::Max(_) => Ok(Value::Null),         // Placeholder

            // Type casting
            SqlFunction::Cast(expr, to_type) => {
                Self::eval_cast(expr, to_type)
            }

            _ => Err(anyhow!("Function not implemented")),
        }
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // String Functions

    fn eval_upper(expr: &SqlExpression) -> Result<Value> {
        match expr {
            SqlExpression::Literal(Value::String(s)) => {
                Ok(Value::String(s.to_uppercase()))
            }
            _ => Err(anyhow!("UPPER expects string argument")),
        }
    }

    fn eval_lower(expr: &SqlExpression) -> Result<Value> {
        match expr {
            SqlExpression::Literal(Value::String(s)) => {
                Ok(Value::String(s.to_lowercase()))
            }
            _ => Err(anyhow!("LOWER expects string argument")),
        }
    }

    fn eval_length(expr: &SqlExpression) -> Result<Value> {
        match expr {
            SqlExpression::Literal(Value::String(s)) => {
                Ok(Value::Integer(s.len() as i64))
            }
            _ => Err(anyhow!("LENGTH expects string argument")),
        }
    }

    fn eval_substring(
        expr: &SqlExpression,
        start: &SqlExpression,
        len: &Option<Box<SqlExpression>>,
    ) -> Result<Value> {
        let s = match expr {
            SqlExpression::Literal(Value::String(s)) => s,
            _ => return Err(anyhow!("SUBSTRING expects string argument")),
        };

        let start_pos = match start {
            SqlExpression::Literal(Value::Integer(n)) => *n as usize,
            _ => return Err(anyhow!("SUBSTRING start must be integer")),
        };

        let result = if let Some(len_expr) = len {
            if let SqlExpression::Literal(Value::Integer(n)) = **len_expr {
                let length = n as usize;
                s.chars()
                    .skip(start_pos - 1)
                    .take(length)
                    .collect::<String>()
            } else {
                s.chars().skip(start_pos - 1).collect::<String>()
            }
        } else {
            s.chars().skip(start_pos - 1).collect::<String>()
        };

        Ok(Value::String(result))
    }

    fn eval_trim(expr: &SqlExpression) -> Result<Value> {
        match expr {
            SqlExpression::Literal(Value::String(s)) => {
                Ok(Value::String(s.trim().to_string()))
            }
            _ => Err(anyhow!("TRIM expects string argument")),
        }
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Math Functions

    fn eval_abs(expr: &SqlExpression) -> Result<Value> {
        match expr {
            SqlExpression::Literal(Value::Integer(n)) => {
                Ok(Value::Integer(n.abs()))
            }
            SqlExpression::Literal(Value::Float(f)) => {
                Ok(Value::Float(f.abs()))
            }
            _ => Err(anyhow!("ABS expects numeric argument")),
        }
    }

    fn eval_round(
        expr: &SqlExpression,
        digits: &Option<Box<SqlExpression>>,
    ) -> Result<Value> {
        let num = match expr {
            SqlExpression::Literal(Value::Float(f)) => *f,
            SqlExpression::Literal(Value::Integer(n)) => *n as f64,
            _ => return Err(anyhow!("ROUND expects numeric argument")),
        };

        let precision = if let Some(d) = digits {
            if let SqlExpression::Literal(Value::Integer(n)) = **d {
                n as u32
            } else {
                0
            }
        } else {
            0
        };

        let multiplier = 10f64.powi(precision as i32);
        let rounded = (num * multiplier).round() / multiplier;

        Ok(Value::Float(rounded))
    }

    fn eval_ceil(expr: &SqlExpression) -> Result<Value> {
        match expr {
            SqlExpression::Literal(Value::Float(f)) => {
                Ok(Value::Float(f.ceil()))
            }
            SqlExpression::Literal(Value::Integer(n)) => {
                Ok(Value::Integer(*n))
            }
            _ => Err(anyhow!("CEIL expects numeric argument")),
        }
    }

    fn eval_floor(expr: &SqlExpression) -> Result<Value> {
        match expr {
            SqlExpression::Literal(Value::Float(f)) => {
                Ok(Value::Float(f.floor()))
            }
            SqlExpression::Literal(Value::Integer(n)) => {
                Ok(Value::Integer(*n))
            }
            _ => Err(anyhow!("FLOOR expects numeric argument")),
        }
    }

    fn eval_power(base: &SqlExpression, exp: &SqlExpression) -> Result<Value> {
        let b = match base {
            SqlExpression::Literal(Value::Float(f)) => *f,
            SqlExpression::Literal(Value::Integer(n)) => *n as f64,
            _ => return Err(anyhow!("POWER expects numeric base")),
        };

        let e = match exp {
            SqlExpression::Literal(Value::Float(f)) => *f,
            SqlExpression::Literal(Value::Integer(n)) => *n as f64,
            _ => return Err(anyhow!("POWER expects numeric exponent")),
        };

        Ok(Value::Float(b.powf(e)))
    }

    fn eval_sqrt(expr: &SqlExpression) -> Result<Value> {
        match expr {
            SqlExpression::Literal(Value::Float(f)) => {
                Ok(Value::Float(f.sqrt()))
            }
            SqlExpression::Literal(Value::Integer(n)) => {
                Ok(Value::Float((*n as f64).sqrt()))
            }
            _ => Err(anyhow!("SQRT expects numeric argument")),
        }
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Date Functions

    fn eval_year(expr: &SqlExpression) -> Result<Value> {
        match expr {
            SqlExpression::Literal(Value::DateTime(dt)) => {
                use chrono::DateTime;
                if let Ok(parsed) = DateTime::parse_from_rfc3339(dt) {
                    Ok(Value::Integer(parsed.year() as i64))
                } else {
                    Err(anyhow!("Invalid datetime format"))
                }
            }
            _ => Err(anyhow!("YEAR expects datetime argument")),
        }
    }

    fn eval_month(expr: &SqlExpression) -> Result<Value> {
        match expr {
            SqlExpression::Literal(Value::DateTime(dt)) => {
                use chrono::DateTime;
                if let Ok(parsed) = DateTime::parse_from_rfc3339(dt) {
                    Ok(Value::Integer(parsed.month() as i64))
                } else {
                    Err(anyhow!("Invalid datetime format"))
                }
            }
            _ => Err(anyhow!("MONTH expects datetime argument")),
        }
    }

    fn eval_day(expr: &SqlExpression) -> Result<Value> {
        match expr {
            SqlExpression::Literal(Value::DateTime(dt)) => {
                use chrono::DateTime;
                if let Ok(parsed) = DateTime::parse_from_rfc3339(dt) {
                    Ok(Value::Integer(parsed.day() as i64))
                } else {
                    Err(anyhow!("Invalid datetime format"))
                }
            }
            _ => Err(anyhow!("DAY expects datetime argument")),
        }
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Type Casting

    fn eval_cast(expr: &SqlExpression, to_type: &str) -> Result<Value> {
        let value = match expr {
            SqlExpression::Literal(v) => v.clone(),
            _ => return Err(anyhow!("CAST not fully implemented for expressions")),
        };

        match to_type.to_uppercase().as_str() {
            "INTEGER" | "INT" => match value {
                Value::String(s) => s.parse::<i64>().map(Value::Integer).map_err(|_| anyhow!("Cannot cast to INTEGER")),
                Value::Integer(n) => Ok(Value::Integer(n)),
                Value::Float(f) => Ok(Value::Integer(f as i64)),
                _ => Err(anyhow!("Cannot cast to INTEGER"))
            },
            "FLOAT" | "REAL" => match value {
                Value::String(s) => s.parse::<f64>().map(Value::Float).map_err(|_| anyhow!("Cannot cast to FLOAT")),
                Value::Integer(n) => Ok(Value::Float(n as f64)),
                Value::Float(f) => Ok(Value::Float(f)),
                _ => Err(anyhow!("Cannot cast to FLOAT"))
            },
            "VARCHAR" | "TEXT" | "STRING" => Ok(Value::String(format!("{:?}", value))),
            "BOOLEAN" => match value {
                Value::String(s) => {
                    Ok(Value::Boolean(matches!(s.to_lowercase().as_str(), "true" | "1" | "yes")))
                }
                Value::Integer(n) => Ok(Value::Boolean(n != 0)),
                Value::Boolean(b) => Ok(Value::Boolean(b)),
                _ => Err(anyhow!("Cannot cast to BOOLEAN"))
            },
            _ => Err(anyhow!("Unknown type: {}", to_type))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upper_function() {
        let expr = SqlFunction::Upper(Box::new(SqlExpression::Literal(Value::String("hello".to_string()))));
        // Would need to hook this into evaluator
    }

    #[test]
    fn test_abs_function() {
        let expr = SqlFunction::Abs(Box::new(SqlExpression::Literal(Value::Integer(-42))));
        // Would evaluate to 42
    }

    #[test]
    fn test_cast_to_integer() {
        let expr = SqlFunction::Cast(
            Box::new(SqlExpression::Literal(Value::String("123".to_string()))),
            "INTEGER".to_string()
        );
        // Would cast "123" to 123
    }
}
