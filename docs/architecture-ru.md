# Архитектура NexusDB

## Общий обзор

NexusDB состоит из нескольких слоёв, каждый отвечает за определённые функции:

```
Приложение
    ↓
┌──────────────────────────────┐
│   Query Processing Layer     │
│ (Parser, Executor, Optimizer)│
└──────────────────────────────┘
    ↓
┌──────────────────────────────┐
│    Storage & Index Layer     │
│ (MVCC, Transactions, Cache)  │
└──────────────────────────────┘
    ↓
┌──────────────────────────────┐
│  Distribution & HA Layer     │
│  (Raft, Sharding, Replication│
└──────────────────────────────┘
    ↓
  Хранилище (In-Memory / Disk)
```

## 1. Query Processing Layer

### 1.1 SQL Parser
- Парсирует SQL согласно диалекту PostgreSQL
- Поддерживает SELECT, INSERT, UPDATE, DELETE, CREATE TABLE
- Обрабатывает сложные выражения и операторы

### 1.2 Query Executor
Основной компонент выполнения запросов:

**WHERE Evaluation**
```rust
// Вычисление условий с типизацией
pub fn evaluate_where(&self, row: &Row, condition: &Expr) -> Result<bool>
```

**JOINs**
- INNER JOIN: только совпадающие строки
- LEFT JOIN: все из левой таблицы
- RIGHT JOIN: все из правой таблицы
- FULL JOIN: все из обеих таблиц
- CROSS JOIN: декартово произведение

**Aggregates**
- COUNT, SUM, AVG, MIN, MAX
- Работает с GROUP BY и HAVING

### 1.3 Window Functions
Аналитические функции для расширенного анализа:

```sql
ROW_NUMBER()   -- Порядковый номер в партиции
RANK()         -- Ранг с пропусками
DENSE_RANK()   -- Плотный ранг
LAG()          -- Предыдущая строка
LEAD()         -- Следующая строка
FIRST_VALUE()  -- Первое значение в окне
LAST_VALUE()   -- Последнее значение в окне
```

### 1.4 CTEs (Common Table Expressions)
- Простые CTEs с материализацией
- Рекурсивные CTEs с обнаружением циклов
- Оптимизированное кеширование результатов

### 1.5 Query Optimizer
Основан на статистике и селективности:

```
Сбор статистики
    ↓
Построение гистограмм
    ↓
Оценка селективности
    ↓
Переупорядочение предикатов
    ↓
Рекомендации индексов
```

## 2. Storage & Index Layer

### 2.1 MVCC (Multi-Version Concurrency Control)
Механизм для конкурентного доступа без глобальных блокировок:

```
Row ID 1 → [Version 1] → [Version 2] → [Version 3]
           (TxID 100)    (TxID 150)    (TxID 200)
```

**Версионные цепочки:**
- Каждая строка имеет цепочку версий
- Каждая версия помечена ID транзакции
- Используется для определения видимости данных

### 2.2 Transaction Management
```rust
pub enum IsolationLevel {
    ReadUncommitted,   // Грязное чтение разрешено
    ReadCommitted,     // Видны только коммитенные данные
    RepeatableRead,    // Снимок изолирован от других
    Serializable,      // Полная изоляция
}
```

**Lock Manager:**
- Shared Lock (чтение): несколько читателей
- Exclusive Lock (запись): один автор, никого больше

**Conflict Detection:**
- Граф конфликтов через DFS
- Обнаружение циклов = deadlock

### 2.3 Write-Ahead Logging (WAL)
- Все изменения логируются перед применением
- Гарантирует durability при сбого
- Позволяет восстановление после сбоя

### 2.4 Индексирование

**B-Tree Index**
- Сбалансированные деревья поиска
- Эффективен для диапазонных запросов
- Используется для PRIMARY KEY

**Hash Index**
- Быстрый поиск по точному значению
- Оптимален для JOIN условий

**Bloom Filter**
- Вероятностная структура данных
- Быстро исключает неправильные значения

### 2.5 Caching

**LRU Cache**
```
Самые свежие ← → Самые старые
↑                ↓
Новые данные   Удаляются первыми
```

**Buffer Pool**
- Управление страницами памяти
- Pin/Unpin механизм
- Вытеснение LRU политикой

**Query Result Cache**
- Кеширование результатов запросов
- Потокобезопасное хранилище
- TTL-based invalidation

## 3. Distribution & HA Layer

### 3.1 Raft Consensus
Алгоритм для распределённого согласия:

```
Follower ──(election timeout)──> Candidate
   ↑                                  ↓
   └─────(win election)─────────> Leader
        └───(lost)─────────────────> Follower
```

**Состояния:**
- Follower: слушает leader
- Candidate: претендует на лозунство
- Leader: управляет кластером

**Log Replication:**
```
Leader → [Log Entry 1] ✓
       → [Log Entry 2] ✓
       → [Log Entry 3] ✓
           (кворум = большинство)
```

### 3.2 Sharding
Распределение данных по узлам:

**Consistent Hashing**
```
Node 1 (0°)
   ↗   ↖
Node 4   Node 2 (120°)
   ↖   ↗
Node 3 (240°)

Key hash → ближайший по часовой узел
```

**Range Partitioning**
```
Shard 1: Keys [A-L]
Shard 2: Keys [M-Z]
Shard 3: Keys [0-9]
```

**List Partitioning**
```
Shard 1: Country IN ('US', 'CA')
Shard 2: Country IN ('UK', 'FR')
Shard 3: Country IN (остальные)
```

### 3.3 Replication & Failover

**Node Health Monitoring:**
- Проверка heartbeat
- Отслеживание `last_heartbeat`
- Автоматическое понижение при сбое

**Failover Process:**
```
1. Leader не отвечает
2. Followers видят timeout
3. Кандидат запрашивает голоса
4. Большинство голосует
5. Новый leader становится active
```

## 4. Full-Text Search

### 4.1 Inverted Index
Обратный индекс от термина к документам:

```
"database" → [Doc1: [pos 1, 5], Doc2: [pos 3]]
"query"    → [Doc1: [pos 10], Doc3: [pos 2, 8]]
```

### 4.2 Tokenizer
- Приведение к нижнему регистру
- Удаление стоп-слов
- Porter stemmer для нормализации

```
"Running" → "run"
"databases" → "databas"
"the" → REMOVED
```

### 4.3 BM25 Scoring
Алгоритм релевантности:

```
Score = Σ IDF(term) * ((k1+1) * TF) / (TF + k1*(1-b+b*length_norm))

где:
  IDF = log(total_docs / docs_with_term)
  TF = term frequency in doc
  k1 = term frequency saturation (1.5)
  b = length normalization (0.75)
```

## 5. Monitoring & Observability

### 5.1 Query Metrics
```rust
pub struct QueryMetrics {
    query_id: String,
    query_text: String,
    execution_time_ms: u64,
    rows_scanned: u64,
    rows_returned: u64,
    index_used: Option<String>,
    full_scan: bool,
}
```

### 5.2 Slow Query Log
- Порог срабатывания (ms)
- Скользящее окно истории
- Анализ проблемных запросов

### 5.3 Health Check
```
CPU Usage: 45%
Memory: 1.2 GB / 8 GB
Connections: 42/100
Cache Hit Ratio: 94.2%
Response Time (avg): 12ms
```

## Data Flow

### Write Operation
```
SQL: INSERT INTO users VALUES (...)
  ↓
Parser → Executor → MVCC Manager
  ↓
Version Chain (add new version)
  ↓
WAL (write entry to log)
  ↓
Replication (send to followers)
  ↓
Commit (mark version as committed)
```

### Read Operation
```
SQL: SELECT * FROM users WHERE id > 10
  ↓
Parser → Executor
  ↓
Index Lookup (if available)
  ↓
MVCC Visibility Check (find right version)
  ↓
WHERE Evaluation
  ↓
Aggregation/Window Functions (if needed)
  ↓
Result Caching
  ↓
Return Results
```

### Query Optimization
```
Raw Query
  ↓
Statistics Collection
  ↓
Selectivity Estimation
  ↓
Predicate Ordering
  ↓
Join Order
  ↓
Index Selection
  ↓
Optimized Execution Plan
```

## Performance Characteristics

| Операция | Сложность | Примечания |
|----------|-----------|-----------|
| SELECT точное значение | O(1) | С индексом Hash |
| SELECT диапазон | O(log n) | С индексом B-Tree |
| JOINс индексом | O(n) | Nested loop + index |
| GROUP BY | O(n log n) | Сортировка |
| Window Function | O(n²) | В худшем случае |
| MVCC Read | O(1) | Быстрая версионная цепь |

---

Дополнительные детали см. в [API Reference](./api-reference-ru.md)
