# Введение в NexusDB

## Что такое NexusDB?

NexusDB — это полнофункциональная многомодельная система управления базами данных (СУБД), написанная на Rust. Она обеспечивает:

- **Высокую производительность** через MVCC (Multi-Version Concurrency Control)
- **Надёжность** благодаря Raft консенсусу и репликации
- **Продвинутые возможности** SQL с window функциями, CTE и оптимизацией запросов
- **Масштабируемость** через sharding и распределённые вычисления
- **Профессиональный мониторинг** с метриками и health checks

## Быстрый старт

### Установка

```bash
git clone https://github.com/localzet/nexus.git
cd nexus
cargo build --release
```

### Первый запуск

```rust
use nexus_db::{MultiModelEngine, QueryExecutor};

fn main() -> anyhow::Result<()> {
    // Инициализация движка
    let mut engine = MultiModelEngine::new();
    
    // Создание таблицы
    engine.create_table("users", vec!["id", "name", "email"])?;
    
    // Вставка данных
    engine.insert("users", vec![
        vec!["1".into(), "Alice".into(), "alice@example.com".into()],
        vec!["2".into(), "Bob".into(), "bob@example.com".into()],
    ])?;
    
    // Выполнение запроса
    let results = engine.execute("SELECT * FROM users WHERE id > 0")?;
    println!("Результаты: {:?}", results);
    
    Ok(())
}
```

## Основные компоненты

### Query Processing
Полная поддержка SQL:
- SELECT с WHERE, GROUP BY, ORDER BY
- JOINs (INNER, LEFT, RIGHT, FULL, CROSS)
- Aggregate functions (COUNT, SUM, AVG, MIN, MAX)
- Window functions (ROW_NUMBER, RANK, DENSE_RANK, LAG, LEAD)
- Common Table Expressions (CTEs)
- Subqueries

### Storage & Transactions
- MVCC для конкурентной обработки
- 4 уровня изоляции (READ UNCOMMITTED → SERIALIZABLE)
- Bloom Filter индексы
- B-Tree и Hash индексы
- Write-Ahead Logging (WAL)

### Distribution
- Raft консенсус для надёжной репликации
- Consistent hashing для sharding
- Failover и load balancing
- Geo-distributed replication

### Performance
- LRU кеширование
- Buffer pool с управлением страницами
- Query result caching
- Полнотекстовый поиск с BM25

## Архитектура

```
┌─ Query Layer ──────────┐
│ Parser → Executor      │
│ Optimizer → Planner    │
└────────────────────────┘
         ↓
┌─ Storage Layer ────────┐
│ MVCC → Transactions    │
│ Indexes → Cache        │
└────────────────────────┘
         ↓
┌─ Distribution Layer ───┐
│ Raft → Sharding        │
│ Replication → Failover │
└────────────────────────┘
```

## Документация

- **[Полная архитектура](./architecture-ru.md)** — Детальное описание всех компонентов
- **[API Reference](./api-reference-ru.md)** — Справочник функций и типов
- **[Примеры](./examples-ru.md)** — Готовые решения для типичных задач
- **[Производительность](./performance-ru.md)** — Оптимизация и бенчмарки

## Примеры использования

### Window Functions

```sql
SELECT 
    name, 
    salary,
    ROW_NUMBER() OVER (ORDER BY salary DESC) as rank,
    LAG(salary) OVER (ORDER BY salary DESC) as prev_salary
FROM employees;
```

### CTEs (Recursive)

```sql
WITH RECURSIVE org_tree AS (
    SELECT id, name, manager_id, 1 as level
    FROM employees 
    WHERE manager_id IS NULL
    
    UNION ALL
    
    SELECT e.id, e.name, e.manager_id, t.level + 1
    FROM employees e
    JOIN org_tree t ON e.manager_id = t.id
)
SELECT * FROM org_tree;
```

### Transactions

```rust
let tx = engine.begin_transaction(IsolationLevel::Serializable)?;
tx.insert("users", row)?;
tx.commit()?;
```

## Требования

- Rust 1.70 или выше
- Cargo
- Linux/macOS/Windows

## Тестирование

```bash
# Все тесты
cargo test --lib

# Конкретный модуль
cargo test --lib mvcc
cargo test --lib window_functions

# С выводом
cargo test --lib -- --nocapture
```

## Лицензия

GNU Affero General Public License v3.0 — смотрите [LICENSE](../LICENSE) для деталей.

## Автор

Ivan Zorin (localzet) — creator@localzet.com

---

Дополнительная помощь: https://github.com/localzet/nexus/issues
