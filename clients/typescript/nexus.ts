/**
 * NexusDB TypeScript/JavaScript Client Library
 * 
 * Usage:
 *   const client = new NexusClient("localhost:5433");
 *   const result = await client.execute({
 *     op: "select",
 *     from: "users",
 *     where: { field: "age", gt: 25 },
 *     limit: 10
 *   });
 */

import { Socket } from "net";

interface NQLCommand {
  op: string;
  [key: string]: any;
}

interface NQLError {
  code: string;
  message: string;
  context?: Record<string, any>;
}

interface NQLResponse<T = any> {
  success: boolean;
  result?: T;
  error?: NQLError;
}

interface QueryResult {
  rows: Record<string, any>[];
  columns: string[];
  row_count: number;
  execution_time_ms: number;
}

export class NexusClient {
  private host: string;
  private port: number;
  private socket: Socket | null = null;
  private timeout: number;
  private inTransaction = false;
  private buffer = "";

  constructor(host: string = "localhost", port: number = 5433, timeout: number = 30000) {
    this.host = host;
    this.port = port;
    this.timeout = timeout;
  }

  /**
   * Connect to NexusDB server
   */
  async connect(): Promise<void> {
    if (this.socket) {
      return;
    }

    return new Promise((resolve, reject) => {
      this.socket = new Socket();

      this.socket.setTimeout(this.timeout);
      this.socket.setEncoding("utf8");

      this.socket.on("data", (data: string) => {
        this.buffer += data;
      });

      this.socket.on("error", (error: Error) => {
        reject(error);
      });

      this.socket.on("timeout", () => {
        reject(new Error("Connection timeout"));
      });

      this.socket.connect(this.port, this.host, () => {
        resolve();
      });
    });
  }

  /**
   * Disconnect from server
   */
  disconnect(): void {
    if (this.socket) {
      this.socket.destroy();
      this.socket = null;
    }
  }

  /**
   * Send NQL command and receive response
   */
  private async sendCommand(command: NQLCommand): Promise<any> {
    if (!this.socket) {
      await this.connect();
    }

    // Send command as JSON line
    const json = JSON.stringify(command);
    return new Promise((resolve, reject) => {
      this.socket!.write(json + "\n", (error?: Error) => {
        if (error) {
          reject(error);
        }
      });

      // Wait for response
      const checkBuffer = () => {
        const newlineIndex = this.buffer.indexOf("\n");
        if (newlineIndex !== -1) {
          const line = this.buffer.substring(0, newlineIndex);
          this.buffer = this.buffer.substring(newlineIndex + 1);

          try {
            const response: NQLResponse = JSON.parse(line);

            if (!response.success) {
              reject(new Error(response.error?.message));
            } else {
              resolve(response.result);
            }
          } catch (e) {
            reject(e);
          }
        } else {
          // Wait a bit more for data to arrive
          setTimeout(checkBuffer, 10);
        }
      };

      checkBuffer();
    });
  }

  /**
   * Execute NQL command
   */
  async execute(command: NQLCommand): Promise<any> {
    const result = await this.sendCommand(command);
    if (result && "rows" in result) {
      return new QueryResultWrapper(result);
    }
    return result;
  }

  /**
   * SELECT query
   */
  async select(
    table: string,
    columns?: string[],
    where?: Record<string, any>,
    limit?: number,
    offset?: number
  ): Promise<QueryResultWrapper> {
    const command: NQLCommand = {
      op: "select",
      from: table,
    };

    if (columns) command.columns = columns;
    if (where) command.where = where;
    if (limit) command.limit = limit;
    if (offset) command.offset = offset;

    return (await this.execute(command)) as QueryResultWrapper;
  }

  /**
   * INSERT rows
   */
  async insert(table: string, values: Record<string, any> | Record<string, any>[]): Promise<any> {
    const command: NQLCommand = {
      op: "insert",
      into: table,
      values: Array.isArray(values) ? values : [values],
    };

    return this.execute(command);
  }

  /**
   * INSERT single row
   */
  async insertOne(table: string, value: Record<string, any>): Promise<any> {
    return this.insert(table, value);
  }

  /**
   * UPDATE rows
   */
  async update(
    table: string,
    updates: Record<string, any>,
    where: Record<string, any>
  ): Promise<any> {
    const command: NQLCommand = {
      op: "update",
      table,
      set: updates,
      where,
    };

    return this.execute(command);
  }

  /**
   * DELETE rows
   */
  async delete(table: string, where: Record<string, any>): Promise<any> {
    const command: NQLCommand = {
      op: "delete",
      from: table,
      where,
    };

    return this.execute(command);
  }

  /**
   * CREATE TABLE
   */
  async createTable(
    name: string,
    columns: Array<{
      name: string;
      type: string;
      primary_key?: boolean;
      nullable?: boolean;
    }>
  ): Promise<any> {
    const command: NQLCommand = {
      op: "create_table",
      name,
      columns,
    };

    return this.execute(command);
  }

  /**
   * Begin transaction
   */
  async beginTransaction(isolation: string = "read_committed"): Promise<void> {
    this.inTransaction = true;
    await this.execute({
      op: "begin_transaction",
      isolation_level: isolation,
    });
  }

  /**
   * Commit transaction
   */
  async commit(): Promise<void> {
    if (!this.inTransaction) {
      throw new Error("No active transaction");
    }

    await this.execute({ op: "commit" });
    this.inTransaction = false;
  }

  /**
   * Rollback transaction
   */
  async rollback(): Promise<void> {
    if (!this.inTransaction) {
      throw new Error("No active transaction");
    }

    await this.execute({ op: "rollback" });
    this.inTransaction = false;
  }

  /**
   * Execute function within transaction
   */
  async transaction<T>(
    fn: (client: NexusClient) => Promise<T>,
    isolation: string = "serializable"
  ): Promise<T> {
    try {
      await this.beginTransaction(isolation);
      const result = await fn(this);
      await this.commit();
      return result;
    } catch (error) {
      await this.rollback();
      throw error;
    }
  }
}

/**
 * Query result wrapper with helper methods
 */
export class QueryResultWrapper implements QueryResult {
  rows: Record<string, any>[];
  columns: string[];
  row_count: number;
  execution_time_ms: number;

  constructor(data: QueryResult) {
    this.rows = data.rows;
    this.columns = data.columns;
    this.row_count = data.row_count;
    this.execution_time_ms = data.execution_time_ms;
  }

  /**
   * Get first row
   */
  first(): Record<string, any> | undefined {
    return this.rows[0];
  }

  /**
   * Map rows
   */
  map<T>(fn: (row: Record<string, any>) => T): T[] {
    return this.rows.map(fn);
  }

  /**
   * Filter rows
   */
  filter(fn: (row: Record<string, any>) => boolean): Record<string, any>[] {
    return this.rows.filter(fn);
  }
}

/**
 * Query builder for SELECT
 */
export class SelectBuilder {
  private query: NQLCommand;

  constructor(table: string) {
    this.query = {
      op: "select",
      from: table,
    };
  }

  columns(...cols: string[]): this {
    this.query.columns = cols;
    return this;
  }

  where(field: string, op: string, value: any): this {
    this.query.where = { field, [op]: value };
    return this;
  }

  and(field: string, op: string, value: any): this {
    if (!this.query.where) {
      throw new Error("Must call where() first");
    }

    this.query.where = {
      and: [this.query.where, { field, [op]: value }],
    };
    return this;
  }

  or(field: string, op: string, value: any): this {
    if (!this.query.where) {
      throw new Error("Must call where() first");
    }

    this.query.where = {
      or: [this.query.where, { field, [op]: value }],
    };
    return this;
  }

  orderBy(field: string, desc: boolean = false): this {
    this.query.order_by = [{ field, desc }];
    return this;
  }

  limit(n: number): this {
    this.query.limit = n;
    return this;
  }

  offset(n: number): this {
    this.query.offset = n;
    return this;
  }

  build(): NQLCommand {
    return this.query;
  }
}

// Example usage
export async function example() {
  const client = new NexusClient("localhost", 5433);

  try {
    // Create table
    await client.createTable("users", [
      { name: "id", type: "integer", primary_key: true },
      { name: "name", type: "string" },
      { name: "age", type: "integer" },
    ]);

    // Insert with transaction
    await client.transaction(async (cli) => {
      await cli.insert("users", {
        id: 1,
        name: "Alice",
        age: 30,
      });

      const result = await cli.select("users", undefined, { field: "age", gt: 25 });
      console.log(`Found ${result.row_count} users`);
      result.rows.forEach((row) => console.log(row));
    });
  } finally {
    client.disconnect();
  }
}
