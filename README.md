# NexusDB — Многомодельная СУБД нового поколения

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Tests: 317 passing](https://img.shields.io/badge/tests-317%20passing-brightgreen)](#testing)
[![Version: 1.0.0](https://img.shields.io/badge/version-1.0.0-blue.svg)](https://github.com/localzet/nexus/releases)

**Современная база данных для приложений, которым нужны SQL + JSON + графы + векторы в одной системе**

NexusDB — это полнофункциональная многомодельная СУБД, позволяющая работать со всеми типами данных без необходимости синхронизировать несколько отдельных сервисов. Написана на Rust для максимальной произв

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
# На Docker (проще всего)
docker run -d --name nexus -p 5433:5433 ghcr.io/localzet/nexus:latest

# Или из исходников
git clone https://github.com/localzet/nexus.git
cd nexus && cargo build --release
./target/release/nexus
```

Сервер запустится на **порту 5433** с NQL протоколом.

## 📚 Документация

**Quick Links:**
- [NQL Specification](./docs/NQL-specification.md) — Полный протокол (JSON commands)
- [NQL Examples](./docs/NQL-EXAMPLES.md) — Примеры на Python, TypeScript, PHP  
- [Client Libraries](./clients/README.md) — Python, TypeScript, PHP SDKs

**Architecture & Design:**
- [System Architecture](./docs/architecture-ru.md) — Как устроена NexusDB
- [MVCC Transactions](./docs/MVCC.md) — Параллельные транзакции ACID
- [Raft Replication](./docs/Raft.md) — Консенсус и кластеризация

## 💡 Примеры использования

### Python Client

```python
from nexus import NexusClient

client = NexusClient("localhost", 5433)

# Create table
client.create_table("users", [
    ("id", "integer", {"primary_key": true}),
    ("name", "varchar", {"length": 100}),
    ("email", "varchar", {"length": 100})
])

# Insert
client.insert("users", {"id": 1, "name": "Alice", "email": "alice@example.com"})

# Select
result = client.select("users", where={"field": "id", "eq": 1})
print(result.rows)
```

### TypeScript Client

```typescript
import { NexusClient } from "nexus-db";

const client = new NexusClient("localhost", 5433);

const result = await client.select(
    "users",
    ["id", "name", "email"],
    { field: "id", eq: 1 }
);
console.log(result.rows);
```

### PHP Client

```php
use NexusDB\NexusClient;

$client = new NexusClient("localhost", 5433);

$result = $client->select(
    "users",
    columns: ["id", "name", "email"],
    where: ["field" => "id", "eq" => 1]
);
foreach ($result->rows as $row) {
    echo $row["name"];
}
```

## 📊 Архитектура

```
┌─── NQL Protocol (Port 5433) ────┐
│  JSON-based command interface    │
│  TCP Native Connection           │
└────────────────────────────────┘
         ↓
┌─ Query Processing ────────────┐
│ Parser • Optimizer • Executor  │
├────────────────────────────────┤
│ SELECT • INSERT • UPDATE        │
│ Transactions • Aggregations     │
│ Graphs • Vectors • Full-text    │
└────────────────────────────────┘
         ↓
┌─ Storage Layer ───────────────┐
│ MVCC • Transactions • Cache    │
│ B-Tree Indexes • WAL           │
└────────────────────────────────┘
         ↓
┌─ Distribution (Optional) ─────┐
│ Raft Consensus • Replication   │
│ Sharding • Failover            │
└────────────────────────────────┘
```

**Single Protocol:** NQL (JSON) — всё работает через единый TCP порт 5433

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
