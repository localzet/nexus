/// ═══════════════════════════════════════════════════════════════════════════════
/// NEXUS DB - Multi-Model Database Server
/// NQL Protocol: Unified JSON-based command interface
/// ═══════════════════════════════════════════════════════════════════════════════

use nexus_db::{
    MultiModelEngine, ColumnDef, ColumnType, Row, Value, RecordId, Document, 
    GraphNode, GraphEdge, VectorRecord, QueryExecutor, IndexingManager,
};
use nexus_db::protocol::{create_router, AppState, TcpServer};
use std::sync::Arc;
use axum::Router;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // Инициализируем компоненты
    let engine = Arc::new(MultiModelEngine::new());
    let indexing = Arc::new(IndexingManager::new());
    let query_executor = Arc::new(QueryExecutor::new(engine.clone()));

    println!("Инициализация...");
    
    // Demonstrate multi-model capabilities
    demo_all_phases(engine.clone(), indexing.clone()).await;

    // NQL server initialization (unified protocol on port 5433)
    let nql_server = TcpServer::new(
        engine.clone(),
        indexing.clone(),
        query_executor.clone(),
        "0.0.0.0:5433".to_string(),
    );

    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║          🚀 NEXUS DB Server запущен                          ║");
    println!("║                                                               ║");
    println!("║  NQL Protocol: 0.0.0.0:5433 (TCP)                           ║");
    println!("║  Unified JSON command interface                              ║");
    println!("║                                                               ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    // Запускаем NQL сервер
    let tcp_handle = tokio::spawn(async move {
        nql_server.start().await.unwrap_or_else(|e| {
            eprintln!("NQL Server error: {}", e);
        });
    });
}

async fn demo_all_phases(engine: Arc<MultiModelEngine>, indexing: Arc<IndexingManager>) {
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 NexusDB Demo: Storage, SQL, & Advanced Features");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Relational tables demo
    println!("\n  Creating table 'users'...");
    let users_schema = vec![
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
        ColumnDef {
            name: "email".to_string(),
            column_type: ColumnType::String,
            nullable: true,
            default: None,
        },
    ];

    if let Err(e) = engine.tables.create_table("users".to_string(), users_schema) {
        eprintln!("  Ошибка создания таблицы: {}", e);
        return;
    }
    println!("    ✓ Таблица 'users' создана");

    // Insert some data
    let mut row1 = Row::new(RecordId::new());
    row1.insert("name".to_string(), Value::String("Alice".to_string()));
    row1.insert("email".to_string(), Value::String("alice@db.com".to_string()));
    
    let mut row2 = Row::new(RecordId::new());
    row2.insert("name".to_string(), Value::String("Bob".to_string()));
    row2.insert("email".to_string(), Value::String("bob@db.com".to_string()));

    engine.tables.insert_row("users", row1).ok();
    engine.tables.insert_row("users", row2).ok();
    println!("    ✓ Вставлено 2 строки");

    // Indexing demo
    println!("\n  Creating indices...");
    indexing.create_btree_index("users".to_string(), "name".to_string()).ok();
    indexing.create_hash_index("users".to_string(), "email".to_string()).ok();
    println!("    ✓ B-tree индекс по 'name'");
    println!("    ✓ Hash индекс по 'email'");

    // Create blooms for fast negation
    indexing.create_bloom_filter("users".to_string(), "name".to_string(), 1000).ok();
    println!("    ✓ Bloom filter для 'name'");

    // Document storage demo
    println!("\n  Creating document collection...");
    engine.documents.create_collection("posts".to_string()).ok();
    let doc = Document::new(
        RecordId::new(),
        serde_json::json!({
            "title": "NexusDB Demo",
            "author": "Alice",
            "likes": 100
        }),
    );
    engine.documents.insert_document("posts", doc).ok();
    println!("    ✓ Коллекция создана, 1 документ вставлен");

    // Graph database demo
    println!("\n  Creating graph...");
    engine.graphs.create_graph("social".to_string()).ok();
    
    let mut node1 = GraphNode::new(RecordId::new(), "Person".to_string());
    node1.properties.insert("name".to_string(), Value::String("Alice".to_string()));
    
    let mut node2 = GraphNode::new(RecordId::new(), "Person".to_string());
    node2.properties.insert("name".to_string(), Value::String("Bob".to_string()));

    let n1 = engine.graphs.add_node("social", node1).ok();
    let n2 = engine.graphs.add_node("social", node2).ok();

    if let (Some(id1), Some(id2)) = (n1, n2) {
        let edge = GraphEdge::new(id1, id2, "KNOWS".to_string());
        engine.graphs.add_edge("social", edge).ok();
    }
    println!("    ✓ Граф создан: 2 узла, 1 связь");

    // Vector storage demo
    println!("\n  Creating vector collection...");
    engine.vectors.create_collection("embeddings".to_string()).ok();
    
    for (word, vec) in vec![
        ("computer", vec![0.9, 0.1, 0.0]),
        ("laptop", vec![0.85, 0.15, 0.0]),
        ("mouse", vec![0.2, 0.7, 0.1]),
    ] {
        let record = VectorRecord {
            id: RecordId::new(),
            vector: vec,
            metadata: serde_json::json!({"word": word}),
        };
        engine.vectors.insert_vector("embeddings", record).ok();
    }
    println!("    ✓ Вставлено 3 вектора");

    let results = engine.vectors.search_neighbors(
        "embeddings",
        &[0.88, 0.12, 0.0],
        2
    ).unwrap_or_default();
    println!("    ✓ Поиск соседей: {} результатов", results.len());

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Demo завершен. Статистика:");
    println!("  • Таблиц: {}", engine.tables.list_tables().len());
    println!("  • Коллекций документов: {}", engine.documents.list_collections().len());
    println!("  • Графов: {}", engine.graphs.list_graphs().len());
    println!("  • Векторных коллекций: {}", engine.vectors.list_collections().len());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
}
