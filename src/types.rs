/// ═══════════════════════════════════════════════════════════════════════════════
/// NEXUS DB - Универсальные типы данных
/// ═══════════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Уникальный идентификатор записи
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecordId(pub Uuid);

impl RecordId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for RecordId {
    fn default() -> Self {
        Self::new()
    }
}

/// Единица данных - может быть строкой, числом, JSON и т.д.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Binary(Vec<u8>),
    Json(serde_json::Value),
    Vector(Vec<f32>),  // Для векторных операций
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Boolean(_) => "boolean",
            Value::Integer(_) => "integer",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Binary(_) => "binary",
            Value::Json(_) => "json",
            Value::Vector(_) => "vector",
        }
    }

    /// Преобразование в JSON для сериализации
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Value::Null => serde_json::json!(null),
            Value::Boolean(b) => serde_json::json!(b),
            Value::Integer(i) => serde_json::json!(i),
            Value::Float(f) => serde_json::json!(f),
            Value::String(s) => serde_json::json!(s),
            Value::Binary(b) => serde_json::json!(base64::encode(b)),
            Value::Json(j) => j.clone(),
            Value::Vector(v) => serde_json::json!(v),
        }
    }
}

/// Определение колонки в таблице
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDef {
    pub name: String,
    pub column_type: ColumnType,
    pub nullable: bool,
    pub default: Option<Value>,
}

/// Типы данных колонок
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnType {
    Integer,
    Float,
    String,
    Boolean,
    Binary,
    Json,
    Vector(u32),  // Размерность вектора
    Timestamp,
}

impl std::fmt::Display for ColumnType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColumnType::Integer => write!(f, "INTEGER"),
            ColumnType::Float => write!(f, "FLOAT"),
            ColumnType::String => write!(f, "STRING"),
            ColumnType::Boolean => write!(f, "BOOLEAN"),
            ColumnType::Binary => write!(f, "BINARY"),
            ColumnType::Json => write!(f, "JSON"),
            ColumnType::Vector(dim) => write!(f, "VECTOR({})", dim),
            ColumnType::Timestamp => write!(f, "TIMESTAMP"),
        }
    }
}

/// Табличная запись (строка)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    pub id: RecordId,
    pub values: HashMap<String, Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Row {
    pub fn new(id: RecordId) -> Self {
        let now = Utc::now();
        Self {
            id,
            values: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn insert(&mut self, column: String, value: Value) {
        self.values.insert(column, value);
        self.updated_at = Utc::now();
    }

    pub fn get(&self, column: &str) -> Option<&Value> {
        self.values.get(column)
    }
}

/// JSON Документ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: RecordId,
    pub data: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Document {
    pub fn new(id: RecordId, data: serde_json::Value) -> Self {
        let now = Utc::now();
        Self {
            id,
            data,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Граф-вершина (Node)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: RecordId,
    pub label: String,
    pub properties: HashMap<String, Value>,
    pub created_at: DateTime<Utc>,
}

impl GraphNode {
    pub fn new(id: RecordId, label: String) -> Self {
        Self {
            id,
            label,
            properties: HashMap::new(),
            created_at: Utc::now(),
        }
    }
}

/// Граф-ребро (Edge)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: RecordId,
    pub from_node: RecordId,
    pub to_node: RecordId,
    pub relation_type: String,
    pub properties: HashMap<String, Value>,
    pub created_at: DateTime<Utc>,
}

impl GraphEdge {
    pub fn new(from: RecordId, to: RecordId, relation_type: String) -> Self {
        Self {
            id: RecordId::new(),
            from_node: from,
            to_node: to,
            relation_type,
            properties: HashMap::new(),
            created_at: Utc::now(),
        }
    }
}

/// Результат запроса
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<HashMap<String, Value>>,
    pub execution_time_ms: f64,
    pub row_count: usize,
}

impl QueryResult {
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            execution_time_ms: 0.0,
            row_count: 0,
        }
    }

    pub fn empty() -> Self {
        Self::new()
    }
}

impl Default for QueryResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Metadata таблицы
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableMetadata {
    pub name: String,
    pub columns: Vec<ColumnDef>,
    pub primary_key: String,
    pub created_at: DateTime<Utc>,
    pub row_count: u64,
}

impl TableMetadata {
    pub fn new(name: String, primary_key: String) -> Self {
        Self {
            name,
            columns: Vec::new(),
            primary_key,
            created_at: Utc::now(),
            row_count: 0,
        }
    }

    pub fn add_column(&mut self, col: ColumnDef) {
        self.columns.push(col);
    }
}

/// Простая таблица для DML операций
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub name: String,
    pub rows: Vec<Row>,
}

impl Table {
    pub fn new(name: String) -> Self {
        Self {
            name,
            rows: Vec::new(),
        }
    }

    pub fn insert(&mut self, row: Row) {
        self.rows.push(row);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_type_names() {
        assert_eq!(Value::Null.type_name(), "null");
        assert_eq!(Value::Boolean(true).type_name(), "boolean");
        assert_eq!(Value::Integer(42).type_name(), "integer");
        assert_eq!(Value::String("hello".to_string()).type_name(), "string");
    }

    #[test]
    fn test_row_creation() {
        let mut row = Row::new(RecordId::new());
        row.insert("name".to_string(), Value::String("Alice".to_string()));
        row.insert("age".to_string(), Value::Integer(30));
        
        assert_eq!(row.get("name"), Some(&Value::String("Alice".to_string())));
        assert_eq!(row.get("age"), Some(&Value::Integer(30)));
    }

    #[test]
    fn test_graph_creation() {
        let node1 = GraphNode::new(RecordId::new(), "User".to_string());
        let node2 = GraphNode::new(RecordId::new(), "Post".to_string());
        let edge = GraphEdge::new(node1.id, node2.id, "WROTE".to_string());
        
        assert_eq!(edge.from_node, node1.id);
        assert_eq!(edge.to_node, node2.id);
        assert_eq!(edge.relation_type, "WROTE");
    }
}
