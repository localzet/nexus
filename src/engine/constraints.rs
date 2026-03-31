//! Ограничения данных: PRIMARY KEY, UNIQUE, FOREIGN KEY, CHECK

use crate::types::*;
use std::collections::{HashMap, HashSet};
use anyhow::{Result, anyhow};

/// Constraint types
#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintType {
    PrimaryKey,
    UniqueKey,
    ForeignKey { ref_table: String, ref_column: String },
    CheckConstraint(String), // SQL expression
}

/// Constraint definition
#[derive(Debug, Clone)]
pub struct Constraint {
    pub name: String,
    pub columns: Vec<String>,
    pub constraint_type: ConstraintType,
}

/// Constraint metadata storage
#[derive(Debug, Clone)]
pub struct ConstraintMetadata {
    pub table_name: String,
    pub constraints: Vec<Constraint>,
    pub unique_indexes: HashMap<String, HashSet<String>>, // column -> set of values
    pub primary_key_column: Option<String>,
    pub foreign_keys: Vec<ForeignKeyRef>,
}

/// Foreign key reference
#[derive(Debug, Clone)]
pub struct ForeignKeyRef {
    pub column: String,
    pub ref_table: String,
    pub ref_column: String,
}

impl ConstraintMetadata {
    pub fn new(table_name: String) -> Self {
        Self {
            table_name,
            constraints: Vec::new(),
            unique_indexes: HashMap::new(),
            primary_key_column: None,
            foreign_keys: Vec::new(),
        }
    }

    /// Add a constraint
    pub fn add_constraint(&mut self, constraint: Constraint) -> Result<()> {
        // Check for duplicate constraint names
        if self.constraints.iter().any(|c| c.name == constraint.name) {
            return Err(anyhow!("Constraint '{}' already exists", constraint.name));
        }

        // Only allow one PRIMARY KEY
        if constraint.constraint_type == ConstraintType::PrimaryKey {
            if self.primary_key_column.is_some() {
                return Err(anyhow!("Table already has a PRIMARY KEY"));
            }
            if constraint.columns.len() != 1 {
                return Err(anyhow!("PRIMARY KEY must be on a single column"));
            }
            self.primary_key_column = Some(constraint.columns[0].clone());
        }

        // Initialize unique index for UNIQUE constraints
        if constraint.constraint_type == ConstraintType::UniqueKey {
            for col in &constraint.columns {
                self.unique_indexes.entry(col.clone()).or_insert_with(HashSet::new);
            }
        }

        // Track FOREIGN KEYs
        if let ConstraintType::ForeignKey { ref ref_table, ref ref_column } = constraint.constraint_type {
            for col in &constraint.columns {
                self.foreign_keys.push(ForeignKeyRef {
                    column: col.clone(),
                    ref_table: ref_table.clone(),
                    ref_column: ref_column.clone(),
                });
            }
        }

        self.constraints.push(constraint);
        Ok(())
    }

    /// Validate a row against all constraints
    pub fn validate_row(&self, row: &Row, existing_rows: &[Row]) -> Result<()> {
        for constraint in &self.constraints {
            match &constraint.constraint_type {
                ConstraintType::PrimaryKey => {
                    self.validate_primary_key(row, &constraint.columns, existing_rows)?;
                }
                ConstraintType::UniqueKey => {
                    self.validate_unique_key(row, &constraint.columns, existing_rows)?;
                }
                ConstraintType::ForeignKey { ref_table, ref_column } => {
                    self.validate_foreign_key(row, &constraint.columns, ref_table, ref_column)?;
                }
                ConstraintType::CheckConstraint(expr) => {
                    self.validate_check_constraint(row, expr)?;
                }
            }
        }
        Ok(())
    }

    fn validate_primary_key(&self, row: &Row, columns: &[String], existing_rows: &[Row]) -> Result<()> {
        // Extract primary key values
        let pk_values: Vec<String> = columns
            .iter()
            .map(|col| self.value_to_string(row.values.get(col)))
            .collect();

        // Check for NULL values
        if pk_values.iter().any(|v| v == "NULL") {
            return Err(anyhow!("PRIMARY KEY column cannot be NULL"));
        }

        // Check for duplicates in existing rows
        for existing in existing_rows {
            let existing_pk: Vec<String> = columns
                .iter()
                .map(|col| self.value_to_string(existing.values.get(col)))
                .collect();

            if pk_values == existing_pk {
                return Err(anyhow!("Duplicate entry for PRIMARY KEY"));
            }
        }

        Ok(())
    }

    fn validate_unique_key(&self, row: &Row, columns: &[String], existing_rows: &[Row]) -> Result<()> {
        // Extract unique key values
        let uk_values: Vec<String> = columns
            .iter()
            .map(|col| self.value_to_string(row.values.get(col)))
            .collect();

        // Allow multiple NULL values for UNIQUE (standard behavior)
        if uk_values.iter().all(|v| v == "NULL") {
            return Ok(());
        }

        // Check for duplicates in existing rows
        for existing in existing_rows {
            let existing_uk: Vec<String> = columns
                .iter()
                .map(|col| self.value_to_string(existing.values.get(col)))
                .collect();

            if uk_values == existing_uk && !uk_values.iter().any(|v| v == "NULL") {
                return Err(anyhow!("Duplicate entry for UNIQUE constraint on columns: {}", columns.join(", ")));
            }
        }

        Ok(())
    }

    fn validate_foreign_key(&self, _row: &Row, _columns: &[String], _ref_table: &str, _ref_column: &str) -> Result<()> {
        // Simplified - full implementation would check against referenced table
        // For now, just return OK
        Ok(())
    }

    fn validate_check_constraint(&self, row: &Row, _expression: &str) -> Result<()> {
        // Simplified - would evaluate CHECK expression against row values
        // For now, just return OK
        Ok(())
    }

    fn value_to_string(&self, val: Option<&Value>) -> String {
        match val {
            Some(Value::Null) => "NULL".to_string(),
            Some(Value::String(s)) => s.clone(),
            Some(Value::Integer(i)) => i.to_string(),
            Some(Value::Float(f)) => f.to_string(),
            Some(Value::Boolean(b)) => b.to_string(),
            _ => "NULL".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_primary_key_constraint() {
        let mut meta = ConstraintMetadata::new("users".to_string());
        
        let pk = Constraint {
            name: "pk_users".to_string(),
            columns: vec!["id".to_string()],
            constraint_type: ConstraintType::PrimaryKey,
        };
        
        assert!(meta.add_constraint(pk).is_ok());
        assert_eq!(meta.primary_key_column, Some("id".to_string()));
    }

    #[test]
    fn test_unique_constraint() {
        let mut meta = ConstraintMetadata::new("users".to_string());
        
        let uk = Constraint {
            name: "uk_email".to_string(),
            columns: vec!["email".to_string()],
            constraint_type: ConstraintType::UniqueKey,
        };
        
        assert!(meta.add_constraint(uk).is_ok());
        assert!(meta.unique_indexes.contains_key("email"));
    }

    #[test]
    fn test_foreign_key_constraint() {
        let mut meta = ConstraintMetadata::new("orders".to_string());
        
        let fk = Constraint {
            name: "fk_user".to_string(),
            columns: vec!["user_id".to_string()],
            constraint_type: ConstraintType::ForeignKey {
                ref_table: "users".to_string(),
                ref_column: "id".to_string(),
            },
        };
        
        assert!(meta.add_constraint(fk).is_ok());
        assert_eq!(meta.foreign_keys.len(), 1);
    }

    #[test]
    fn test_duplicate_constraint_name() {
        let mut meta = ConstraintMetadata::new("users".to_string());
        
        let c1 = Constraint {
            name: "constraint1".to_string(),
            columns: vec!["id".to_string()],
            constraint_type: ConstraintType::PrimaryKey,
        };
        
        let c2 = Constraint {
            name: "constraint1".to_string(),
            columns: vec!["email".to_string()],
            constraint_type: ConstraintType::UniqueKey,
        };
        
        assert!(meta.add_constraint(c1).is_ok());
        assert!(meta.add_constraint(c2).is_err());
    }

    #[test]
    fn test_multiple_primary_keys_rejected() {
        let mut meta = ConstraintMetadata::new("users".to_string());
        
        let pk1 = Constraint {
            name: "pk1".to_string(),
            columns: vec!["id".to_string()],
            constraint_type: ConstraintType::PrimaryKey,
        };
        
        let pk2 = Constraint {
            name: "pk2".to_string(),
            columns: vec!["email".to_string()],
            constraint_type: ConstraintType::PrimaryKey,
        };
        
        assert!(meta.add_constraint(pk1).is_ok());
        assert!(meta.add_constraint(pk2).is_err());
    }
}
