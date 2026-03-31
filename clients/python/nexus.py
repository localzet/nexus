"""
NexusDB Python Client Library

Usage:
    from nexus import NexusClient, query
    
    client = NexusClient("localhost:5433")
    
    # Using NQL (unified protocol)
    result = client.execute({
        "op": "select",
        "from": "users",
        "where": {"field": "age", "gt": 25},
        "limit": 10
    })
    
    # Or using context manager for transactions
    with client.transaction(isolation="serializable"):
        client.insert_one("users", {"name": "Alice", "age": 30})
        client.insert_one("users", {"name": "Bob", "age": 25})
"""

import json
import socket
from typing import Any, Dict, List, Optional, Union
from dataclasses import dataclass, asdict


@dataclass
class NQLError:
    """NQL error response"""
    code: str
    message: str
    context: Dict[str, Any]
    
    @classmethod
    def from_dict(cls, data: Dict) -> "NQLError":
        return cls(
            code=data.get("code", "UNKNOWN_ERROR"),
            message=data.get("message", "Unknown error"),
            context=data.get("context", {})
        )


@dataclass  
class QueryResult:
    """Query result wrapper"""
    rows: List[Dict[str, Any]]
    columns: List[str]
    row_count: int
    execution_time_ms: float
    
    @classmethod
    def from_response(cls, data: Dict) -> "QueryResult":
        return cls(
            rows=data.get("rows", []),
            columns=data.get("columns", []),
            row_count=data.get("row_count", 0),
            execution_time_ms=data.get("execution_time_ms", 0.0)
        )


class NexusClient:
    """NexusDB TCP client for NQL protocol"""
    
    def __init__(self, host: str = "localhost", port: int = 5433, 
                 timeout: int = 30):
        """
        Initialize NexusDB client
        
        Args:
            host: Server hostname
            port: TCP port (default 5433, NOT 5432)
            timeout: Connection timeout in seconds
        """
        self.host = host
        self.port = port
        self.timeout = timeout
        self._socket = None
        self._in_transaction = False
    
    def connect(self) -> None:
        """Connect to NexusDB server"""
        if self._socket:
            return
        
        self._socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self._socket.settimeout(self.timeout)
        self._socket.connect((self.host, self.port))
    
    def disconnect(self) -> None:
        """Disconnect from server"""
        if self._socket:
            self._socket.close()
            self._socket = None
    
    def _send_command(self, command: Dict[str, Any]) -> Dict[str, Any]:
        """Send NQL command and receive response"""
        if not self._socket:
            self.connect()
        
        # Send as JSON line
        json_str = json.dumps(command)
        self._socket.send(json_str.encode() + b"\n")
        
        # Receive response (also JSON line)
        response_bytes = b""
        while b"\n" not in response_bytes:
            chunk = self._socket.recv(4096)
            if not chunk:
                raise ConnectionError("Server closed connection")
            response_bytes += chunk
        
        response = json.loads(response_bytes.decode().strip())
        
        # Check for errors
        if not response.get("success", False):
            error_data = response.get("error", {})
            raise NQLError.from_dict(error_data)
        
        return response.get("result", {})
    
    def execute(self, command: Dict[str, Any]) -> Union[QueryResult, Any]:
        """Execute NQL command"""
        result = self._send_command(command)
        
        # Return QueryResult for select operations
        if isinstance(result, dict) and "rows" in result:
            return QueryResult.from_response(result)
        
        return result
    
    def select(self, table: str, columns: Optional[List[str]] = None,
               where: Optional[Dict] = None, limit: int = None,
               offset: int = None) -> QueryResult:
        """Execute SELECT query"""
        command = {
            "op": "select",
            "from": table
        }
        
        if columns:
            command["columns"] = columns
        if where:
            command["where"] = where
        if limit:
            command["limit"] = limit
        if offset:
            command["offset"] = offset
        
        return self.execute(command)
    
    def insert(self, table: str, values: Union[Dict, List[Dict]]) -> Dict:
        """Insert one or multiple rows"""
        if isinstance(values, dict):
            values = [values]
        
        command = {
            "op": "insert",
            "into": table,
            "values": values
        }
        
        return self.execute(command)
    
    def insert_one(self, table: str, value: Dict) -> Dict:
        """Insert single row"""
        return self.insert(table, value)
    
    def update(self, table: str, updates: Dict, where: Dict) -> Dict:
        """Update rows"""
        command = {
            "op": "update",
            "table": table,
            "set": updates,
            "where": where
        }
        
        return self.execute(command)
    
    def delete(self, table: str, where: Dict) -> Dict:
        """Delete rows"""
        command = {
            "op": "delete",
            "from": table,
            "where": where
        }
        
        return self.execute(command)
    
    def create_table(self, name: str, columns: List[Dict]) -> Dict:
        """Create new table"""
        command = {
            "op": "create_table",
            "name": name,
            "columns": columns
        }
        
        return self.execute(command)
    
    def begin_transaction(self, isolation: str = "read_committed") -> None:
        """Begin transaction"""
        self._in_transaction = True
        command = {
            "op": "begin_transaction",
            "isolation_level": isolation
        }
        self.execute(command)
    
    def commit(self) -> None:
        """Commit transaction"""
        if not self._in_transaction:
            raise RuntimeError("No active transaction")
        
        self.execute({"op": "commit"})
        self._in_transaction = False
    
    def rollback(self) -> None:
        """Rollback transaction"""
        if not self._in_transaction:
            raise RuntimeError("No active transaction")
        
        self.execute({"op": "rollback"})
        self._in_transaction = False
    
    def transaction(self, isolation: str = "serializable"):
        """Context manager for transactions"""
        return TransactionContext(self, isolation)
    
    def __enter__(self):
        self.connect()
        return self
    
    def __exit__(self, *args):
        self.disconnect()


class TransactionContext:
    """Context manager for database transactions"""
    
    def __init__(self, client: NexusClient, isolation: str):
        self.client = client
        self.isolation = isolation
    
    def __enter__(self):
        self.client.begin_transaction(self.isolation)
        return self.client
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        if exc_type:
            self.client.rollback()
        else:
            self.client.commit()


# ========== Query Builder ==========

class SelectBuilder:
    """Fluent query builder for SELECT"""
    
    def __init__(self, table: str):
        self.query = {
            "op": "select",
            "from": table
        }
    
    def columns(self, *cols: str) -> "SelectBuilder":
        self.query["columns"] = list(cols)
        return self
    
    def where(self, field: str, op: str, value: Any) -> "SelectBuilder":
        self.query["where"] = {"field": field, op: value}
        return self
    
    def and_(self, field: str, op: str, value: Any) -> "SelectBuilder":
        if "where" not in self.query:
            raise ValueError("Must call where() first")
        
        self.query["where"] = {
            "and": [
                self.query["where"],
                {"field": field, op: value}
            ]
        }
        return self
    
    def or_(self, field: str, op: str, value: Any) -> "SelectBuilder":
        if "where" not in self.query:
            raise ValueError("Must call where() first")
        
        self.query["where"] = {
            "or": [
                self.query["where"],
                {"field": field, op: value}
            ]
        }
        return self
    
    def order_by(self, field: str, desc: bool = False) -> "SelectBuilder":
        self.query["order_by"] = [{"field": field, "desc": desc}]
        return self
    
    def limit(self, n: int) -> "SelectBuilder":
        self.query["limit"] = n
        return self
    
    def offset(self, n: int) -> "SelectBuilder":
        self.query["offset"] = n
        return self
    
    def build(self) -> Dict:
        return self.query


# Example usage
if __name__ == "__main__":
    # Connect to NexusDB
    with NexusClient("localhost", 5433) as client:
        # Create table
        client.create_table("users", [
            {"name": "id", "type": "integer", "primary_key": True},
            {"name": "name", "type": "string"},
            {"name": "age", "type": "integer"}
        ])
        
        # Insert data
        client.insert("users", {
            "id": 1,
            "name": "Alice",
            "age": 30
        })
        
        # Query with transaction
        with client.transaction("serializable"):
            result = client.select("users", where={"field": "age", "gt": 25})
            print(f"Found {result.row_count} users")
            for row in result.rows:
                print(row)
            
            client.insert("users", {"id": 2, "name": "Bob", "age": 28})
