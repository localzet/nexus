# NexusDB - Полное руководство пользователя

## Что такое NexusDB?

NexusDB — это **многомодельная СУБД**, предназначенная для одновременной работы с различными типами данных:

- **SQL Таблицы** — реляционные данные c полной поддержкой ACID и транзакций
- **JSON Документы** — гибкие структурированные данные с поддержкой вложенности
- **Графы** — знаниевые базы и семантические структуры со связями между сущностями
- **Векторы** — embeddings и ML-модели для семантического поиска

Всё это в **одной СУБД** с единой архитектурой хранения, без необходимости синхронизировать несколько отдельных систем.

## Ключевые особенности

### 🔒 ACID транзакции
- Полная поддержка ACID свойств
- Multi-Version Concurrency Control (MVCC) без блокировок
- 4 уровня изоляции: READ UNCOMMITTED → READ COMMITTED → REPEATABLE READ → SERIALIZABLE
- Write-Ahead Logging (WAL) для гарантии durability

### ⚡ Производительность
- Оптимизированные индексы (B-Tree, Hash, Bloom Filter)
- LRU кеширование с умным управлением памятью
- Полнотекстовый поиск (Full-Text Search)
- Query result caching

### 🔄 Высокая доступность
- Raft консенсус для репликации
- Automatic failover
- Geo-distributed replication
- Load balancing

### 📊 Продвинутые SQL возможности
- Window Functions (ROW_NUMBER, RANK, LAG, LEAD, и т.д.)
- Common Table Expressions (CTEs) с рекурсией
- Сложные JOINы
- Aggregate functions
- Подзапросы

## Установка и развёртывание

### 🐳 Docker (рекомендуется)

```bash
# Скачивание образа
docker pull localzet/nexus:latest

# Запуск контейнера
docker run -d \
  --name nexus-db \
  -p 8080:8080 \
  -p 5433:5433 \
  -v nexus-data:/data \
  -e NEXUS_PORT=5433 \
  -e HTTP_PORT=8080 \
  localzet/nexus:latest

# Проверка статуса
docker logs nexus-db
```

### 📦 Linux (.deb пакет)

```bash
# Загрузить пакет
wget https://github.com/localzet/nexus/releases/download/v1.0.0/nexus_1.0.0_amd64.deb

# Установить
sudo dpkg -i nexus_1.0.0_amd64.deb

# Запустить
sudo systemctl start nexus
sudo systemctl enable nexus  # Автозапуск при перезагрузке

# Проверить статус
sudo systemctl status nexus
```

### 🔨 Компиляция из исходников

```bash
git clone https://github.com/localzet/nexus.git
cd nexus
cargo build --release
./target/release/nexus
```

## Первое подключение

### TCP Protocol (порт 5433)

```bash
# Подключение
psql -h localhost -p 5433 -U nexus

# Для проверки доступности:
nelstat -an | grep 5433  # Linux
netstat -ano | findstr :5433  # Windows
```

### REST API (порт 8080)

```bash
# Проверка здоровья сервера
curl http://localhost:8080/health

# Получить метрики
curl http://localhost:8080/metrics
```

## Работа с данными

### SQL Таблицы

```sql
-- Создание таблицы
CREATE TABLE users (
  id INTEGER PRIMARY KEY,
  name STRING NOT NULL,
  email STRING UNIQUE,
  age INTEGER,
  created_at TIMESTAMP
);

-- Вставка данных
INSERT INTO users VALUES 
  (1, 'Alice', 'alice@example.com', 30, NOW()),
  (2, 'Bob', 'bob@example.com', 25, NOW());

-- Простой запрос
SELECT * FROM users WHERE age > 25;

-- Agregation
SELECT age, COUNT(*) as count FROM users GROUP BY age;

-- Window Functions
SELECT 
  name, 
  age,
  ROW_NUMBER() OVER (ORDER BY age DESC) as rank
FROM users;
```

### JSON Документы (REST API)

```bash
# Создание коллекции
curl -X POST http://localhost:8080/collections \
  -H "Content-Type: application/json" \
  -d '{"name": "posts"}'

# Добавление документа
curl -X POST http://localhost:8080/collections/posts \
  -H "Content-Type: application/json" \
  -d '{
    "title": "NexusDB Guide",
    "author": "Alice",
    "tags": ["database", "sql"],
    "metadata": {
      "version": 1,
      "draft": false
    }
  }'

# Поиск по полям
curl 'http://localhost:8080/collections/posts/search?author=Alice'

# Полнотекстовый поиск
curl 'http://localhost:8080/collections/posts/search?fulltext=NexusDB'
```

### Графы (REST API)

```bash
# Создание графа
curl -X POST http://localhost:8080/graphs \
  -H "Content-Type: application/json" \
  -d '{"name": "social_network"}'

# Добавление узла
curl -X POST http://localhost:8080/graphs/social_network/nodes \
  -H "Content-Type: application/json" \
  -d '{
    "id": "user_1",
    "label": "Person",
    "properties": {
      "name": "Alice",
      "age": 30
    }
  }'

# Добавление связи
curl -X POST http://localhost:8080/graphs/social_network/edges \
  -H "Content-Type: application/json" \
  -d '{
    "from": "user_1",
    "to": "user_2",
    "label": "KNOWS",
    "properties": {"since": 2020}
  }'

# Поиск связей (graph traversal)
curl 'http://localhost:8080/graphs/social_network/path?from=user_1&to=user_3'
```

### Векторные данные (ML embeddings, REST API)

```bash
# Создание коллекции векторов
curl -X POST http://localhost:8080/vectors \
  -H "Content-Type: application/json" \
  -d '{"name": "embeddings", "dimension": 300}'

# Добавление вектора (например, из модели)
curl -X POST http://localhost:8080/vectors/embeddings \
  -H "Content-Type: application/json" \
  -d '{
    "word": "computer",
    "vector": [0.9, 0.1, 0.0, 0.2, ...],
    "metadata": {"language": "en"}
  }'

# Поиск похожих векторов (k-nearest neighbors)
curl -X POST http://localhost:8080/vectors/embeddings/knn \
  -H "Content-Type: application/json" \
  -d '{
    "query_vector": [0.85, 0.15, 0.05, ...],
    "k": 5,
    "threshold": 0.7
  }'
```

## Транзакции

```sql
-- Начало транзакции
BEGIN TRANSACTION;

-- Набор операций
INSERT INTO accounts VALUES (1, 'Alice', 1000);
INSERT INTO transactions VALUES (1, 1, 'deposit', 100);
UPDATE accounts SET balance = balance + 100 WHERE id = 1;

-- Либо успешно завершить
COMMIT;

-- Либо откатить всё
ROLLBACK;
```

## Версионирование и временные ряды

```bash
# При вставке документов можно указать версию
curl -X POST http://localhost:8080/collections/posts \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Updated article",
    "_version": 2,
    "_timestamp": "2026-03-31T12:00:00Z"
  }'

# Возврат к конкретной версии
GET /collections/posts/{id}?version=1
```

## Мониторинг и доступ к логам

```bash
# Здоровье сервера
curl http://localhost:8080/health

# Метрики (Prometheus format)
curl http://localhost:8080/metrics

# Slow query log
curl http://localhost:8080/logs/slow-queries

# Query execution plan
curl -X POST http://localhost:8080/query/explain \
  -H "Content-Type: application/json" \
  -d '{"sql": "SELECT * FROM users WHERE age > 25"}'
```

## Конфигурация

Создайте файл `nexus.toml` в папке конфигурации:

```toml
[server]
# Адрес прослушивания
host = "0.0.0.0"

# TCP порт (NOT 5432, как в PostgreSQL!)
tcp_port = 5433

# REST API порт
rest_port = 8080

[database]
# Папка для хранения данных
data_dir = "/var/lib/nexus"

# Размер буфера в MB
cache_size_mb = 2048

# Максимальное количество соединений
max_connections = 1000

[replication]
# Включить Raft репликацию
enable_raft = true

# Узлы кластера
cluster_nodes = [
  "node1.example.com:6379",
  "node2.example.com:6379",
  "node3.example.com:6379"
]

[performance]
# Кеширование результатов запросов
enable_query_cache = true
query_cache_mb = 512

# Полнотекстовый поиск
enable_fulltext_index = true

[security]
# Аутентификация
auth_enabled = true
default_user = "nexus"
default_password = "change-me"
```

## Команды в CLI

```bash
# Подключение через psql
psql -h localhost -p 5433 -U nexus

nexus=> \dt          # Список таблиц
nexus=> \di          # Список индексов
nexus=> \dc          # Список коллекций
nexus=> \dg          # Список графов
nexus=> \dv          # Список векторов
nexus=> HELP;        # Справка
```

## Типичные сценарии использования

### Сценарий 1: E-commerce платформа

```sql
-- Таблицы для продуктов
CREATE TABLE products (id, name, description, price);

-- JSON для детального описания
curl -X POST /collections/product_specs -d '{...}'

-- Граф для категорий и связей
curl -X POST /graphs/category_tree -d '{...}'

-- Векторы для поиска по описанию
curl -X POST /vectors/product_embeddings -d '{...}'
```

### Сценарий 2: Социальная сеть

```bash
# Таблицы для пользователей и постов
SQL: CREATE TABLE users, posts

# Граф для фолловеров и подписок
curl -X POST /graphs/social_network

# JSON для профилей и подробных данных
curl -X POST /collections/user_profiles
```

### Сценарий 3: AI/ML приложение

```bash
# Метаданные в таблицах
SQL: CREATE TABLE models, datasets, experiments

# Векторы для embeddings моделей
curl -X POST /vectors/model_embeddings

# Граф вычислительного графа (DAG)
curl -X POST /graphs/computation_graph

# JSON для конфигураций
curl -X POST /collections/model_configs
```

## Производительность и тунинг

```bash
# Индексация для SQL таблиц
CREATE INDEX idx_users_email ON users(email);

# Анализ плана запроса
EXPLAIN SELECT * FROM users WHERE email = 'alice@example.com';

# Сбор статистики
ANALYZE TABLE users;
```

## Лицензия

GNU Affero General Public License v3.0

См. [LICENSE](../LICENSE)

## Её участь в экосистеме

NexusDB НЕ заменяет:
- PostgreSQL для чисто реляционных задач
- MongoDB для чистых документов
- Neo4j для графов
- Milvus для векторов

NexusDB решает задачу **"когда вам нужны ВСЕ типы данных вместе**" и вы хотите **одну архитектуру**, а не stack из 4 СУБД с синхронизацией между ними.

## Дальнейшая помощь

- [Архитектура](./architecture-ru.md) — Как устроена NexusDB
- GitHub Issues: https://github.com/localzet/nexus/issues
- GitHub Discussions: https://github.com/localzet/nexus/discussions
