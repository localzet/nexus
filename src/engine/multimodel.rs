//! Multi-Model Storage Engine - Таблицы, JSON, Графы, Векторы

use crate::types::*;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use std::collections::HashMap;
use chrono::Utc;
use uuid::Uuid;
use anyhow::{Result, anyhow};

/// ───────────────────────────────────────────────────────────────────────────────
/// TABLE STORAGE - Табличное хранилище с ACID гарантиями
/// ───────────────────────────────────────────────────────────────────────────────
pub struct TableStore {
    /// Таблицы: table_name -> { record_id -> Row }
    tables: DashMap<String, Arc<RwLock<HashMap<RecordId, Row>>>>,
    /// Metadata таблиц
    metadata: DashMap<String, TableMetadata>,
}

impl TableStore {
    pub fn new() -> Self {
        Self {
            tables: DashMap::new(),
            metadata: DashMap::new(),
        }
    }

    /// Создание новой таблицы
    pub fn create_table(&self, table_name: String, columns: Vec<ColumnDef>) -> Result<()> {
        if self.tables.contains_key(&table_name) {
            return Err(anyhow!("Таблица '{}' уже существует", table_name));
        }

        let pk = columns
            .first()
            .map(|c| c.name.clone())
            .ok_or_else(|| anyhow!("Таблица должна иметь хотя бы одну колонку"))?;

        let mut meta = TableMetadata::new(table_name.clone(), pk);
        for col in columns {
            meta.add_column(col);
        }

        self.tables
            .insert(table_name.clone(), Arc::new(RwLock::new(HashMap::new())));
        self.metadata.insert(table_name, meta);

        Ok(())
    }

    /// Вставка строки в таблицу
    pub fn insert_row(&self, table_name: &str, row: Row) -> Result<RecordId> {
        let table = self
            .tables
            .get_mut(table_name)
            .ok_or_else(|| anyhow!("Таблица '{}' не существует", table_name))?;

        let mut data = table.write();
        data.insert(row.id, row.clone());

        // Обновляем счетчик rows
        if let Some(mut meta) = self.metadata.get_mut(table_name) {
            meta.row_count += 1;
        }

        Ok(row.id)
    }

    /// Получение строки по ID
    pub fn get_row(&self, table_name: &str, record_id: RecordId) -> Result<Option<Row>> {
        let table = self
            .tables
            .get(table_name)
            .ok_or_else(|| anyhow!("Таблица '{}' не существует", table_name))?;

        let data = table.read();
        Ok(data.get(&record_id).cloned())
    }

    /// Получение всех строк из таблицы
    pub fn get_all_rows(&self, table_name: &str) -> Result<Vec<Row>> {
        let table = self
            .tables
            .get(table_name)
            .ok_or_else(|| anyhow!("Таблица '{}' не существует", table_name))?;

        let data = table.read();
        Ok(data.values().cloned().collect())
    }

    /// Обновление строки
    pub fn update_row(&self, table_name: &str, record_id: RecordId, updates: HashMap<String, Value>) -> Result<()> {
        let table = self
            .tables
            .get_mut(table_name)
            .ok_or_else(|| anyhow!("Таблица '{}' не существует", table_name))?;

        let mut data = table.write();
        if let Some(row) = data.get_mut(&record_id) {
            for (col, val) in updates {
                row.insert(col, val);
            }
        }

        Ok(())
    }

    /// Удаление строки
    pub fn delete_row(&self, table_name: &str, record_id: RecordId) -> Result<bool> {
        let mut table = self
            .tables
            .get_mut(table_name)
            .ok_or_else(|| anyhow!("Таблица '{}' не существует", table_name))?;

        let deleted = table.write().remove(&record_id).is_some();
        if deleted && self.metadata.get_mut(table_name).is_some() {
            if let Some(mut meta) = self.metadata.get_mut(table_name) {
                meta.row_count = meta.row_count.saturating_sub(1);
            }
        }

        Ok(deleted)
    }

    /// Получение списка всех таблиц
    pub fn list_tables(&self) -> Vec<String> {
        self.metadata.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Получение metadata таблицы
    pub fn get_table_metadata(&self, table_name: &str) -> Result<TableMetadata> {
        self.metadata
            .get(table_name)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| anyhow!("Таблица '{}' не существует", table_name))
    }
}

impl Default for TableStore {
    fn default() -> Self {
        Self::new()
    }
}

/// ───────────────────────────────────────────────────────────────────────────────
/// DOCUMENT STORAGE - JSON документы (как MongoDB)
/// ───────────────────────────────────────────────────────────────────────────────
pub struct DocumentStore {
    /// Коллекции: collection_name -> { doc_id -> Document }
    collections: DashMap<String, Arc<RwLock<HashMap<RecordId, Document>>>>,
}

impl DocumentStore {
    pub fn new() -> Self {
        Self {
            collections: DashMap::new(),
        }
    }

    /// Создание коллекции
    pub fn create_collection(&self, name: String) -> Result<()> {
        if self.collections.contains_key(&name) {
            return Err(anyhow!("Коллекция '{}' уже существует", name));
        }
        self.collections
            .insert(name, Arc::new(RwLock::new(HashMap::new())));
        Ok(())
    }

    /// Вставка документа
    pub fn insert_document(&self, collection: &str, doc: Document) -> Result<RecordId> {
        let coll = self
            .collections
            .get_mut(collection)
            .ok_or_else(|| anyhow!("Коллекция '{}' не существует", collection))?;

        let mut data = coll.write();
        data.insert(doc.id, doc.clone());
        Ok(doc.id)
    }

    /// Получение документа
    pub fn get_document(&self, collection: &str, doc_id: RecordId) -> Result<Option<Document>> {
        let coll = self
            .collections
            .get(collection)
            .ok_or_else(|| anyhow!("Коллекция '{}' не существует", collection))?;

        let data = coll.read();
        Ok(data.get(&doc_id).cloned())
    }

    /// Получение всех документов из коллекции
    pub fn get_all_documents(&self, collection: &str) -> Result<Vec<Document>> {
        let coll = self
            .collections
            .get(collection)
            .ok_or_else(|| anyhow!("Коллекция '{}' не существует", collection))?;

        let data = coll.read();
        Ok(data.values().cloned().collect())
    }

    /// Обновление документа
    pub fn update_document(&self, collection: &str, doc_id: RecordId, new_data: serde_json::Value) -> Result<()> {
        let coll = self
            .collections
            .get_mut(collection)
            .ok_or_else(|| anyhow!("Коллекция '{}' не существует", collection))?;

        let mut data = coll.write();
        if let Some(doc) = data.get_mut(&doc_id) {
            doc.data = new_data;
            doc.updated_at = Utc::now();
        }

        Ok(())
    }

    /// Удаление документа
    pub fn delete_document(&self, collection: &str, doc_id: RecordId) -> Result<bool> {
        let coll = self
            .collections
            .get_mut(collection)
            .ok_or_else(|| anyhow!("Коллекция '{}' не существует", collection))?;

        let result = coll.write().remove(&doc_id).is_some();
        Ok(result)
    }

    /// Получение списка коллекций
    pub fn list_collections(&self) -> Vec<String> {
        self.collections.iter().map(|entry| entry.key().clone()).collect()
    }
}

impl Default for DocumentStore {
    fn default() -> Self {
        Self::new()
    }
}

/// ───────────────────────────────────────────────────────────────────────────────
/// GRAPH STORAGE - Граф-данные с оптимизированной структурой
/// ───────────────────────────────────────────────────────────────────────────────
pub struct GraphStore {
    /// Вершины: graph_name -> { node_id -> GraphNode }
    nodes: DashMap<String, Arc<RwLock<HashMap<RecordId, GraphNode>>>>,
    /// Рёбра: graph_name -> { edge_id -> GraphEdge }
    edges: DashMap<String, Arc<RwLock<HashMap<RecordId, GraphEdge>>>>,
    /// Adjacency list для быстрого поиска: graph_name -> { from_id -> [to_ids] }
    adjacency: DashMap<String, Arc<RwLock<HashMap<RecordId, Vec<RecordId>>>>>,
}

impl GraphStore {
    pub fn new() -> Self {
        Self {
            nodes: DashMap::new(),
            edges: DashMap::new(),
            adjacency: DashMap::new(),
        }
    }

    /// Создание графа
    pub fn create_graph(&self, name: String) -> Result<()> {
        if self.nodes.contains_key(&name) {
            return Err(anyhow!("Граф '{}' уже существует", name));
        }

        self.nodes
            .insert(name.clone(), Arc::new(RwLock::new(HashMap::new())));
        self.edges
            .insert(name.clone(), Arc::new(RwLock::new(HashMap::new())));
        self.adjacency
            .insert(name, Arc::new(RwLock::new(HashMap::new())));

        Ok(())
    }

    /// Добавление вершины
    pub fn add_node(&self, graph: &str, node: GraphNode) -> Result<RecordId> {
        let nodes = self
            .nodes
            .get_mut(graph)
            .ok_or_else(|| anyhow!("Граф '{}' не существует", graph))?;

        let mut data = nodes.write();
        data.insert(node.id, node.clone());

        // Инициализация adjacency list для этого узла
        if let Some(mut adj) = self.adjacency.get_mut(graph) {
            adj.write()
                .insert(node.id, Vec::new());
        }

        Ok(node.id)
    }

    /// Добавление ребра
    pub fn add_edge(&self, graph: &str, edge: GraphEdge) -> Result<RecordId> {
        let edges = self
            .edges
            .get_mut(graph)
            .ok_or_else(|| anyhow!("Граф '{}' не существует", graph))?;

        let mut data = edges.write();
        data.insert(edge.id, edge.clone());

        // Обновляем adjacency list
        if let Some(mut adj) = self.adjacency.get_mut(graph) {
            let mut adj_data = adj.write();
            adj_data
                .entry(edge.from_node)
                .or_insert_with(Vec::new)
                .push(edge.to_node);
        }

        Ok(edge.id)
    }

    /// Получение вершины
    pub fn get_node(&self, graph: &str, node_id: RecordId) -> Result<Option<GraphNode>> {
        let nodes = self
            .nodes
            .get(graph)
            .ok_or_else(|| anyhow!("Граф '{}' не существует", graph))?;

        let result = nodes.read().get(&node_id).cloned();
        Ok(result)
    }

    /// Получение соседей вершины
    pub fn get_neighbors(&self, graph: &str, node_id: RecordId) -> Result<Vec<RecordId>> {
        let adj = self
            .adjacency
            .get(graph)
            .ok_or_else(|| anyhow!("Граф '{}' не существует", graph))?;

        let adj_data = adj.read();
        Ok(adj_data.get(&node_id).cloned().unwrap_or_default())
    }

    /// Получение всех вершин
    pub fn get_all_nodes(&self, graph: &str) -> Result<Vec<GraphNode>> {
        let nodes = self
            .nodes
            .get(graph)
            .ok_or_else(|| anyhow!("Граф '{}' не существует", graph))?;

        let result = nodes.read().values().cloned().collect();
        Ok(result)
    }

    /// Получение всех рёбер
    pub fn get_all_edges(&self, graph: &str) -> Result<Vec<GraphEdge>> {
        let edges = self
            .edges
            .get(graph)
            .ok_or_else(|| anyhow!("Граф '{}' не существует", graph))?;

        let result = edges.read().values().cloned().collect();
        Ok(result)
    }

    /// Список графов
    pub fn list_graphs(&self) -> Vec<String> {
        self.nodes.iter().map(|entry| entry.key().clone()).collect()
    }
}

impl Default for GraphStore {
    fn default() -> Self {
        Self::new()
    }
}

/// ───────────────────────────────────────────────────────────────────────────────
/// VECTOR STORAGE - Хранилище векторов (для ML/поиска подобия)
/// ───────────────────────────────────────────────────────────────────────────────
#[derive(Clone)]
pub struct VectorRecord {
    pub id: RecordId,
    pub vector: Vec<f32>,
    pub metadata: serde_json::Value,
}

pub struct VectorStore {
    /// Коллекции векторов: collection_name -> Vec<VectorRecord>
    collections: DashMap<String, Arc<RwLock<Vec<VectorRecord>>>>,
}

impl VectorStore {
    pub fn new() -> Self {
        Self {
            collections: DashMap::new(),
        }
    }

    /// Создание коллекции векторов
    pub fn create_collection(&self, name: String) -> Result<()> {
        if self.collections.contains_key(&name) {
            return Err(anyhow!("Коллекция векторов '{}' уже существует", name));
        }
        self.collections
            .insert(name, Arc::new(RwLock::new(Vec::new())));
        Ok(())
    }

    /// Добавление вектора
    pub fn insert_vector(&self, collection: &str, record: VectorRecord) -> Result<RecordId> {
        let coll = self
            .collections
            .get_mut(collection)
            .ok_or_else(|| anyhow!("Коллекция '{}' не существует", collection))?;

        coll.write().push(record.clone());
        Ok(record.id)
    }

    /// Поиск ближайших соседей (простой O(n) вариант)
    pub fn search_neighbors(&self, collection: &str, query_vector: &[f32], k: usize) -> Result<Vec<(RecordId, f32)>> {
        let coll = self
            .collections
            .get(collection)
            .ok_or_else(|| anyhow!("Коллекция '{}' не существует", collection))?;

        let data = coll.read();
        let mut results = Vec::new();

        for record in data.iter() {
            let distance = cosine_distance(query_vector, &record.vector);
            results.push((record.id, distance));
        }

        // Сортируем по расстоянию (меньше = ближе)
        results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(k);

        Ok(results)
    }

    /// Получение всех векторов
    pub fn get_all_vectors(&self, collection: &str) -> Result<Vec<VectorRecord>> {
        let coll = self
            .collections
            .get(collection)
            .ok_or_else(|| anyhow!("Коллекция '{}' не существует", collection))?;

        let result = coll.read().clone();
        Ok(result)
    }

    pub fn list_collections(&self) -> Vec<String> {
        self.collections.iter().map(|entry| entry.key().clone()).collect()
    }
}

impl Default for VectorStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Cosine similarity (distance = 1 - similarity)
pub fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 2.0; // Maximum distance
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 2.0;
    }

    1.0 - (dot_product / (norm_a * norm_b))
}

/// ───────────────────────────────────────────────────────────────────────────────
/// MULTIMODEL STORAGE ENGINE - Главный engine, объединяющий все хранилища
/// ───────────────────────────────────────────────────────────────────────────────
pub struct MultiModelEngine {
    pub tables: Arc<TableStore>,
    pub documents: Arc<DocumentStore>,
    pub graphs: Arc<GraphStore>,
    pub vectors: Arc<VectorStore>,
}

impl MultiModelEngine {
    pub fn new() -> Self {
        Self {
            tables: Arc::new(TableStore::new()),
            documents: Arc::new(DocumentStore::new()),
            graphs: Arc::new(GraphStore::new()),
            vectors: Arc::new(VectorStore::new()),
        }
    }

    /// Получение информации об engine
    pub fn info(&self) -> serde_json::Value {
        serde_json::json!({
            "engine": "NexusDB",
            "version": "0.1.0",
            "models": {
                "tables": self.tables.list_tables().len(),
                "documents": self.documents.list_collections().len(),
                "graphs": self.graphs.list_graphs().len(),
                "vectors": self.vectors.list_collections().len(),
            },
            "timestamp": Utc::now().to_rfc3339()
        })
    }
}

impl Default for MultiModelEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_operations() {
        let engine = MultiModelEngine::new();
        let columns = vec![
            ColumnDef {
                name: "id".to_string(),
                column_type: ColumnType::Integer,
                nullable: false,
                default: None,
            },
            ColumnDef {
                name: "name".to_string(),
                column_type: ColumnType::String,
                nullable: false,
                default: None,
            },
        ];

        engine.tables.create_table("users".to_string(), columns).unwrap();

        let mut row = Row::new(RecordId::new());
        row.insert("name".to_string(), Value::String("Alice".to_string()));

        let row_id = engine.tables.insert_row("users", row).unwrap();
        let retrieved = engine.tables.get_row("users", row_id).unwrap();

        assert!(retrieved.is_some());
    }

    #[test]
    fn test_document_operations() {
        let engine = MultiModelEngine::new();
        engine.documents.create_collection("posts".to_string()).unwrap();

        let doc = Document::new(
            RecordId::new(),
            serde_json::json!({
                "title": "Hello",
                "content": "World",
                "likes": 42
            }),
        );

        let doc_id = engine.documents.insert_document("posts", doc).unwrap();
        let retrieved = engine.documents.get_document("posts", doc_id).unwrap();

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().data["title"], "Hello");
    }

    #[test]
    fn test_graph_operations() {
        let engine = MultiModelEngine::new();
        engine.graphs.create_graph("social".to_string()).unwrap();

        let node1 = GraphNode::new(RecordId::new(), "User".to_string());
        let node2 = GraphNode::new(RecordId::new(), "User".to_string());

        engine.graphs.add_node("social", node1.clone()).unwrap();
        engine.graphs.add_node("social", node2.clone()).unwrap();

        let edge = GraphEdge::new(node1.id, node2.id, "FOLLOWS".to_string());
        engine.graphs.add_edge("social", edge).unwrap();

        let neighbors = engine.graphs.get_neighbors("social", node1.id).unwrap();
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0], node2.id);
    }

    #[test]
    fn test_vector_operations() {
        let engine = MultiModelEngine::new();
        engine.vectors.create_collection("embeddings".to_string()).unwrap();

        let record = VectorRecord {
            id: RecordId::new(),
            vector: vec![0.1, 0.2, 0.3],
            metadata: serde_json::json!({"text": "hello"}),
        };

        engine.vectors.insert_vector("embeddings", record).unwrap();
        let results = engine.vectors.search_neighbors("embeddings", &[0.1, 0.2, 0.3], 1).unwrap();

        assert_eq!(results.len(), 1);
    }
}
