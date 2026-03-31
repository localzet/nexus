# NexusDB

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Build Status](https://img.shields.io/badge/tests-317%20passing-brightgreen)](#testing)
[![Rust 1.70+](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)

Полнофункциональная многомодельная СУБД на Rust с поддержкой MVCC, Raft консенсуса и распределённых вычислений.

**English:** [README.md](./docs/guide-en.md)

## ✨ Возможности

- **SQL Query Engine** — полная поддержка SELECT, JOIN, GROUP BY, Window Functions, CTEs
- **MVCC Transactions** — конкурентная обработка без глобальных блокировок
- **High Availability** — Raft консенсус с автоматическим failover
- **Distributed** — Consistent hashing sharding и geo-replication
- **Performance** — LRU кеширование, индексирование, query optimization
- **Full-Text Search** — инвертированный индекс с BM25 scoring
- **Monitoring** — метрики, health checks, slow query log

## 🚀 Быстрый старт

```bash
# Клонирование репозитория
git clone https://github.com/localzet/nexus.git
cd nexus

# Сборка
cargo build --release

# Тестирование
cargo test --lib
```

## 📚 Документация

- [Руководство пользователя](./docs/guide-ru.md)
- [Архитектура системы](./docs/architecture-ru.md)
- [API Справочник](./docs/api-reference-ru.md)
- [Примеры использования](./docs/examples-ru.md)
- [Производительность](./docs/performance-ru.md)

**English Documentation:**
- [User Guide](./docs/guide-en.md)
- [System Architecture](./docs/architecture-en.md)
- [API Reference](./docs/api-reference-en.md)

## 💡 Примеры

### Базовое использование

```rust
use nexus_db::{MultiModelEngine, QueryExecutor};

fn main() -> anyhow::Result<()> {
    let mut engine = MultiModelEngine::new();
    
    // Создание таблицы
    engine.create_table("users", vec!["id", "name", "email"])?;
    
    // Вставка данных
    engine.insert("users", vec![
        vec!["1".into(), "Alice".into(), "alice@example.com".into()],
    ])?;
    
    // Запрос
    let results = engine.execute("SELECT * FROM users WHERE id = 1")?;
    
    Ok(())
}
```

### Window Functions

```sql
SELECT 
    name, 
    salary,
    ROW_NUMBER() OVER (ORDER BY salary DESC) as rank
FROM employees;
```

### Recursive CTEs

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
SELECT * FROM org_tree WHERE level <= 5;
```

## 📊 Архитектура

```
┌─ Query Processing ─────────┐
│ Parser • Executor • Optimizer │
└────────────────────────────┘
         ↓
┌─ Storage Layer ────────────┐
│ MVCC • Transactions • Cache   │
│ Indexes • WAL                │
└────────────────────────────┘
         ↓
┌─ Distribution Layer ───────┐
│ Raft • Sharding • Replication │
└────────────────────────────┘
```

## 🔧 Функциональность

### Query Processing
- ✅ SELECT with WHERE, GROUP BY, ORDER BY, LIMIT
- ✅ JOINs: INNER, LEFT, RIGHT, FULL, CROSS
- ✅ Aggregates: COUNT, SUM, AVG, MIN, MAX
- ✅ Window Functions: ROW_NUMBER, RANK, DENSE_RANK, LAG, LEAD, FIRST_VALUE, LAST_VALUE
- ✅ CTEs: Simple и Recursive
- ✅ Subqueries: scalar, correlated, EXISTS, IN
- ✅ INSERT, UPDATE, DELETE

### Storage & Concurrency
- ✅ MVCC versioning
- ✅ 4 isolation levels (READ UNCOMMITTED → SERIALIZABLE)
- ✅ Lock management (Shared/Exclusive)
- ✅ Conflict detection
- ✅ Write-Ahead Logging (WAL)
- ✅ Multi-version visibility

### Indexing
- ✅ B-Tree (ordered, range queries)
- ✅ Hash Index (exact match)
- ✅ Bloom Filter (probabilistic filtering)

### Distribution
- ✅ Raft consensus
- ✅ Consistent hashing
- ✅ Range/Hash/List partitioning
- ✅ Automatic failover

### Performance
- ✅ LRU Cache
- ✅ Buffer Pool
- ✅ Query Result Caching
- ✅ Full-Text Search (Inverted Index + BM25)
- ✅ Query Optimization

## 📈 Производительность

| Операция | Сложность | Примечания |
|----------|-----------|-----------|
| Point SELECT (индекс) | O(1) | Hash index |
| Range SELECT | O(log n) | B-Tree index |
| JOIN | O(n) | С индексом |
| GROUP BY | O(n log n) | Сортирующий алгоритм |
| Window Func | O(n²) | Худший случай |

Подробные бенчмарки см. в [performance-ru.md](./docs/performance-ru.md).

## 🧪 Тестирование

```bash
# Все тесты
cargo test --lib

# Конкретный модуль
cargo test --lib mvcc
cargo test --lib window_functions
cargo test --lib sharding

# С выводом результатов
cargo test --lib -- --nocapture

# Один конкретный тест
cargo test --lib test_mvcc_versioning -- --exact
```

**Статистика:** 317 тестов, все проходят, 0 errors

## 📋 Требования

- **Rust** 1.70 или выше
- **OS**: Linux, macOS, Windows
- **RAM**: минимум 512 MB

## 🤝 Внесение вклада

[CONTRIBUTING.md](./CONTRIBUTING.md) содержит подробные инструкции.

### Основные направления для разработки

- **Performance** — Query plan caching, parallel execution
- **Features** — Columnar storage, advanced analytics
- **Enterprise** — RBAC, data masking, audit logging

## 📄 Лицензия

GNU Affero General Public License v3.0 — смотрите [LICENSE](./LICENSE) для полного текста.

Это означает, что использование этого ПО в сетевых приложениях требует открытия исходного кода.

## 👤 Автор

**Ivan Zorin** (localzet)
- Email: creator@localzet.com
- GitHub: https://github.com/localzet
- Website: https://localzet.com

## 🔗 Ссылки

- **GitHub**: https://github.com/localzet/nexus
- **Issues**: https://github.com/localzet/nexus/issues
- **Discussions**: https://github.com/localzet/nexus/discussions
- **Docs**: [./docs](./docs)

## 🎯 Дорожная карта

### v1.0.0 ✅
- Multi-model storage (Tables, JSON, Graphs, Vectors)
- SQL query engine
- MVCC transactions
- Raft replication
- Window functions
- CTEs & subqueries
- Full-text search
- Query optimization

### v1.1.0 (Планируется)
- Query plan caching
- Parallel query execution
- Columnar storage
- Extended window functions
- Performance improvements

### v1.2.0 (Планируется)
- RBAC (Role-Based Access Control)
- Data masking
- Audit logging
- Geo-replication

---

Спасибо за использование NexusDB! 🚀

Если у вас есть вопросы — открывайте [issue](https://github.com/localzet/nexus/issues) или [discussion](https://github.com/localzet/nexus/discussions).
