use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Operation History DTO exposed to TypeScript UI
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct OperationHistoryDto {
    pub id: i64,
    pub operation_id: String,
    pub timestamp_ms: i64,
    pub operation_type: String,
    pub display_name: String,
    pub original_path: Option<String>,
    pub staged_path: Option<String>,
    pub inode: Option<i64>,
    pub status: String,
    pub can_undo: bool,
    pub details: Option<String>,
}

/// Input entry for recording an operation
#[derive(Debug, Clone)]
pub struct NewHistoryEntry {
    pub operation_id: String,
    pub operation_type: String,
    pub display_name: String,
    pub original_path: Option<String>,
    pub staged_path: Option<String>,
    pub inode: Option<i64>,
    pub status: String,
    pub can_undo: bool,
    pub details: Option<String>,
}

pub struct HistoryManager {
    conn: Mutex<Connection>,
    max_history_limit: usize,
}

impl HistoryManager {
    /// Opens or creates the SQLite persistent database in $XDG_DATA_HOME/Cuddle/history.db
    pub fn new() -> Result<Self, String> {
        let db_dir = dirs::data_dir()
            .map(|p| p.join("Cuddle"))
            .unwrap_or_else(|| PathBuf::from("."));

        fs::create_dir_all(&db_dir)
            .map_err(|e| format!("Failed to create database directory {}: {}", db_dir.display(), e))?;

        let db_path = db_dir.join("history.db");
        let conn = Connection::open(&db_path)
            .map_err(|e| format!("Failed to open SQLite database at {}: {}", db_path.display(), e))?;

        let mgr = Self {
            conn: Mutex::new(conn),
            max_history_limit: 50,
        };

        mgr.init_db()?;
        Ok(mgr)
    }

    /// Creates an in-memory SQLite instance for testing
    pub fn new_in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory()
            .map_err(|e| format!("Failed to open in-memory SQLite database: {}", e))?;

        let mgr = Self {
            conn: Mutex::new(conn),
            max_history_limit: 50,
        };

        mgr.init_db()?;
        Ok(mgr)
    }

    fn init_db(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();

        // 1. Enable Write-Ahead Logging (WAL) and Normal synchronous mode
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;",
        )
        .map_err(|e| format!("Failed to configure SQLite PRAGMA settings: {}", e))?;

        // 2. Create operation_history table and index
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS operation_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                operation_id TEXT NOT NULL,
                timestamp_ms INTEGER NOT NULL,
                operation_type TEXT NOT NULL,
                display_name TEXT NOT NULL,
                original_path TEXT,
                staged_path TEXT,
                inode INTEGER,
                status TEXT NOT NULL,
                can_undo INTEGER NOT NULL DEFAULT 0,
                details TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_history_timestamp ON operation_history(timestamp_ms DESC);
            CREATE INDEX IF NOT EXISTS idx_history_op_id ON operation_history(operation_id);",
        )
        .map_err(|e| format!("Failed to initialize operation_history schema: {}", e))?;

        Ok(())
    }

    /// Records a new operation, automatically trimming to the last 50 items
    pub fn record_operation(&self, entry: &NewHistoryEntry) -> Result<i64, String> {
        let conn = self.conn.lock().unwrap();

        let now_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        conn.execute(
            "INSERT INTO operation_history (
                operation_id, timestamp_ms, operation_type, display_name,
                original_path, staged_path, inode, status, can_undo, details
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10);",
            params![
                entry.operation_id,
                now_ms,
                entry.operation_type,
                entry.display_name,
                entry.original_path,
                entry.staged_path,
                entry.inode,
                entry.status,
                if entry.can_undo { 1 } else { 0 },
                entry.details,
            ],
        )
        .map_err(|e| format!("Failed to insert operation history: {}", e))?;

        let inserted_id = conn.last_insert_rowid();

        // Auto-trim to max_history_limit (50 items)
        let _ = conn.execute(
            "DELETE FROM operation_history WHERE id NOT IN (
                SELECT id FROM operation_history ORDER BY timestamp_ms DESC LIMIT ?1
            );",
            params![self.max_history_limit as i64],
        );

        Ok(inserted_id)
    }

    /// Lists the most recent history records up to `limit` (default: 50)
    pub fn list_history(&self, limit: usize) -> Result<Vec<OperationHistoryDto>, String> {
        let conn = self.conn.lock().unwrap();
        let effective_limit = if limit == 0 || limit > self.max_history_limit {
            self.max_history_limit
        } else {
            limit
        };

        let mut stmt = conn
            .prepare(
                "SELECT id, operation_id, timestamp_ms, operation_type, display_name,
                        original_path, staged_path, inode, status, can_undo, details
                 FROM operation_history
                 ORDER BY timestamp_ms DESC
                 LIMIT ?1;",
            )
            .map_err(|e| format!("Failed to prepare list_history statement: {}", e))?;

        let rows = stmt
            .query_map(params![effective_limit as i64], |row| {
                let can_undo_int: i32 = row.get(9)?;
                Ok(OperationHistoryDto {
                    id: row.get(0)?,
                    operation_id: row.get(1)?,
                    timestamp_ms: row.get(2)?,
                    operation_type: row.get(3)?,
                    display_name: row.get(4)?,
                    original_path: row.get(5)?,
                    staged_path: row.get(6)?,
                    inode: row.get(7)?,
                    status: row.get(8)?,
                    can_undo: can_undo_int != 0,
                    details: row.get(10)?,
                })
            })
            .map_err(|e| format!("Failed to query operation history: {}", e))?;

        let mut list = Vec::new();
        for r in rows {
            let item = r.map_err(|e| format!("Failed to read history row: {}", e))?;
            list.push(item);
        }

        Ok(list)
    }

    /// Marks an operation as undone
    pub fn mark_as_undone(&self, operation_id: &str) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE operation_history SET can_undo = 0, status = 'Undone' WHERE operation_id = ?1;",
            params![operation_id],
        )
        .map_err(|e| format!("Failed to mark operation as undone: {}", e))?;
        Ok(())
    }

    /// Clears all operation history
    pub fn clear_history(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM operation_history;", [])
            .map_err(|e| format!("Failed to clear operation history: {}", e))?;
        Ok(())
    }
}

pub fn get_history_manager() -> &'static HistoryManager {
    static INSTANCE: OnceLock<HistoryManager> = OnceLock::new();
    INSTANCE.get_or_init(|| {
        HistoryManager::new().expect("Failed to initialize SQLite Operation History Manager")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_history_sqlite_wal_and_roundtrip() {
        let mgr = HistoryManager::new_in_memory().unwrap();

        // 1. Record an operation
        let entry1 = NewHistoryEntry {
            operation_id: "op-1".to_string(),
            operation_type: "BulkRename".to_string(),
            display_name: "3 files renamed".to_string(),
            original_path: Some("/home/susie/test".to_string()),
            staged_path: Some("/home/susie/.cuddle-temp".to_string()),
            inode: Some(12345),
            status: "Success".to_string(),
            can_undo: true,
            details: Some("Renamed alpha -> beta".to_string()),
        };

        let id1 = mgr.record_operation(&entry1).unwrap();
        assert!(id1 > 0);

        let list = mgr.list_history(10).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].operation_id, "op-1");
        assert_eq!(list[0].operation_type, "BulkRename");
        assert!(list[0].can_undo);
        assert_eq!(list[0].status, "Success");

        // 2. Mark as undone
        mgr.mark_as_undone("op-1").unwrap();
        let list2 = mgr.list_history(10).unwrap();
        assert!(!list2[0].can_undo);
        assert_eq!(list2[0].status, "Undone");

        // 3. Clear history
        mgr.clear_history().unwrap();
        assert_eq!(mgr.list_history(10).unwrap().len(), 0);
    }

    #[test]
    fn test_history_auto_trim_to_50_items() {
        let mgr = HistoryManager::new_in_memory().unwrap();

        // Insert 60 entries
        for i in 0..60 {
            let entry = NewHistoryEntry {
                operation_id: format!("op-{}", i),
                operation_type: "Stage".to_string(),
                display_name: format!("file_{}.txt", i),
                original_path: None,
                staged_path: None,
                inode: None,
                status: "Success".to_string(),
                can_undo: false,
                details: None,
            };
            mgr.record_operation(&entry).unwrap();
        }

        // Must be capped at exactly 50
        let list = mgr.list_history(100).unwrap();
        assert_eq!(list.len(), 50);
    }

    #[test]
    fn test_export_typescript_bindings_history() {
        let _ = std::fs::create_dir_all("../src/types");
        let cfg = ts_rs::Config::default();
        OperationHistoryDto::export(&cfg).expect("Failed to export OperationHistoryDto");
    }
}
