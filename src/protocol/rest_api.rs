//! REST API Server - HTTP endpoints для всех операций

use axum::{
    extract::{Path, State, Json},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post, put, delete},
    Router,
};
use serde_json::json;
use std::sync::Arc;
use crate::engine::{MultiModelEngine, IndexingManager};
use crate::types::*;
use crate::query::QueryExecutor;

/// Application state
pub struct AppState {
    pub engine: Arc<MultiModelEngine>,
    pub indexing: Arc<IndexingManager>,
    pub query_executor: Arc<QueryExecutor>,
}

/// Error response handler
pub struct ApiError(String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": self.0
            }))
        ).into_response()
    }
}

/// Create REST API router
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(health_check))
        
        // Database info
        .route("/info", get(db_info))
        
        // Query endpoints
        .route("/query", post(execute_query))
        
        // Table endpoints
        .route("/tables", get(list_tables))
        .route("/tables/:name", post(create_table))
        .route("/tables/:name/rows", get(get_table_rows))
        .route("/tables/:name/rows", post(insert_row))
        .route("/tables/:name/rows/:row_id", put(update_row))
        .route("/tables/:name/rows/:row_id", delete(delete_row))
        
        // Document endpoints
        .route("/documents", get(list_collections))
        .route("/documents/:collection", post(create_collection))
        .route("/documents/:collection/docs", get(get_all_documents))
        .route("/documents/:collection/docs", post(insert_document))
        .route("/documents/:collection/docs/:doc_id", get(get_document))
        .route("/documents/:collection/docs/:doc_id", put(update_document))
        .route("/documents/:collection/docs/:doc_id", delete(delete_document))
        
        // Graph endpoints
        .route("/graphs", get(list_graphs))
        .route("/graphs/:name", post(create_graph))
        .route("/graphs/:name/nodes", get(get_graph_nodes))
        .route("/graphs/:name/nodes", post(add_node))
        .route("/graphs/:name/edges", get(get_graph_edges))
        .route("/graphs/:name/edges", post(add_edge))
        .route("/graphs/:name/neighbors/:node_id", get(get_node_neighbors))
        
        // Vector endpoints
        .route("/vectors", get(list_vector_collections))
        .route("/vectors/:collection", post(create_vector_collection))
        .route("/vectors/:collection/search", post(search_vectors))
        
        // Index endpoints
        .route("/indices/:table", get(list_table_indices))
        .route("/indices/:table/:column", post(create_index))
        
        // Cache endpoints
        .route("/cache/stats", get(cache_stats))
        .route("/cache/clear", post(clear_cache))
        
        .with_state(Arc::new(state))
}

/// ───────────────────────────────────────────────────────────────────────────────
/// HANDLERS
/// ───────────────────────────────────────────────────────────────────────────────

async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn db_info(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(state.engine.info())
}

async fn execute_query(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<QueryResult>, ApiError> {
    let sql = payload.get("sql")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError("Missing 'sql' field".to_string()))?;

    match state.query_executor.execute(sql) {
        Ok(result) => Ok(Json(result)),
        Err(e) => Err(ApiError(e.to_string())),
    }
}

async fn list_tables(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let tables = state.engine.tables.list_tables();
    Json(json!({"tables": tables}))
}

async fn create_table(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let columns = payload.get("columns")
        .ok_or_else(|| ApiError("Missing 'columns' field".to_string()))?;

    // Создаем простую схему для примера
    let schema = vec![
        ColumnDef {
            name: "id".to_string(),
            column_type: ColumnType::Integer,
            nullable: false,
            default: None,
        },
    ];

    match state.engine.tables.create_table(name.clone(), schema) {
        Ok(_) => Ok(Json(json!({"status": "created", "table": name}))),
        Err(e) => Err(ApiError(e.to_string())),
    }
}

async fn get_table_rows(
    State(state): State<Arc<AppState>>,
    Path(table_name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    match state.engine.tables.get_all_rows(&table_name) {
        Ok(rows) => {
            let rows_json: Vec<_> = rows.iter().map(|r| {
                json!({
                    "id": r.id.0.to_string(),
                    "created_at": r.created_at.to_rfc3339(),
                    "updated_at": r.updated_at.to_rfc3339(),
                })
            }).collect();
            Ok(Json(json!({"rows": rows_json})))
        }
        Err(e) => Err(ApiError(e.to_string())),
    }
}

async fn insert_row(
    State(state): State<Arc<AppState>>,
    Path(table_name): Path<String>,
    Json(_payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = Row::new(RecordId::new());
    match state.engine.tables.insert_row(&table_name, row) {
        Ok(id) => Ok(Json(json!({"status": "inserted", "id": id.0.to_string()}))),
        Err(e) => Err(ApiError(e.to_string())),
    }
}

async fn update_row(
    State(_state): State<Arc<AppState>>,
    Path((table_name, row_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // TODO: Implement
    Ok(Json(json!({"status": "updated", "table": table_name, "id": row_id})))
}

async fn delete_row(
    State(state): State<Arc<AppState>>,
    Path((table_name, row_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // TODO: Parse UUID and delete
    Ok(Json(json!({"status": "deleted", "table": table_name, "id": row_id})))
}

// Document endpoints
async fn list_collections(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let collections = state.engine.documents.list_collections();
    Json(json!({"collections": collections}))
}

async fn create_collection(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    match state.engine.documents.create_collection(name.clone()) {
        Ok(_) => Ok(Json(json!({"status": "created", "collection": name}))),
        Err(e) => Err(ApiError(e.to_string())),
    }
}

async fn get_all_documents(
    State(state): State<Arc<AppState>>,
    Path(collection): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    match state.engine.documents.get_all_documents(&collection) {
        Ok(docs) => {
            let docs_json: Vec<_> = docs.iter().map(|d| {
                json!({
                    "id": d.id.0.to_string(),
                    "data": d.data,
                    "created_at": d.created_at.to_rfc3339(),
                })
            }).collect();
            Ok(Json(json!({"documents": docs_json})))
        }
        Err(e) => Err(ApiError(e.to_string())),
    }
}

async fn insert_document(
    State(state): State<Arc<AppState>>,
    Path(collection): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let doc = Document::new(RecordId::new(), payload);
    match state.engine.documents.insert_document(&collection, doc) {
        Ok(id) => Ok(Json(json!({"status": "inserted", "id": id.0.to_string()}))),
        Err(e) => Err(ApiError(e.to_string())),
    }
}

async fn get_document(
    State(state): State<Arc<AppState>>,
    Path((collection, _doc_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // TODO: Parse UUID and fetch
    Ok(Json(json!({"status": "ok", "collection": collection})))
}

async fn update_document(
    State(state): State<Arc<AppState>>,
    Path((collection, _doc_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(json!({"status": "updated", "collection": collection})))
}

async fn delete_document(
    State(state): State<Arc<AppState>>,
    Path((collection, _doc_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(json!({"status": "deleted", "collection": collection})))
}

// Graph endpoints
async fn list_graphs(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let graphs = state.engine.graphs.list_graphs();
    Json(json!({"graphs": graphs}))
}

async fn create_graph(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    match state.engine.graphs.create_graph(name.clone()) {
        Ok(_) => Ok(Json(json!({"status": "created", "graph": name}))),
        Err(e) => Err(ApiError(e.to_string())),
    }
}

async fn get_graph_nodes(
    State(state): State<Arc<AppState>>,
    Path(graph_name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    match state.engine.graphs.get_all_nodes(&graph_name) {
        Ok(nodes) => {
            let nodes_json: Vec<_> = nodes.iter().map(|n| {
                json!({
                    "id": n.id.0.to_string(),
                    "label": n.label,
                    "properties": n.properties,
                })
            }).collect();
            Ok(Json(json!({"nodes": nodes_json})))
        }
        Err(e) => Err(ApiError(e.to_string())),
    }
}

async fn add_node(
    State(state): State<Arc<AppState>>,
    Path(graph_name): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let label = payload.get("label")
        .and_then(|v| v.as_str())
        .unwrap_or("Node");

    let node = GraphNode::new(RecordId::new(), label.to_string());
    match state.engine.graphs.add_node(&graph_name, node) {
        Ok(id) => Ok(Json(json!({"status": "added", "id": id.0.to_string()}))),
        Err(e) => Err(ApiError(e.to_string())),
    }
}

async fn get_graph_edges(
    State(state): State<Arc<AppState>>,
    Path(graph_name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    match state.engine.graphs.get_all_edges(&graph_name) {
        Ok(edges) => {
            let edges_json: Vec<_> = edges.iter().map(|e| {
                json!({
                    "id": e.id.0.to_string(),
                    "from": e.from_node.0.to_string(),
                    "to": e.to_node.0.to_string(),
                    "relation": e.relation_type,
                })
            }).collect();
            Ok(Json(json!({"edges": edges_json})))
        }
        Err(e) => Err(ApiError(e.to_string())),
    }
}

async fn add_edge(
    State(state): State<Arc<AppState>>,
    Path(graph_name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // TODO: Parse from/to/relation from payload
    Ok(Json(json!({"status": "added"})))
}

async fn get_node_neighbors(
    State(state): State<Arc<AppState>>,
    Path((graph_name, _node_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(json!({"neighbors": []})))
}

// Vector endpoints
async fn list_vector_collections(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let collections = state.engine.vectors.list_collections();
    Json(json!({"collections": collections}))
}

async fn create_vector_collection(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    match state.engine.vectors.create_collection(name.clone()) {
        Ok(_) => Ok(Json(json!({"status": "created", "collection": name}))),
        Err(e) => Err(ApiError(e.to_string())),
    }
}

async fn search_vectors(
    State(state): State<Arc<AppState>>,
    Path(collection): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let query_vec = payload.get("query")
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter().map(|v| v.as_f64()).collect::<Option<Vec<_>>>()
        })
        .map(|v| v.iter().map(|&x| x as f32).collect::<Vec<_>>())
        .ok_or_else(|| ApiError("Invalid query vector".to_string()))?;

    let k = payload.get("k").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

    match state.engine.vectors.search_neighbors(&collection, &query_vec, k) {
        Ok(results) => {
            let results_json: Vec<_> = results.iter().map(|(id, dist)| {
                json!({"id": id.0.to_string(), "distance": dist})
            }).collect();
            Ok(Json(json!({"results": results_json})))
        }
        Err(e) => Err(ApiError(e.to_string())),
    }
}

// Index operations
async fn list_table_indices(
    State(state): State<Arc<AppState>>,
    Path(table): Path<String>,
) -> Json<serde_json::Value> {
    let indices = state.indexing.list_indices(&table);
    Json(json!({"indices": indices}))
}

async fn create_index(
    State(state): State<Arc<AppState>>,
    Path((table, column)): Path<(String, String)>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let index_type = payload.get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("btree");

    match index_type {
        "hash" => {
            state.indexing.create_hash_index(table, column)
                .map_err(|e| ApiError(e.to_string()))?;
            Ok(Json(json!({"status": "created", "type": "hash"})))
        }
        _ => {
            state.indexing.create_btree_index(table, column)
                .map_err(|e| ApiError(e.to_string()))?;
            Ok(Json(json!({"status": "created", "type": "btree"})))
        }
    }
}

async fn cache_stats(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let (used, max) = state.indexing.cache_stats();
    Json(json!({"cache": {"used": used, "max": max}}))
}

async fn clear_cache(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    state.indexing.clear_cache();
    Json(json!({"status": "cache cleared"}))
}
