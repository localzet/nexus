//! Advanced SQL Parser - WHERE, JOINs, GROUP BY, подзапросы

use sqlparser::parser::Parser;
use sqlparser::dialect::GenericDialect;
use sqlparser::ast::*;
use anyhow::{Result, anyhow};

#[derive(Debug, Clone)]
pub enum FilterOp {
    Equal,
    NotEqual,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Like,
    In,
    Between,
    IsNull,
    IsNotNull,
}

#[derive(Debug, Clone)]
pub struct Filter {
    pub column: String,
    pub op: FilterOp,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct ParsedSelect {
    pub table: String,
    pub columns: Vec<String>,
    pub filters: Vec<Filter>,
    pub joins: Vec<ParsedJoin>,
    pub group_by: Vec<String>,
    pub order_by: Vec<(String, bool)>, // (column, ascending)
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub distinct: bool,
}

#[derive(Debug, Clone)]
pub struct ParsedJoin {
    pub join_type: JoinType,
    pub table: String,
    pub on_condition: Filter,
}

#[derive(Debug, Clone, Copy)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

/// Advanced SQL Parser
pub struct AdvancedSqlParser {
    dialect: GenericDialect,
}

impl AdvancedSqlParser {
    pub fn new() -> Self {
        AdvancedSqlParser {
            dialect: GenericDialect {},
        }
    }

    /// Parse SELECT statement
    pub fn parse_select(&self, sql: &str) -> Result<ParsedSelect> {
        let statements = Parser::parse_sql(&self.dialect, sql)
            .map_err(|e| anyhow!("Parse error: {}", e))?;

        if statements.is_empty() {
            return Err(anyhow!("No statement parsed"));
        }

        match &statements[0] {
            Statement::Select(select) => self.parse_select_statement(select),
            _ => Err(anyhow!("Not a SELECT statement")),
        }
    }

    /// Parse CREATE TABLE statement
    pub fn parse_create_table(&self, sql: &str) -> Result<(String, Vec<ColumnDef>)> {
        let statements = Parser::parse_sql(&self.dialect, sql)
            .map_err(|e| anyhow!("Parse error: {}", e))?;

        if statements.is_empty() {
            return Err(anyhow!("No statement parsed"));
        }

        match &statements[0] {
            Statement::CreateTable { name, columns, .. } => {
                let table_name = name.to_string();
                let cols = columns
                    .iter()
                    .map(|col| self.parse_column_def(col))
                    .collect::<Result<Vec<_>>>()?;
                Ok((table_name, cols))
            }
            _ => Err(anyhow!("Not a CREATE TABLE statement")),
        }
    }

    /// Parse INSERT statement
    pub fn parse_insert(&self, sql: &str) -> Result<(String, Vec<String>, Vec<String>)> {
        let statements = Parser::parse_sql(&self.dialect, sql)
            .map_err(|e| anyhow!("Parse error: {}", e))?;

        if statements.is_empty() {
            return Err(anyhow!("No statement parsed"));
        }

        match &statements[0] {
            Statement::Insert {
                table_name,
                columns,
                source,
                ..
            } => {
                let table = table_name.to_string();
                let cols: Vec<String> = columns
                    .iter()
                    .map(|c| c.value.clone())
                    .collect();

                // Parse VALUES
                let values = match source {
                    Some(query) => self.extract_values_from_query(query)?,
                    None => Vec::new(),
                };

                Ok((table, cols, values))
            }
            _ => Err(anyhow!("Not an INSERT statement")),
        }
    }

    /// Parse UPDATE statement
    pub fn parse_update(&self, sql: &str) -> Result<(String, Vec<(String, String)>, Vec<Filter>)> {
        let statements = Parser::parse_sql(&self.dialect, sql)
            .map_err(|e| anyhow!("Parse error: {}", e))?;

        if statements.is_empty() {
            return Err(anyhow!("No statement parsed"));
        }

        match &statements[0] {
            Statement::Update {
                table,
                assignments,
                selection,
                ..
            } => {
                let table_name = table[0].relation.to_string();

                // Parse SET assignments
                let updates: Vec<(String, String)> = assignments
                    .iter()
                    .map(|a| {
                        let column = a.id[0].value.clone();
                        let value = a.value.to_string();
                        (column, value)
                    })
                    .collect();

                // Parse WHERE clause
                let filters = match selection {
                    Some(expr) => self.parse_where_expr(expr)?,
                    None => Vec::new(),
                };

                Ok((table_name, updates, filters))
            }
            _ => Err(anyhow!("Not an UPDATE statement")),
        }
    }

    /// Parse DELETE statement
    pub fn parse_delete(&self, sql: &str) -> Result<(String, Vec<Filter>)> {
        let statements = Parser::parse_sql(&self.dialect, sql)
            .map_err(|e| anyhow!("Parse error: {}", e))?;

        if statements.is_empty() {
            return Err(anyhow!("No statement parsed"));
        }

        match &statements[0] {
            Statement::Delete {
                table_name,
                selection,
                ..
            } => {
                let table = table_name.table.clone();

                let filters = match selection {
                    Some(expr) => self.parse_where_expr(expr)?,
                    None => Vec::new(),
                };

                Ok((table, filters))
            }
            _ => Err(anyhow!("Not a DELETE statement")),
        }
    }

    // ─────────────────────────────────────────────────────────────────────────

    fn parse_select_statement(&self, select: &Select) -> Result<ParsedSelect> {
        // Get columns
        let columns = self.parse_projection(&select.projection)?;

        // Get table and joins
        let (table, joins) = self.parse_from_and_joins(&select.from)?;

        // Get WHERE filters
        let filters = match &select.selection {
            Some(expr) => self.parse_where_expr(expr)?,
            None => Vec::new(),
        };

        // Get GROUP BY
        let group_by = select
            .group_by
            .iter()
            .map(|e| e.to_string())
            .collect();

        // Get ORDER BY
        let order_by = select
            .order_by
            .iter()
            .map(|ob| {
                let col = ob.expr.to_string();
                let asc = matches!(ob.asc, Some(true) | None);
                (col, asc)
            })
            .collect();

        // Get LIMIT and OFFSET
        let limit = select.limit.as_ref().map(|l| match l {
            Expr::Value(Value::Number(n, _)) => {
                n.parse::<u64>().unwrap_or(0)
            }
            _ => 0,
        });

        let offset = select.offset.as_ref().map(|o| {
            if let Offset {
                value: Expr::Value(Value::Number(n, _)),
                ..
            } = o
            {
                n.parse::<u64>().unwrap_or(0)
            } else {
                0
            }
        });

        let distinct = select.distinct.is_some();

        Ok(ParsedSelect {
            table,
            columns,
            filters,
            joins,
            group_by,
            order_by,
            limit,
            offset,
            distinct,
        })
    }

    fn parse_projection(&self, projection: &[SelectItem]) -> Result<Vec<String>> {
        let mut columns = Vec::new();

        for item in projection {
            match item {
                SelectItem::AllColumns => {
                    columns.push("*".to_string());
                }
                SelectItem::UnnamedExpr(expr) => {
                    columns.push(expr.to_string());
                }
                SelectItem::ExprWithAlias { expr, alias } => {
                    columns.push(format!("{} AS {}", expr, alias));
                }
                _ => {}
            }
        }

        if columns.is_empty() {
            columns.push("*".to_string());
        }

        Ok(columns)
    }

    fn parse_from_and_joins(&self, from: &[TableWithJoins]) -> Result<(String, Vec<ParsedJoin>)> {
        if from.is_empty() {
            return Err(anyhow!("No FROM clause"));
        }

        let table = from[0].relation.to_string();
        let mut joins = Vec::new();

        for twj in from {
            for join in &twj.joins {
                let join_type = self.parse_join_type(&join.join_operator);
                let join_table = join.relation.to_string();

                let on_condition = match &join.join_operator {
                    JoinOperator::Inner(constraint) | JoinOperator::LeftOuter(constraint) => {
                        if let JoinConstraint::On(expr) = constraint {
                            let filters = self.parse_where_expr(expr)?;
                            if !filters.is_empty() {
                                filters[0].clone()
                            } else {
                                Filter {
                                    column: "id".to_string(),
                                    op: FilterOp::Equal,
                                    value: "id".to_string(),
                                }
                            }
                        } else {
                            Filter {
                                column: "id".to_string(),
                                op: FilterOp::Equal,
                                value: "id".to_string(),
                            }
                        }
                    }
                    _ => Filter {
                        column: "id".to_string(),
                        op: FilterOp::Equal,
                        value: "id".to_string(),
                    },
                };

                joins.push(ParsedJoin {
                    join_type,
                    table: join_table,
                    on_condition,
                });
            }
        }

        Ok((table, joins))
    }

    fn parse_join_type(&self, op: &JoinOperator) -> JoinType {
        match op {
            JoinOperator::Inner(_) => JoinType::Inner,
            JoinOperator::LeftOuter(_) => JoinType::Left,
            JoinOperator::RightOuter(_) => JoinType::Right,
            JoinOperator::FullOuter(_) => JoinType::Full,
            JoinOperator::CrossJoin => JoinType::Cross,
            _ => JoinType::Inner,
        }
    }

    fn parse_where_expr(&self, expr: &Expr) -> Result<Vec<Filter>> {
        let mut filters = Vec::new();

        match expr {
            Expr::BinaryOp { left, op, right } => {
                let filter = self.parse_binary_op(left, op, right)?;
                filters.push(filter);
            }
            Expr::And { left, right } => {
                filters.extend(self.parse_where_expr(left)?);
                filters.extend(self.parse_where_expr(right)?);
            }
            Expr::Or { left, right } => {
                filters.extend(self.parse_where_expr(left)?);
                filters.extend(self.parse_where_expr(right)?);
            }
            _ => {
                // Try to parse as simple comparison
                let filter = Filter {
                    column: expr.to_string(),
                    op: FilterOp::Equal,
                    value: "true".to_string(),
                };
                filters.push(filter);
            }
        }

        Ok(filters)
    }

    fn parse_binary_op(&self, left: &Expr, op: &BinaryOperator, right: &Expr) -> Result<Filter> {
        let column = left.to_string();
        let value = right.to_string();

        let filter_op = match op {
            BinaryOperator::Equal => FilterOp::Equal,
            BinaryOperator::NotEqual => FilterOp::NotEqual,
            BinaryOperator::Gt => FilterOp::GreaterThan,
            BinaryOperator::GtEq => FilterOp::GreaterThanOrEqual,
            BinaryOperator::Lt => FilterOp::LessThan,
            BinaryOperator::LtEq => FilterOp::LessThanOrEqual,
            BinaryOperator::Like => FilterOp::Like,
            _ => FilterOp::Equal,
        };

        Ok(Filter {
            column,
            op: filter_op,
            value,
        })
    }

    fn parse_column_def(&self, col: &ColumnDef) -> Result<crate::types::ColumnDef> {
        let column_type = match &col.data_type {
            DataType::Int(_) | DataType::Integer(_) => crate::types::ColumnType::Integer,
            DataType::Varchar(_) | DataType::Text => crate::types::ColumnType::String,
            DataType::Boolean => crate::types::ColumnType::Boolean,
            DataType::Timestamp(_, _) => crate::types::ColumnType::DateTime,
            DataType::Float(_) | DataType::Double => crate::types::ColumnType::Float,
            _ => crate::types::ColumnType::String,
        };

        Ok(crate::types::ColumnDef {
            name: col.name.value.clone(),
            column_type,
            nullable: col.options.iter().all(|o| !matches!(o.option, ColumnOptionDef::NotNull)),
            default: None,
        })
    }

    fn extract_values_from_query(&self, query: &Query) -> Result<Vec<String>> {
        match &query.body {
            SetExpr::Values(values) => {
                let mut result = Vec::new();
                for value_list in &values.0 {
                    for expr in value_list {
                        result.push(expr.to_string());
                    }
                }
                Ok(result)
            }
            _ => Ok(Vec::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_select() {
        let parser = AdvancedSqlParser::new();
        let result = parser.parse_select("SELECT id, name FROM users");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_select_with_where() {
        let parser = AdvancedSqlParser::new();
        let result = parser.parse_select("SELECT * FROM users WHERE age > 30");
        assert!(result.is_ok());
        if let Ok(parsed) = result {
            assert!(!parsed.filters.is_empty());
        }
    }

    #[test]
    fn test_parse_select_with_order_by() {
        let parser = AdvancedSqlParser::new();
        let result = parser.parse_select("SELECT * FROM users ORDER BY name ASC LIMIT 10");
        assert!(result.is_ok());
        if let Ok(parsed) = result {
            assert!(parsed.limit == Some(10));
        }
    }

    #[test]
    fn test_parse_simple_insert() {
        let parser = AdvancedSqlParser::new();
        let result = parser.parse_insert("INSERT INTO users (name, age) VALUES ('Alice', 30)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_simple_update() {
        let parser = AdvancedSqlParser::new();
        let result = parser.parse_update("UPDATE users SET age = 31 WHERE id = 1");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_simple_delete() {
        let parser = AdvancedSqlParser::new();
        let result = parser.parse_delete("DELETE FROM users WHERE age < 18");
        assert!(result.is_ok());
    }
}
