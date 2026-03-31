# NQL — NexusDB Query Language Specification

**NQL** — это унифицированный язык команд для NexusDB, который:
- ✅ Работает одинаково в TCP и REST
- ✅ Объединяет SQL, JSON и граф операции в единую синтаксис
- ✅ Родной для JSON (легко парсить/сериализовать)
- ✅ Безопасный (нет SQL injection)

## Архитектура

NQL команда = JSON объект с полями:
```json
{
  "op": "operation_type",
  "table": "collection_name",
  "params": {...},
  "options": {...}
}
```

## Операции

### 1️⃣ SELECT (Query)

```json
{
  "op": "select",
  "from": "users",
  "columns": ["id", "name", "email"],
  "where": {
    "and": [
      {"field": "age", "gt": 25},
      {"field": "status", "eq": "active"}
    ]
  },
  "order_by": [
    {"field": "created_at", "desc": true}
  ],
  "limit": 10,
  "offset": 0
}
```

#### WHERE условия (логика фильтрации)

```json
{
  "where": {
    "or": [
      {"field": "age", "eq": 30},
      {"field": "age", "eq": 25},
      {
        "and": [
          {"field": "status", "eq": "active"},
          {"field": "verified", "eq": true}
        ]
      }
    ]
  }
}
```

**Операторы сравнения:**
- `"eq"` — equals (=)
- `"ne"` — not equals (!=)
- `"gt"` — greater than (>)
- `"gte"` — greater or equal (>=)
- `"lt"` — less than (<)
- `"lte"` — less or equal (<=)
- `"like"` — string pattern matching
- `"in"` — array membership
- `"exists"` — field existence (for JSON)

**Array/JSON navigation:**
```json
{
  "field": "profile.address.city",
  "eq": "Moscow"
}
```

### 2️⃣ INSERT

```json
{
  "op": "insert",
  "into": "users",
  "values": [
    {
      "id": 1,
      "name": "Alice",
      "email": "alice@example.com",
      "profile": {
        "age": 30,
        "city": "Moscow"
      }
    },
    {
      "id": 2,
      "name": "Bob",
      "email": "bob@example.com"
    }
  ]
}
```

### 3️⃣ UPDATE

```json
{
  "op": "update",
  "table": "users",
  "set": {
    "status": "inactive",
    "updated_at": "2026-03-31T12:00:00Z"
  },
  "where": {
    "field": "id",
    "eq": 1
  }
}
```

### 4️⃣ DELETE

```json
{
  "op": "delete",
  "from": "users",
  "where": {
    "field": "id",
    "eq": 1
  }
}
```

### 5️⃣ CREATE TABLE

```json
{
  "op": "create_table",
  "name": "users",
  "columns": [
    {"name": "id", "type": "integer", "primary_key": true},
    {"name": "name", "type": "string", "nullable": false},
    {"name": "email", "type": "string", "nullable": true},
    {"name": "profile", "type": "json"},
    {"name": "created_at", "type": "timestamp"}
  ]
}
```

### 6️⃣ TRANSACTION

```json
{
  "op": "transaction",
  "isolation_level": "serializable",
  "commands": [
    {
      "op": "insert",
      "into": "accounts",
      "values": [{"id": 1, "balance": 1000}]
    },
    {
      "op": "update",
      "table": "transactions",
      "set": {"status": "completed"},
      "where": {"field": "id", "eq": 100}
    }
  ]
}
```

### 7️⃣ AGGREGATION

```json
{
  "op": "select",
  "from": "users",
  "aggregate": [
    {"func": "count", "as": "total"},
    {"func": "avg", "field": "age", "as": "avg_age"},
    {"func": "max", "field": "salary", "as": "max_salary"}
  ],
  "group_by": ["department"],
  "having": {
    "field": "total",
    "gt": 5
  }
}
```

### 8️⃣ JSON DOCUMENT (Multi-model collection)

```json
{
  "op": "insert",
  "collection": "posts",
  "document": {
    "title": "NexusDB Guide",
    "author": "Alice",
    "tags": ["database", "sql"],
    "metadata": {
      "version": 1,
      "draft": false
    }
  }
}
```

**Search в документах:**
```json
{
  "op": "search",
  "collection": "posts",
  "query": {
    "field": "author",
    "eq": "Alice"
  },
  "fulltext": {
    "field": "content",
    "match": "database"
  }
}
```

### 9️⃣ GRAPH OPERATIONS

**Добавить узел:**
```json
{
  "op": "graph_add_node",
  "graph": "social_network",
  "node": {
    "id": "user_1",
    "label": "Person",
    "properties": {
      "name": "Alice",
      "age": 30
    }
  }
}
```

**Добавить связь:**
```json
{
  "op": "graph_add_edge",
  "graph": "social_network",
  "edge": {
    "from": "user_1",
    "to": "user_2",
    "label": "KNOWS",
    "properties": {"since": 2020}
  }
}
```

**Поиск пути:**
```json
{
  "op": "graph_traverse",
  "graph": "social_network",
  "start": "user_1",
  "end": "user_5",
  "max_depth": 3
}
```

### 🔟 VECTOR OPERATIONS

**Добавить вектор:**
```json
{
  "op": "vector_insert",
  "collection": "embeddings",
  "vector": {
    "text": "computer",
    "embedding": [0.1, 0.2, 0.3, ...],
    "metadata": {"language": "en"}
  }
}
```

**Поиск похожих (KNN):**
```json
{
  "op": "vector_search",
  "collection": "embeddings",
  "query_vector": [0.15, 0.25, 0.35, ...],
  "k": 5,
  "threshold": 0.7
}
```

## Типы данных

| Тип | JSON | Описание |
|-----|------|---------|
| `integer` | `{"type": "integer"}` | Int64 |
| `string` | `{"type": "string"}` | UTF-8 строка |
| `real` | `{"type": "real"}` | Float64 |
| `boolean` | `{"type": "boolean"}` | true/false |
| `json` | `{"type": "json"}` | Встроенный JSON объект |
| `timestamp` | `{"type": "timestamp"}` | ISO 8601 |
| `blob` | `{"type": "blob"}` | Binary data |
| `vector` | `{"type": "vector", "dim": 300}` | Вектор для ML |

## Обработка ошибок

Все ошибки возвращаются в единую структуру:

```json
{
  "success": false,
  "error": {
    "code": "TABLE_NOT_FOUND",
    "message": "Table 'users' does not exist",
    "context": {
      "table": "users",
      "operation": "select"
    }
  }
}
```

**Коды ошибок:**
- `TABLE_NOT_FOUND` — таблица не существует
- `COLUMN_NOT_FOUND` — колонка не существует
- `INVALID_SYNTAX` — неправильный формат команды
- `TYPE_MISMATCH` — несовместимость типов
- `CONSTRAINT_VIOLATION` — нарушение ограничения
- `TRANSACTION_CONFLICT` — конфликт транзакции
- `PERMISSION_DENIED` — отсутствует доступ

## Успешный ответ

```json
{
  "success": true,
  "result": {
    "rows": [...],
    "columns": [...],
    "row_count": 10,
    "execution_time_ms": 25.5
  }
}
```

## Протокольная интеграция

### TCP Protocol

NQL отправляется как JSON-строка, заканчивающаяся `\n`:

```
TCP Client           →  Server
{"op":"select","from":"users"}\n
                     ←  {"success":true,"result":{...}}\n
```

### REST API

**POST /query** с JSON body:

```bash
curl -X POST http://localhost:8080/query \
  -H "Content-Type: application/json" \
  -d '{
    "op": "select",
    "from": "users",
    "limit": 10
  }'
```

## Пример: E-commerce сценарий

```json
{
  "op": "transaction",
  "isolation_level": "serializable",
  "commands": [
    {
      "op": "select",
      "from": "products",
      "columns": ["id", "price"],
      "where": {
        "field": "id",
        "eq": 123
      }
    },
    {
      "op": "insert",
      "into": "orders",
      "values": [{
        "user_id": 1,
        "product_id": 123,
        "quantity": 2,
        "total": 999.98
      }]
    },
    {
      "op": "update",
      "table": "products",
      "set": {"stock": 98},
      "where": {"field": "id", "eq": 123}
    }
  ]
}
```

## Версионирование

Текущая версия: **NQL v1.0**

Добавить в каждый запрос для future-compatibility:
```json
{
  "nql_version": "1.0",
  "op": "select",
  "from": "users"
}
```

## Безопасность

- ✅ **Параметризованные значения** — no SQL injection
- ✅ **Type safety** — значения валидируются против схемы
- ✅ **Field path validation** — запрет на обход директорий in JSON fields
