<?php

/**
 * NexusDB PHP Client Library
 * 
 * Usage:
 *   $client = new NexusClient('localhost', 5433);
 *   $result = $client->select('users', where: ['field' => 'age', 'gt' => 25]);
 *   foreach ($result->rows as $row) {
 *       echo $row['name'];
 *   }
 */

namespace NexusDB;

class NexusException extends \Exception {}

class NQLError {
    public string $code;
    public string $message;
    public array $context;
    
    public function __construct(array $data) {
        $this->code = $data['code'] ?? 'UNKNOWN_ERROR';
        $this->message = $data['message'] ?? 'Unknown error';
        $this->context = $data['context'] ?? [];
    }
}

class QueryResult {
    public array $rows;
    public array $columns;
    public int $row_count;
    public float $execution_time_ms;
    
    public function __construct(array $data) {
        $this->rows = $data['rows'] ?? [];
        $this->columns = $data['columns'] ?? [];
        $this->row_count = $data['row_count'] ?? 0;
        $this->execution_time_ms = $data['execution_time_ms'] ?? 0.0;
    }
    
    public function first(): ?array {
        return $this->rows[0] ?? null;
    }
    
    public function all(): array {
        return $this->rows;
    }
}

class NexusClient {
    private string $host;
    private int $port;
    private int $timeout;
    private $socket = null;
    private bool $inTransaction = false;
    
    public function __construct(
        string $host = 'localhost',
        int $port = 5433,
        int $timeout = 30
    ) {
        $this->host = $host;
        $this->port = $port;
        $this->timeout = $timeout;
    }
    
    /**
     * Connect to NexusDB
     */
    public function connect(): void {
        if ($this->socket !== null) {
            return;
        }
        
        $this->socket = @fsockopen($this->host, $this->port, $errno, $errstr, $this->timeout);
        
        if (!$this->socket) {
            throw new NexusException("Failed to connect: $errstr ($errno)");
        }
        
        stream_set_timeout($this->socket, $this->timeout);
    }
    
    /**
     * Disconnect from server
     */
    public function disconnect(): void {
        if ($this->socket) {
            fclose($this->socket);
            $this->socket = null;
        }
    }
    
    /**
     * Send NQL command
     */
    private function sendCommand(array $command): array {
        if ($this->socket === null) {
            $this->connect();
        }
        
        $json = json_encode($command) . "\n";
        
        if (fwrite($this->socket, $json) === false) {
            throw new NexusException("Failed to send command");
        }
        
        $response = trim(fgets($this->socket, 4096));
        
        if (empty($response)) {
            throw new NexusException("No response from server");
        }
        
        $data = json_decode($response, true);
        
        if ($data === null) {
            throw new NexusException("Invalid JSON response: $response");
        }
        
        if (!$data['success'] ?? false) {
            $error = $data['error'] ?? [];
            throw new NexusException($error['message'] ?? 'Unknown error');
        }
        
        return $data['result'] ?? [];
    }
    
    /**
     * Execute NQL command
     */
    public function execute(array $command): mixed {
        $result = $this->sendCommand($command);
        
        if (isset($result['rows'])) {
            return new QueryResult($result);
        }
        
        return $result;
    }
    
    /**
     * SELECT query
     */
    public function select(
        string $table,
        ?array $columns = null,
        ?array $where = null,
        ?int $limit = null,
        ?int $offset = null
    ): QueryResult {
        $command = [
            'op' => 'select',
            'from' => $table,
        ];
        
        if ($columns !== null) {
            $command['columns'] = $columns;
        }
        if ($where !== null) {
            $command['where'] = $where;
        }
        if ($limit !== null) {
            $command['limit'] = $limit;
        }
        if ($offset !== null) {
            $command['offset'] = $offset;
        }
        
        return $this->execute($command);
    }
    
    /**
     * INSERT rows
     */
    public function insert(string $table, array $values): array {
        if (!is_array($values[0] ?? null)) {
            $values = [$values];
        }
        
        $command = [
            'op' => 'insert',
            'into' => $table,
            'values' => $values,
        ];
        
        return $this->execute($command);
    }
    
    /**
     * INSERT single row
     */
    public function insertOne(string $table, array $value): array {
        return $this->insert($table, [$value]);
    }
    
    /**
     * UPDATE rows
     */
    public function update(string $table, array $updates, array $where): array {
        $command = [
            'op' => 'update',
            'table' => $table,
            'set' => $updates,
            'where' => $where,
        ];
        
        return $this->execute($command);
    }
    
    /**
     * DELETE rows
     */
    public function delete(string $table, array $where): array {
        $command = [
            'op' => 'delete',
            'from' => $table,
            'where' => $where,
        ];
        
        return $this->execute($command);
    }
    
    /**
     * CREATE TABLE
     */
    public function createTable(string $name, array $columns): array {
        $command = [
            'op' => 'create_table',
            'name' => $name,
            'columns' => $columns,
        ];
        
        return $this->execute($command);
    }
    
    /**
     * BEGIN TRANSACTION
     */
    public function beginTransaction(string $isolation = 'read_committed'): void {
        $this->inTransaction = true;
        $this->execute([
            'op' => 'begin_transaction',
            'isolation_level' => $isolation,
        ]);
    }
    
    /**
     * COMMIT
     */
    public function commit(): void {
        if (!$this->inTransaction) {
            throw new NexusException('No active transaction');
        }
        
        $this->execute(['op' => 'commit']);
        $this->inTransaction = false;
    }
    
    /**
     * ROLLBACK
     */
    public function rollback(): void {
        if (!$this->inTransaction) {
            throw new NexusException('No active transaction');
        }
        
        $this->execute(['op' => 'rollback']);
        $this->inTransaction = false;
    }
    
    /**
     * Execute callback within transaction
     */
    public function transaction(callable $callback, string $isolation = 'serializable'): mixed {
        try {
            $this->beginTransaction($isolation);
            $result = $callback($this);
            $this->commit();
            return $result;
        } catch (\Exception $e) {
            if ($this->inTransaction) {
                $this->rollback();
            }
            throw $e;
        }
    }
    
    /**
     * Close connection on destruct
     */
    public function __destruct() {
        $this->disconnect();
    }
}

/**
 * Query builder
 */
class SelectBuilder {
    private array $query;
    
    public function __construct(string $table) {
        $this->query = [
            'op' => 'select',
            'from' => $table,
        ];
    }
    
    public function columns(string ...$cols): self {
        $this->query['columns'] = $cols;
        return $this;
    }
    
    public function where(string $field, string $operator, mixed $value): self {
        $this->query['where'] = ['field' => $field, $operator => $value];
        return $this;
    }
    
    public function and(string $field, string $operator, mixed $value): self {
        if (!isset($this->query['where'])) {
            throw new NexusException('Must call where() first');
        }
        
        $this->query['where'] = [
            'and' => [
                $this->query['where'],
                ['field' => $field, $operator => $value],
            ],
        ];
        return $this;
    }
    
    public function or(string $field, string $operator, mixed $value): self {
        if (!isset($this->query['where'])) {
            throw new NexusException('Must call where() first');
        }
        
        $this->query['where'] = [
            'or' => [
                $this->query['where'],
                ['field' => $field, $operator => $value],
            ],
        ];
        return $this;
    }
    
    public function orderBy(string $field, bool $desc = false): self {
        $this->query['order_by'] = [['field' => $field, 'desc' => $desc]];
        return $this;
    }
    
    public function limit(int $n): self {
        $this->query['limit'] = $n;
        return $this;
    }
    
    public function offset(int $n): self {
        $this->query['offset'] = $n;
        return $this;
    }
    
    public function build(): array {
        return $this->query;
    }
}

// Example usage
function example() {
    $client = new NexusClient('localhost', 5433);
    
    try {
        // Create table
        $client->createTable('users', [
            ['name' => 'id', 'type' => 'integer', 'primary_key' => true],
            ['name' => 'name', 'type' => 'string'],
            ['name' => 'age', 'type' => 'integer'],
        ]);
        
        // Insert with transaction
        $client->transaction(function($cli) {
            $cli->insert('users', [
                'id' => 1,
                'name' => 'Alice',
                'age' => 30,
            ]);
            
            $result = $cli->select('users', where: ['field' => 'age', 'gt' => 25]);
            echo "Found {$result->row_count} users\n";
            foreach ($result->rows as $row) {
                echo "{$row['name']}: {$row['age']} years\n";
            }
        });
    } finally {
        $client->disconnect();
    }
}
