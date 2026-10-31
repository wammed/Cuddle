use crate::policy::OriginalSnapshotDto;
use crate::workspace::{CloneMethod, StageStatus, StagedItemDto};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;
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

/// Index repair queue entry (Task 1-1)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct IndexRepairQueueItem {
    pub id: i64,
    pub path_base64: String,
    pub action: String,
    pub retry_count: u32,
    pub last_error: Option<String>,
    pub created_at_ms: i64,
}

/// Result of executing index repair queue (Task 1-1)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct IndexRepairResultDto {
    pub processed_count: usize,
    pub succeeded_count: usize,
    pub failed_count: usize,
    pub remaining_count: usize,
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

        fs::create_dir_all(&db_dir).map_err(|e| {
            format!(
                "Failed to create database directory {}: {}",
                db_dir.display(),
                e
            )
        })?;

        let db_path = db_dir.join("history.db");
        let conn = Connection::open(&db_path).map_err(|e| {
            format!(
                "Failed to open SQLite database at {}: {}",
                db_path.display(),
                e
            )
        })?;

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
            CREATE INDEX IF NOT EXISTS idx_history_op_id ON operation_history(operation_id);

            CREATE TABLE IF NOT EXISTS session_items (
                stage_id TEXT PRIMARY KEY,
                original_item_id TEXT NOT NULL,
                raw_base64 TEXT NOT NULL,
                display_name TEXT NOT NULL,
                staged_raw_base64 TEXT NOT NULL,
                staged_path TEXT NOT NULL,
                size INTEGER NOT NULL,
                clone_method TEXT NOT NULL,
                status TEXT NOT NULL,
                staged_at_ms INTEGER NOT NULL,
                canonical_path TEXT NOT NULL,
                inode INTEGER NOT NULL,
                dev INTEGER NOT NULL,
                orig_size INTEGER NOT NULL,
                mtime_ms INTEGER NOT NULL,
                sha256 TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS index_repair_queue (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path_base64 TEXT NOT NULL,
                action TEXT NOT NULL,
                retry_count INTEGER NOT NULL DEFAULT 0,
                last_error TEXT,
                created_at_ms INTEGER NOT NULL,
                UNIQUE(path_base64, action)
            );
            CREATE INDEX IF NOT EXISTS idx_repair_queue_created ON index_repair_queue(created_at_ms);",
        )
        .map_err(|e| format!("Failed to initialize SQLite schemas: {}", e))?;

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

    /// Records a Stage operation into `operation_history` and registers the staged item in `session_items`
    /// within a single atomic SQLite transaction. If either insert fails, both are rolled back,
    /// preventing ghost history records or orphaned session items.
    pub fn record_stage_operation_and_session(
        &self,
        entry: &NewHistoryEntry,
        item: &StagedItemDto,
    ) -> Result<i64, String> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn
            .transaction()
            .map_err(|e| format!("Failed to begin transaction for stage operation: {}", e))?;

        let now_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        tx.execute(
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

        let inserted_id = tx.last_insert_rowid();

        // Auto-trim to max_history_limit (50 items)
        let _ = tx.execute(
            "DELETE FROM operation_history WHERE id NOT IN (
                SELECT id FROM operation_history ORDER BY timestamp_ms DESC LIMIT ?1
            );",
            params![self.max_history_limit as i64],
        );

        let clone_method_str = match item.clone_method {
            CloneMethod::Reflink => "Reflink",
            CloneMethod::StandardCopy => "StandardCopy",
        };
        let status_str = match item.status {
            StageStatus::Staged => "Staged",
            StageStatus::Modified => "Modified",
            StageStatus::Applying => "Applying",
            StageStatus::Reflected => "Reflected",
            StageStatus::Conflict => "Conflict",
        };

        tx.execute(
            "INSERT INTO session_items (
                stage_id, original_item_id, raw_base64, display_name,
                staged_raw_base64, staged_path, size, clone_method, status,
                staged_at_ms, canonical_path, inode, dev, orig_size, mtime_ms, sha256
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
            ON CONFLICT(stage_id) DO UPDATE SET
                display_name = excluded.display_name,
                staged_raw_base64 = excluded.staged_raw_base64,
                staged_path = excluded.staged_path,
                size = excluded.size,
                clone_method = excluded.clone_method,
                status = excluded.status,
                canonical_path = excluded.canonical_path,
                inode = excluded.inode,
                dev = excluded.dev,
                orig_size = excluded.orig_size,
                mtime_ms = excluded.mtime_ms,
                sha256 = excluded.sha256;",
            params![
                item.stage_id,
                item.original_item_id,
                item.raw_base64,
                item.display_name,
                item.staged_raw_base64,
                item.staged_path,
                item.size as i64,
                clone_method_str,
                status_str,
                item.staged_at_ms,
                item.original_snapshot.canonical_path,
                item.original_snapshot.inode as i64,
                item.original_snapshot.dev as i64,
                item.original_snapshot.size as i64,
                item.original_snapshot.mtime_ms,
                item.original_snapshot.sha256.as_deref().unwrap_or(""),
            ],
        )
        .map_err(|e| format!("Failed to save session item {}: {}", item.stage_id, e))?;

        tx.commit()
            .map_err(|e| format!("Failed to commit stage operation transaction: {}", e))?;

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

    /// Retrieves a single history record by its operation_id
    pub fn get_operation(&self, operation_id: &str) -> Result<Option<OperationHistoryDto>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, operation_id, timestamp_ms, operation_type, display_name,
                        original_path, staged_path, inode, status, can_undo, details
                 FROM operation_history
                 WHERE operation_id = ?1
                 LIMIT 1;",
            )
            .map_err(|e| format!("Failed to prepare get_operation statement: {}", e))?;

        let mut rows = stmt
            .query_map(params![operation_id], |row| {
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
            .map_err(|e| format!("Failed to query get_operation: {}", e))?;

        if let Some(r) = rows.next() {
            let item = r.map_err(|e| format!("Failed to read history row: {}", e))?;
            Ok(Some(item))
        } else {
            Ok(None)
        }
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

    /// Saves or updates a staged item in the persistent session table
    pub fn save_session_item(&self, item: &StagedItemDto) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let clone_method_str = match item.clone_method {
            CloneMethod::Reflink => "Reflink",
            CloneMethod::StandardCopy => "StandardCopy",
        };
        let status_str = match item.status {
            StageStatus::Staged => "Staged",
            StageStatus::Modified => "Modified",
            StageStatus::Applying => "Applying",
            StageStatus::Reflected => "Reflected",
            StageStatus::Conflict => "Conflict",
        };

        conn.execute(
            "INSERT INTO session_items (
                stage_id, original_item_id, raw_base64, display_name,
                staged_raw_base64, staged_path, size, clone_method, status,
                staged_at_ms, canonical_path, inode, dev, orig_size, mtime_ms, sha256
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
            ON CONFLICT(stage_id) DO UPDATE SET
                display_name = excluded.display_name,
                staged_raw_base64 = excluded.staged_raw_base64,
                staged_path = excluded.staged_path,
                size = excluded.size,
                clone_method = excluded.clone_method,
                status = excluded.status,
                canonical_path = excluded.canonical_path,
                inode = excluded.inode,
                dev = excluded.dev,
                orig_size = excluded.orig_size,
                mtime_ms = excluded.mtime_ms,
                sha256 = excluded.sha256;",
            params![
                item.stage_id,
                item.original_item_id,
                item.raw_base64,
                item.display_name,
                item.staged_raw_base64,
                item.staged_path,
                item.size as i64,
                clone_method_str,
                status_str,
                item.staged_at_ms,
                item.original_snapshot.canonical_path,
                item.original_snapshot.inode as i64,
                item.original_snapshot.dev as i64,
                item.original_snapshot.size as i64,
                item.original_snapshot.mtime_ms,
                item.original_snapshot.sha256.as_deref().unwrap_or(""),
            ],
        )
        .map_err(|e| format!("Failed to save session item {}: {}", item.stage_id, e))?;

        Ok(())
    }

    /// Saves or updates multiple staged items within an atomic SQLite transaction.
    /// If any insert/update fails, the entire transaction is rolled back.
    pub fn save_session_items_batch(&self, items: &[StagedItemDto]) -> Result<(), String> {
        if items.is_empty() {
            return Ok(());
        }

        let mut conn = self.conn.lock().unwrap();
        let tx = conn
            .transaction()
            .map_err(|e| format!("Failed to begin transaction for session items: {}", e))?;

        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO session_items (
                        stage_id, original_item_id, raw_base64, display_name,
                        staged_raw_base64, staged_path, size, clone_method, status,
                        staged_at_ms, canonical_path, inode, dev, orig_size, mtime_ms, sha256
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
                    ON CONFLICT(stage_id) DO UPDATE SET
                        display_name = excluded.display_name,
                        staged_raw_base64 = excluded.staged_raw_base64,
                        staged_path = excluded.staged_path,
                        size = excluded.size,
                        clone_method = excluded.clone_method,
                        status = excluded.status,
                        canonical_path = excluded.canonical_path,
                        inode = excluded.inode,
                        dev = excluded.dev,
                        orig_size = excluded.orig_size,
                        mtime_ms = excluded.mtime_ms,
                        sha256 = excluded.sha256;",
                )
                .map_err(|e| format!("Failed to prepare batch statement: {}", e))?;

            for item in items {
                let clone_method_str = match item.clone_method {
                    CloneMethod::Reflink => "Reflink",
                    CloneMethod::StandardCopy => "StandardCopy",
                };
                let status_str = match item.status {
                    StageStatus::Staged => "Staged",
                    StageStatus::Modified => "Modified",
                    StageStatus::Applying => "Applying",
                    StageStatus::Reflected => "Reflected",
                    StageStatus::Conflict => "Conflict",
                };

                stmt.execute(params![
                    item.stage_id,
                    item.original_item_id,
                    item.raw_base64,
                    item.display_name,
                    item.staged_raw_base64,
                    item.staged_path,
                    item.size as i64,
                    clone_method_str,
                    status_str,
                    item.staged_at_ms,
                    item.original_snapshot.canonical_path,
                    item.original_snapshot.inode as i64,
                    item.original_snapshot.dev as i64,
                    item.original_snapshot.size as i64,
                    item.original_snapshot.mtime_ms,
                    item.original_snapshot.sha256.as_deref().unwrap_or(""),
                ])
                .map_err(|e| format!("Failed to save session item {}: {}", item.stage_id, e))?;
            }
        }

        tx.commit()
            .map_err(|e| format!("Failed to commit batch session items: {}", e))?;
        Ok(())
    }

    /// Checks if any of the given stage_ids already exist in the session_items table.
    /// Returns the list of conflicting stage_ids (empty if none exist).
    pub fn check_existing_stage_ids(&self, stage_ids: &[String]) -> Result<Vec<String>, String> {
        if stage_ids.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self.conn.lock().unwrap();
        let mut conflicts = Vec::new();
        let mut stmt = conn
            .prepare("SELECT stage_id FROM session_items WHERE stage_id = ?1;")
            .map_err(|e| format!("Failed to prepare conflict check statement: {}", e))?;

        for id in stage_ids {
            let exists = stmt
                .exists(params![id])
                .map_err(|e| format!("Failed to query stage_id {}: {}", id, e))?;
            if exists {
                conflicts.push(id.clone());
            }
        }

        Ok(conflicts)
    }

    /// Inserts restored session items strictly using pure INSERT (NO ON-CONFLICT UPSERT).
    /// Used exclusively during backup restoration to prevent overwriting active session records
    /// or causing partial-update generation mismatches.
    /// Fails immediately and rolls back the entire batch if any conflict occurs.
    pub fn insert_restored_session_items_batch(
        &self,
        items: &[StagedItemDto],
    ) -> Result<(), String> {
        if items.is_empty() {
            return Ok(());
        }

        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction().map_err(|e| {
            format!(
                "Failed to begin transaction for restored session items: {}",
                e
            )
        })?;

        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO session_items (
                        stage_id, original_item_id, raw_base64, display_name,
                        staged_raw_base64, staged_path, size, clone_method, status,
                        staged_at_ms, canonical_path, inode, dev, orig_size, mtime_ms, sha256
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16);",
                )
                .map_err(|e| format!("Failed to prepare restore insert statement: {}", e))?;

            for item in items {
                let clone_method_str = match item.clone_method {
                    CloneMethod::Reflink => "Reflink",
                    CloneMethod::StandardCopy => "StandardCopy",
                };
                let status_str = match item.status {
                    StageStatus::Staged => "Staged",
                    StageStatus::Modified => "Modified",
                    StageStatus::Applying => "Applying",
                    StageStatus::Reflected => "Reflected",
                    StageStatus::Conflict => "Conflict",
                };

                stmt.execute(params![
                    item.stage_id,
                    item.original_item_id,
                    item.raw_base64,
                    item.display_name,
                    item.staged_raw_base64,
                    item.staged_path,
                    item.size as i64,
                    clone_method_str,
                    status_str,
                    item.staged_at_ms,
                    item.original_snapshot.canonical_path,
                    item.original_snapshot.inode as i64,
                    item.original_snapshot.dev as i64,
                    item.original_snapshot.size as i64,
                    item.original_snapshot.mtime_ms,
                    item.original_snapshot.sha256.as_deref().unwrap_or(""),
                ])
                .map_err(|e| format!("Failed to insert restored session item {} (conflict or integrity violation): {}", item.stage_id, e))?;
            }
        }

        tx.commit()
            .map_err(|e| format!("Failed to commit restored session items: {}", e))?;
        Ok(())
    }

    /// Removes a staged item from the session table
    pub fn remove_session_item(&self, stage_id: &str) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM session_items WHERE stage_id = ?1;",
            params![stage_id],
        )
        .map_err(|e| format!("Failed to remove session item {}: {}", stage_id, e))?;
        Ok(())
    }

    /// Clears all session items
    pub fn clear_session_items(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM session_items;", [])
            .map_err(|e| format!("Failed to clear session items: {}", e))?;
        Ok(())
    }

    /// Loads all saved session items
    pub fn load_all_session_items(&self) -> Result<Vec<StagedItemDto>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT stage_id, original_item_id, raw_base64, display_name,
                        staged_raw_base64, staged_path, size, clone_method, status,
                        staged_at_ms, canonical_path, inode, dev, orig_size, mtime_ms, sha256
                 FROM session_items;",
            )
            .map_err(|e| format!("Failed to prepare load_all_session_items: {}", e))?;

        let rows = stmt
            .query_map([], |row| {
                let stage_id: String = row.get(0)?;
                let original_item_id: String = row.get(1)?;
                let raw_base64: String = row.get(2)?;
                let display_name: String = row.get(3)?;
                let staged_raw_base64: String = row.get(4)?;
                let staged_path: String = row.get(5)?;
                let size_i64: i64 = row.get(6)?;
                let clone_method_str: String = row.get(7)?;
                let status_str: String = row.get(8)?;
                let staged_at_ms: i64 = row.get(9)?;
                let canonical_path: String = row.get(10)?;
                let inode_i64: i64 = row.get(11)?;
                let dev_i64: i64 = row.get(12)?;
                let orig_size_i64: i64 = row.get(13)?;
                let mtime_ms: i64 = row.get(14)?;
                let sha256: Option<String> = row.get(15)?;

                let clone_method = match clone_method_str.as_str() {
                    "Reflink" => CloneMethod::Reflink,
                    _ => CloneMethod::StandardCopy,
                };
                let status = match status_str.as_str() {
                    "Modified" => StageStatus::Modified,
                    "Applying" => StageStatus::Applying,
                    "Reflected" => StageStatus::Reflected,
                    "Conflict" => StageStatus::Conflict,
                    _ => StageStatus::Staged,
                };

                let original_snapshot = OriginalSnapshotDto {
                    item_id: original_item_id.clone(),
                    raw_base64: raw_base64.clone(),
                    display_name: display_name.clone(),
                    canonical_path,
                    inode: inode_i64 as u64,
                    dev: dev_i64 as u64,
                    size: orig_size_i64 as u64,
                    mtime_ms,
                    sha256,
                };

                Ok(StagedItemDto {
                    stage_id,
                    original_item_id,
                    raw_base64,
                    display_name,
                    staged_raw_base64,
                    staged_path,
                    size: size_i64 as u64,
                    clone_method,
                    status,
                    staged_at_ms,
                    original_snapshot,
                })
            })
            .map_err(|e| format!("Failed to query session items: {}", e))?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(|e| format!("Failed to read session row: {}", e))?);
        }

        Ok(list)
    }

    /// Enqueues a failed or pending indexing action to the repair queue (Task 1-1).
    pub fn enqueue_index_repair(
        &self,
        path_base64: &str,
        action: &str,
        error: &str,
    ) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let now_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        conn.execute(
            "INSERT INTO index_repair_queue (path_base64, action, retry_count, last_error, created_at_ms)
             VALUES (?1, ?2, 0, ?3, ?4)
             ON CONFLICT(path_base64, action) DO UPDATE SET
                 last_error = excluded.last_error,
                 retry_count = index_repair_queue.retry_count + 1,
                 created_at_ms = excluded.created_at_ms;",
            params![path_base64, action, error, now_ms],
        )
        .map_err(|e| format!("Failed to enqueue index repair: {}", e))?;

        Ok(())
    }

    /// Returns the total number of items waiting in the index repair queue (Task 1-1).
    pub fn get_index_repair_queue_count(&self) -> Result<usize, String> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM index_repair_queue;", [], |row| {
                row.get(0)
            })
            .map_err(|e| format!("Failed to count repair queue: {}", e))?;
        Ok(count as usize)
    }

    /// Lists pending repair items up to the specified limit (Task 1-1).
    pub fn list_pending_repairs(
        &self,
        max_items: usize,
    ) -> Result<Vec<IndexRepairQueueItem>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, path_base64, action, retry_count, last_error, created_at_ms
                 FROM index_repair_queue
                 WHERE retry_count < 5
                 ORDER BY created_at_ms ASC
                 LIMIT ?1;",
            )
            .map_err(|e| format!("Failed to prepare list_pending_repairs: {}", e))?;

        let rows = stmt
            .query_map(params![max_items as i64], |row| {
                Ok(IndexRepairQueueItem {
                    id: row.get(0)?,
                    path_base64: row.get(1)?,
                    action: row.get(2)?,
                    retry_count: row.get(3)?,
                    last_error: row.get(4)?,
                    created_at_ms: row.get(5)?,
                })
            })
            .map_err(|e| format!("Failed to query repair items: {}", e))?;

        let mut items = Vec::new();
        for item in rows.flatten() {
            items.push(item);
        }
        Ok(items)
    }

    /// Deletes a successfully processed repair item from the queue (Task 1-1).
    pub fn delete_repair_item(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM index_repair_queue WHERE id = ?1;", params![id])
            .map_err(|e| format!("Failed to delete repair item {}: {}", id, e))?;
        Ok(())
    }

    /// Increments retry count and records the latest error for a failed repair attempt (Task 1-1).
    pub fn increment_repair_retry(&self, id: i64, err: &str) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE index_repair_queue
             SET retry_count = retry_count + 1, last_error = ?2
             WHERE id = ?1;",
            params![id, err],
        )
        .map_err(|e| format!("Failed to increment repair retry for {}: {}", id, e))?;
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

        // Verify get_operation
        let fetched = mgr.get_operation("op-1").unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().operation_id, "op-1");

        let missing = mgr.get_operation("non-existent").unwrap();
        assert!(missing.is_none());

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
    fn test_session_items_persistence_roundtrip() {
        let mgr = HistoryManager::new_in_memory().unwrap();

        let item = StagedItemDto {
            stage_id: "stage-persist-1".to_string(),
            original_item_id: "orig-1".to_string(),
            raw_base64: "L3BhdGg=".to_string(),
            display_name: "test.txt".to_string(),
            staged_raw_base64: "L3N0YWdlZA==".to_string(),
            staged_path: "/tmp/staged_test.txt".to_string(),
            size: 1024,
            clone_method: CloneMethod::Reflink,
            status: StageStatus::Staged,
            staged_at_ms: 1729750000000,
            original_snapshot: OriginalSnapshotDto {
                item_id: "orig-1".to_string(),
                raw_base64: "L3BhdGg=".to_string(),
                display_name: "test.txt".to_string(),
                canonical_path: "/home/susie/test.txt".to_string(),
                inode: 9999,
                dev: 66305,
                size: 1024,
                mtime_ms: 1729750000000,
                sha256: Some("hash9999".to_string()),
            },
        };

        // 1. Save item
        mgr.save_session_item(&item).unwrap();
        let list = mgr.load_all_session_items().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].stage_id, "stage-persist-1");
        assert_eq!(list[0].display_name, "test.txt");
        assert_eq!(list[0].status, StageStatus::Staged);

        // 2. Update status to Conflict
        let mut updated_item = item.clone();
        updated_item.status = StageStatus::Conflict;
        mgr.save_session_item(&updated_item).unwrap();
        let list2 = mgr.load_all_session_items().unwrap();
        assert_eq!(list2.len(), 1);
        assert_eq!(list2[0].status, StageStatus::Conflict);

        // 3. Remove item
        mgr.remove_session_item("stage-persist-1").unwrap();
        assert_eq!(mgr.load_all_session_items().unwrap().len(), 0);
    }

    #[test]
    fn test_session_items_batch_persistence() {
        let mgr = HistoryManager::new_in_memory().unwrap();

        let item1 = StagedItemDto {
            stage_id: "batch-1".to_string(),
            original_item_id: "orig-1".to_string(),
            raw_base64: "L3BhdGgx".to_string(),
            display_name: "test1.txt".to_string(),
            staged_raw_base64: "L3N0YWdlZDE=".to_string(),
            staged_path: "/tmp/staged_test1.txt".to_string(),
            size: 100,
            clone_method: CloneMethod::StandardCopy,
            status: StageStatus::Staged,
            staged_at_ms: 1000,
            original_snapshot: OriginalSnapshotDto {
                item_id: "orig-1".to_string(),
                raw_base64: "L3BhdGgx".to_string(),
                display_name: "test1.txt".to_string(),
                canonical_path: "/home/susie/test1.txt".to_string(),
                inode: 1001,
                dev: 1,
                size: 100,
                mtime_ms: 1000,
                sha256: None,
            },
        };

        let item2 = StagedItemDto {
            stage_id: "batch-2".to_string(),
            original_item_id: "orig-2".to_string(),
            raw_base64: "L3BhdGgy".to_string(),
            display_name: "test2.txt".to_string(),
            staged_raw_base64: "L3N0YWdlZDI=".to_string(),
            staged_path: "/tmp/staged_test2.txt".to_string(),
            size: 200,
            clone_method: CloneMethod::Reflink,
            status: StageStatus::Modified,
            staged_at_ms: 2000,
            original_snapshot: OriginalSnapshotDto {
                item_id: "orig-2".to_string(),
                raw_base64: "L3BhdGgy".to_string(),
                display_name: "test2.txt".to_string(),
                canonical_path: "/home/susie/test2.txt".to_string(),
                inode: 1002,
                dev: 1,
                size: 200,
                mtime_ms: 2000,
                sha256: None,
            },
        };

        // Batch save
        mgr.save_session_items_batch(&[item1, item2]).unwrap();
        let list = mgr.load_all_session_items().unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_export_typescript_bindings_history() {
        let _ = std::fs::create_dir_all("../src/types");
        let cfg = ts_rs::Config::default();
        OperationHistoryDto::export(&cfg).expect("Failed to export OperationHistoryDto");
        IndexRepairQueueItem::export(&cfg).expect("Failed to export IndexRepairQueueItem");
        IndexRepairResultDto::export(&cfg).expect("Failed to export IndexRepairResultDto");
    }
}
