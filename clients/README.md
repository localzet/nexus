# NexusDB Client Libraries

Официальные клиентские библиотеки для NexusDB с поддержкой **NQL** (NexusDB Query Language) — унифицированного языка команд.

## Что такое NQL?

**NQL** — это JSON-based язык команд, обеспечивающий:
- ✅ **Единое API** для всех сценариев: SQL + JSON + Graphs + Vectors
- ✅ **Безопасность** — параметризованные запросы без SQL injection
- ✅ **Простоту** — натур JSON, легко парсить и использовать
- ✅ **Мощность** — поддержка транзакций, агрегации, window functions

📚 Полная спецификация: [NQL-specification.md](../NQL-specification.md)

## Установка

### Python

```bash
pip install nexus-db
```

```python
from nexus import NexusClient

client = NexusClient("localhost:5433")
result = client.select("users", where={"field": "age", "gt": 25})
for row in result.rows:
    print(row)
```

### TypeScript/JavaScript

```bash
npm install nexus-db
```

```typescript
import { NexusClient } from "nexus-db";

const client = new NexusClient("localhost", 5433);
const result = await client.select("users", undefined, 
  { field: "age", gt: 25 }
);
console.log(result.row_count, "users found");
```

### PHP

```bash
composer require nexus-db/php
```

```php
use NexusDB\NexusClient;

$client = new NexusClient('localhost', 5433);
$result = $client->select('users', where: ['field' => 'age', 'gt' => 25]);
foreach ($result->rows as $row) {
    echo $row['name'];
}
```

## Примеры использования

### Базовые операции

#### SELECT

```python
# Python
result = client.select(
    "users",
    columns=["id", "name", "email"],
    where={"field": "age", "gt": 25},
    limit=10
)
```

```typescript
// TypeScript
const result = await client.select(
  "users",
  ["id", "name", "email"],
  { field: "age", gt: 25 },
  10
);
```

```php
// PHP
$result = $client->select('users', 
  columns: ['id', 'name', 'email'],
  where: ['field' => 'age', 'gt' => 25],
  limit: 10
);
```

#### INSERT

```python
client.insert("users", {
    "id": 1,
    "name": "Alice",
    "email": "alice@example.com"
})
```

```typescript
await client.insert("users", {
    id: 1,
    name: "Alice",
    email: "alice@example.com"
});
```

```php
$client->insert('users', [
    'id' => 1,
    'name' => 'Alice',
    'email' => 'alice@example.com'
]);
```

#### UPDATE

```python
client.update(
    "users",
    {"status": "active"},
    {"field": "id", "eq": 1}
)
```

```typescript
await client.update(
    "users",
    { status: "active" },
    { field: "id", eq: 1 }
);
```

```php
$client->update('users',
    ['status' => 'active'],
    ['field' => 'id', 'eq' => 1]
);
```

#### DELETE

```python
client.delete("users", {"field": "id", "eq": 1})
```

```typescript
await client.delete("users", { field: "id", eq: 1 });
```

```php
$client->delete('users', ['field' => 'id', 'eq' => 1]);
```

### Транзакции

#### Python

```python
with client.transaction(isolation="serializable"):
    client.insert("accounts", {"id": 1, "balance": 1000})
    client.insert("accounts", {"id": 2, "balance": 500})
    client.update(
        "accounts",
        {"balance": 1000 - 100},
        {"field": "id", "eq": 1}
    )
    client.update(
        "accounts",
        {"balance": 500 + 100},
        {"field": "id", "eq": 2}
    )
```

#### TypeScript

```typescript
await client.transaction(async (cli) => {
    await cli.insert("accounts", { id: 1, balance: 1000 });
    await cli.insert("accounts", { id: 2, balance: 500 });
    await cli.update("accounts", { balance: 900 }, { field: "id", eq: 1 });
    await cli.update("accounts", { balance: 600 }, { field: "id", eq: 2 });
}, "serializable");
```

#### PHP

```php
$client->transaction(function($cli) {
    $cli->insert('accounts', ['id' => 1, 'balance' => 1000]);
    $cli->insert('accounts', ['id' => 2, 'balance' => 500]);
    $cli->update('accounts', ['balance' => 900], ['field' => 'id', 'eq' => 1]);
    $cli->update('accounts', ['balance' => 600], ['field' => 'id', 'eq' => 2]);
}, 'serializable');
```

### Сложные фильтры

#### WHERE с AND/OR

```python
# Python
result = client.execute({
    "op": "select",
    "from": "users",
    "where": {
        "and": [
            {"field": "age", "gt": 25},
            {
                "or": [
                    {"field": "status", "eq": "active"},
                    {"field": "status", "eq": "trial"}
                ]
            }
        ]
    }
})
```

```typescript
// TypeScript
const result = await client.execute({
    op: "select",
    from: "users",
    where: {
        and: [
            { field: "age", gt: 25 },
            {
                or: [
                    { field: "status", eq: "active" },
                    { field: "status", eq: "trial" }
                ]
            }
        ]
    }
});
```

```php
// PHP
$result = $client->execute([
    'op' => 'select',
    'from' => 'users',
    'where' => [
        'and' => [
            ['field' => 'age', 'gt' => 25],
            [
                'or' => [
                    ['field' => 'status', 'eq' => 'active'],
                    ['field' => 'status', 'eq' => 'trial']
                ]
            ]
        ]
    ]
]);
```

### Window Functions

```python
result = client.execute({
    "op": "select",
    "from": "employees",
    "columns": ["name", "salary"],
    "aggregate": [
        {"func": "row_number", "as": "rank"}
    ],
    "order_by": [{"field": "salary", "desc": True}]
})
```

### Запросы к JSON документам

```python
result = client.execute({
    "op": "search",
    "collection": "posts",
    "query": {
        "or": [
            {"field": "author", "eq": "Alice"},
            {"field": "tags", "in": ["python", "database"]}
        ]
    },
    "fulltext": {
        "field": "content",
        "match": "NexusDB"
    }
})
```

### Графовые запросы

```python
# Найти путь между узлами
result = client.execute({
    "op": "graph_traverse",
    "graph": "social_network",
    "start": "user_1",
    "end": "user_10",
    "max_depth": 3
})
```

### Векторный поиск

```python
# K-nearest neighbors
result = client.execute({
    "op": "vector_search",
    "collection": "embeddings",
    "query_vector": [0.1, 0.2, 0.3, ...],
    "k": 5,
    "threshold": 0.7
})
```

## Query Builder (Fluent Interface)

### Python

```python
from nexus import SelectBuilder

query = (
    SelectBuilder("users")
    .columns("id", "name", "email")
    .where("age", "gt", 25)
    .and_("status", "eq", "active")
    .order_by("created_at", desc=True)
    .limit(10)
)

result = client.execute(query.build())
```

### TypeScript

```typescript
import { SelectBuilder } from "nexus-db";

const query = new SelectBuilder("users")
    .columns("id", "name", "email")
    .where("age", "gt", 25)
    .and("status", "eq", "active")
    .orderBy("created_at", true)
    .limit(10);

const result = await client.execute(query.build());
```

### PHP

```php
use NexusDB\SelectBuilder;

$query = (new SelectBuilder('users'))
    ->columns('id', 'name', 'email')
    ->where('age', 'gt', 25)
    ->and('status', 'eq', 'active')
    ->orderBy('created_at', true)
    ->limit(10);

$result = $client->execute($query->build());
```

## Обработка ошибок

Все ошибки возвращаются в структурированном виде:

```python
# Python
try:
    result = client.select("nonexistent_table")
except NQLError as e:
    print(f"Error: {e.code} - {e.message}")
    print(f"Context: {e.context}")
```

```typescript
// TypeScript
try {
    const result = await client.select("nonexistent_table");
} catch (error) {
    console.error(`Error: ${error.message}`);
}
```

```php
// PHP
try {
    $result = $client->select('nonexistent_table');
} catch (NexusException $e) {
    echo "Error: " . $e->getMessage();
}
```

## Connection Settings

### Environment Variables

```bash
# .env
NEXUS_HOST=localhost
NEXUS_PORT=5433
NEXUS_TIMEOUT=30
```

### Create Client with Custom Settings

```python
client = NexusClient(
    host="db.example.com",
    port=5433,
    timeout=60
)
```

```typescript
const client = new NexusClient("db.example.com", 5433, 60000);
```

```php
$client = new NexusClient('db.example.com', 5433, 60);
```

## Server Compatibility

| Функция | Версия Min |
|---------|-----------|
| NQL Basic | 1.0.0 |
| Transactions | 1.0.0 |
| Window Functions | 1.0.0 |
| Graphs | 1.0.0 |
| Vectors | 1.0.0 |
| Full-Text Search | 1.0.0 |

## Дополнительно

- [NQL Specification](../NQL-specification.md) — Полный справочник команд
- [Architecture](../architecture-ru.md) — Как устроена NexusDB
- [Performance Tips](../performance-ru.md) — Оптимизация запросов
- [Security](../../SECURITY.md) — Рекомендации по безопасности

## Лицензия

GNU Affero General Public License v3.0

## Поддержка

- 🐛 Issues: https://github.com/localzet/nexus/issues
- 💬 Discussions: https://github.com/localzet/nexus/discussions
- 📧 Email: creator@localzet.com
