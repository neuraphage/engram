//! Storage layer for Engram: JSONL files + SQLite cache.

use crate::types::{Edge, EdgeKind, Item, Status};
use eyre::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// Storage directory name.
const ENGRAM_DIR: &str = ".engram";

/// JSONL file for items.
const ITEMS_FILE: &str = "items.jsonl";

/// JSONL file for edges.
const EDGES_FILE: &str = "edges.jsonl";

/// SQLite database file.
const DB_FILE: &str = "engram.db";

/// Storage handle for reading/writing engram data.
pub struct Storage {
    root: PathBuf,
    db: Connection,
}

impl Storage {
    /// Initialize storage in the given directory.
    pub fn init(root: &Path) -> Result<Self> {
        let engram_dir = root.join(ENGRAM_DIR);
        fs::create_dir_all(&engram_dir).context("Failed to create .engram directory")?;

        // Create empty JSONL files if they don't exist
        let items_path = engram_dir.join(ITEMS_FILE);
        let edges_path = engram_dir.join(EDGES_FILE);

        if !items_path.exists() {
            File::create(&items_path).context("Failed to create items.jsonl")?;
        }
        if !edges_path.exists() {
            File::create(&edges_path).context("Failed to create edges.jsonl")?;
        }

        // Create SQLite database
        let db_path = engram_dir.join(DB_FILE);
        let db = Connection::open(&db_path).context("Failed to open SQLite database")?;

        let mut storage = Self {
            root: root.to_path_buf(),
            db,
        };

        storage.init_schema()?;
        storage.rebuild_from_jsonl()?;

        Ok(storage)
    }

    /// Open existing storage.
    pub fn open(root: &Path) -> Result<Self> {
        let engram_dir = root.join(ENGRAM_DIR);
        if !engram_dir.exists() {
            eyre::bail!("No .engram directory found. Run 'engram init' first.");
        }

        let db_path = engram_dir.join(DB_FILE);
        let db = Connection::open(&db_path).context("Failed to open SQLite database")?;

        let mut storage = Self {
            root: root.to_path_buf(),
            db,
        };

        storage.init_schema()?;

        // Check consistency and rebuild if needed
        if storage.needs_rebuild()? {
            storage.rebuild_from_jsonl()?;
        }

        Ok(storage)
    }

    /// Initialize SQLite schema.
    fn init_schema(&self) -> Result<()> {
        self.db
            .execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS items (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    description TEXT,
                    status TEXT NOT NULL CHECK (status IN ('open', 'in_progress', 'blocked', 'closed')),
                    priority INTEGER NOT NULL CHECK (priority BETWEEN 0 AND 4),
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    closed_at TEXT,
                    close_reason TEXT
                );

                CREATE TABLE IF NOT EXISTS labels (
                    item_id TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
                    label TEXT NOT NULL,
                    PRIMARY KEY (item_id, label)
                );
                CREATE INDEX IF NOT EXISTS idx_labels_label ON labels(label);

                CREATE TABLE IF NOT EXISTS edges (
                    from_id TEXT NOT NULL,
                    to_id TEXT NOT NULL,
                    kind TEXT NOT NULL CHECK (kind IN ('blocks', 'parent_child', 'related')),
                    created_at TEXT NOT NULL,
                    PRIMARY KEY (from_id, to_id, kind)
                );
                CREATE INDEX IF NOT EXISTS idx_edges_to ON edges(to_id);
                CREATE INDEX IF NOT EXISTS idx_edges_kind ON edges(kind);

                CREATE TABLE IF NOT EXISTS meta (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );
            "#,
            )
            .context("Failed to initialize schema")?;

        Ok(())
    }

    /// Check if SQLite needs to be rebuilt from JSONL.
    fn needs_rebuild(&self) -> Result<bool> {
        let items_path = self.root.join(ENGRAM_DIR).join(ITEMS_FILE);
        let edges_path = self.root.join(ENGRAM_DIR).join(EDGES_FILE);

        let items_lines = count_lines(&items_path)?;
        let edges_lines = count_lines(&edges_path)?;

        let stored_items: i64 = self
            .db
            .query_row(
                "SELECT COALESCE((SELECT value FROM meta WHERE key = 'jsonl_items_lines'), '0')",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let stored_edges: i64 = self
            .db
            .query_row(
                "SELECT COALESCE((SELECT value FROM meta WHERE key = 'jsonl_edges_lines'), '0')",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        Ok(items_lines as i64 != stored_items || edges_lines as i64 != stored_edges)
    }

    /// Rebuild SQLite cache from JSONL files.
    pub fn rebuild_from_jsonl(&mut self) -> Result<()> {
        let items_path = self.root.join(ENGRAM_DIR).join(ITEMS_FILE);
        let edges_path = self.root.join(ENGRAM_DIR).join(EDGES_FILE);

        // Clear existing data
        self.db
            .execute_batch(
                r#"
                DELETE FROM labels;
                DELETE FROM edges;
                DELETE FROM items;
            "#,
            )
            .context("Failed to clear tables")?;

        // Read items (last occurrence wins)
        let mut items: HashMap<String, Item> = HashMap::new();
        let mut items_line_count = 0;

        if items_path.exists() {
            let file = File::open(&items_path).context("Failed to open items.jsonl")?;
            let reader = BufReader::new(file);

            for line in reader.lines() {
                items_line_count += 1;
                let line = match line {
                    Ok(l) => l,
                    Err(e) => {
                        log::warn!("Failed to read line {}: {}", items_line_count, e);
                        continue;
                    }
                };

                if line.trim().is_empty() {
                    continue;
                }

                match serde_json::from_str::<Item>(&line) {
                    Ok(item) => {
                        items.insert(item.id.clone(), item);
                    }
                    Err(e) => {
                        log::warn!("Failed to parse item at line {}: {}", items_line_count, e);
                    }
                }
            }
        }

        // Insert items into SQLite
        for item in items.values() {
            self.insert_item_to_db(item)?;
        }

        // Read edges (handle tombstones)
        let mut edges: HashMap<(String, String, String), Option<Edge>> = HashMap::new();
        let mut edges_line_count = 0;

        if edges_path.exists() {
            let file = File::open(&edges_path).context("Failed to open edges.jsonl")?;
            let reader = BufReader::new(file);

            for line in reader.lines() {
                edges_line_count += 1;
                let line = match line {
                    Ok(l) => l,
                    Err(e) => {
                        log::warn!("Failed to read edge line {}: {}", edges_line_count, e);
                        continue;
                    }
                };

                if line.trim().is_empty() {
                    continue;
                }

                match serde_json::from_str::<Edge>(&line) {
                    Ok(edge) => {
                        let key = (
                            edge.from_id.clone(),
                            edge.to_id.clone(),
                            format!("{:?}", edge.kind).to_lowercase(),
                        );
                        if edge.deleted {
                            edges.insert(key, None);
                        } else {
                            edges.insert(key, Some(edge));
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to parse edge at line {}: {}", edges_line_count, e);
                    }
                }
            }
        }

        // Insert non-deleted edges into SQLite
        for edge in edges.values().flatten() {
            self.insert_edge_to_db(edge)?;
        }

        // Update metadata
        self.db.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('jsonl_items_lines', ?)",
            params![items_line_count.to_string()],
        )?;
        self.db.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('jsonl_edges_lines', ?)",
            params![edges_line_count.to_string()],
        )?;

        Ok(())
    }

    /// Insert an item into SQLite.
    fn insert_item_to_db(&self, item: &Item) -> Result<()> {
        let status_str = match item.status {
            Status::Open => "open",
            Status::InProgress => "in_progress",
            Status::Blocked => "blocked",
            Status::Closed => "closed",
        };

        self.db.execute(
            r#"
            INSERT OR REPLACE INTO items (id, title, description, status, priority, created_at, updated_at, closed_at, close_reason)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                item.id,
                item.title,
                item.description,
                status_str,
                item.priority,
                item.created_at.to_rfc3339(),
                item.updated_at.to_rfc3339(),
                item.closed_at.map(|dt| dt.to_rfc3339()),
                item.close_reason,
            ],
        )?;

        // Delete existing labels and insert new ones
        self.db
            .execute("DELETE FROM labels WHERE item_id = ?", params![item.id])?;
        for label in &item.labels {
            self.db.execute(
                "INSERT INTO labels (item_id, label) VALUES (?, ?)",
                params![item.id, label],
            )?;
        }

        Ok(())
    }

    /// Insert an edge into SQLite.
    fn insert_edge_to_db(&self, edge: &Edge) -> Result<()> {
        let kind_str = match edge.kind {
            EdgeKind::Blocks => "blocks",
            EdgeKind::ParentChild => "parent_child",
            EdgeKind::Related => "related",
        };

        self.db.execute(
            r#"
            INSERT OR REPLACE INTO edges (from_id, to_id, kind, created_at)
            VALUES (?, ?, ?, ?)
            "#,
            params![edge.from_id, edge.to_id, kind_str, edge.created_at.to_rfc3339(),],
        )?;

        Ok(())
    }

    /// Append an item to the JSONL file.
    pub fn append_item(&mut self, item: &Item) -> Result<()> {
        let items_path = self.root.join(ENGRAM_DIR).join(ITEMS_FILE);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&items_path)
            .context("Failed to open items.jsonl for append")?;

        let json = serde_json::to_string(item).context("Failed to serialize item")?;
        writeln!(file, "{}", json).context("Failed to write to items.jsonl")?;
        file.sync_all().context("Failed to sync items.jsonl")?;

        // Update SQLite cache
        self.insert_item_to_db(item)?;

        // Update line count
        self.db.execute(
            "UPDATE meta SET value = CAST(CAST(value AS INTEGER) + 1 AS TEXT) WHERE key = 'jsonl_items_lines'",
            [],
        )?;

        Ok(())
    }

    /// Append an edge to the JSONL file.
    pub fn append_edge(&mut self, edge: &Edge) -> Result<()> {
        let edges_path = self.root.join(ENGRAM_DIR).join(EDGES_FILE);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&edges_path)
            .context("Failed to open edges.jsonl for append")?;

        let json = serde_json::to_string(edge).context("Failed to serialize edge")?;
        writeln!(file, "{}", json).context("Failed to write to edges.jsonl")?;
        file.sync_all().context("Failed to sync edges.jsonl")?;

        // Update SQLite cache (only if not deleted)
        if !edge.deleted {
            self.insert_edge_to_db(edge)?;
        } else {
            // Remove from SQLite
            let kind_str = match edge.kind {
                EdgeKind::Blocks => "blocks",
                EdgeKind::ParentChild => "parent_child",
                EdgeKind::Related => "related",
            };
            self.db.execute(
                "DELETE FROM edges WHERE from_id = ? AND to_id = ? AND kind = ?",
                params![edge.from_id, edge.to_id, kind_str],
            )?;
        }

        // Update line count
        self.db.execute(
            "UPDATE meta SET value = CAST(CAST(value AS INTEGER) + 1 AS TEXT) WHERE key = 'jsonl_edges_lines'",
            [],
        )?;

        Ok(())
    }

    /// Get an item by ID.
    pub fn get_item(&self, id: &str) -> Result<Option<Item>> {
        let mut stmt = self.db.prepare(
            r#"
            SELECT id, title, description, status, priority, created_at, updated_at, closed_at, close_reason
            FROM items WHERE id = ?
            "#,
        )?;

        let item = stmt
            .query_row(params![id], |row| {
                let status_str: String = row.get(3)?;
                let status = match status_str.as_str() {
                    "open" => Status::Open,
                    "in_progress" => Status::InProgress,
                    "blocked" => Status::Blocked,
                    "closed" => Status::Closed,
                    _ => Status::Open,
                };

                let created_at_str: String = row.get(5)?;
                let updated_at_str: String = row.get(6)?;
                let closed_at_str: Option<String> = row.get(7)?;

                Ok(Item {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    status,
                    priority: row.get(4)?,
                    labels: vec![], // Will be filled below
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    closed_at: closed_at_str.and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                            .ok()
                    }),
                    close_reason: row.get(8)?,
                })
            })
            .optional()?;

        // Load labels if item exists
        if let Some(mut item) = item {
            let mut label_stmt = self
                .db
                .prepare("SELECT label FROM labels WHERE item_id = ? ORDER BY label")?;
            let labels: Vec<String> = label_stmt
                .query_map(params![id], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();
            item.labels = labels;
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }

    /// List all items with optional status filter.
    pub fn list_items(&self, status_filter: Option<Status>) -> Result<Vec<Item>> {
        let sql = match status_filter {
            Some(_) => {
                r#"
                SELECT id, title, description, status, priority, created_at, updated_at, closed_at, close_reason
                FROM items WHERE status = ?
                ORDER BY priority ASC, created_at ASC
                "#
            }
            None => {
                r#"
                SELECT id, title, description, status, priority, created_at, updated_at, closed_at, close_reason
                FROM items
                ORDER BY priority ASC, created_at ASC
                "#
            }
        };

        let mut stmt = self.db.prepare(sql)?;

        let rows = if let Some(status) = status_filter {
            let status_str = match status {
                Status::Open => "open",
                Status::InProgress => "in_progress",
                Status::Blocked => "blocked",
                Status::Closed => "closed",
            };
            stmt.query_map(params![status_str], Self::row_to_item)?
        } else {
            stmt.query_map([], Self::row_to_item)?
        };

        let mut items: Vec<Item> = rows.filter_map(|r| r.ok()).collect();

        // Load labels for each item
        for item in &mut items {
            let mut label_stmt = self
                .db
                .prepare("SELECT label FROM labels WHERE item_id = ? ORDER BY label")?;
            item.labels = label_stmt
                .query_map(params![item.id], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();
        }

        Ok(items)
    }

    /// Get items that are ready to work on (open, not blocked).
    pub fn ready(&self) -> Result<Vec<Item>> {
        let sql = r#"
            SELECT i.id, i.title, i.description, i.status, i.priority, i.created_at, i.updated_at, i.closed_at, i.close_reason
            FROM items i
            WHERE i.status = 'open'
            AND NOT EXISTS (
                SELECT 1 FROM edges e
                JOIN items blocker ON e.to_id = blocker.id
                WHERE e.from_id = i.id
                AND e.kind = 'blocks'
                AND blocker.status IN ('open', 'in_progress', 'blocked')
            )
            AND NOT EXISTS (
                SELECT 1 FROM edges e
                JOIN items child ON e.from_id = child.id
                WHERE e.to_id = i.id
                AND e.kind = 'parent_child'
                AND child.status IN ('open', 'in_progress', 'blocked')
            )
            ORDER BY i.priority ASC, i.created_at ASC
        "#;

        let mut stmt = self.db.prepare(sql)?;
        let mut items: Vec<Item> = stmt.query_map([], Self::row_to_item)?.filter_map(|r| r.ok()).collect();

        // Load labels for each item
        for item in &mut items {
            let mut label_stmt = self
                .db
                .prepare("SELECT label FROM labels WHERE item_id = ? ORDER BY label")?;
            item.labels = label_stmt
                .query_map(params![item.id], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();
        }

        Ok(items)
    }

    /// Check if an edge exists.
    pub fn edge_exists(&self, from_id: &str, to_id: &str, kind: EdgeKind) -> Result<bool> {
        let kind_str = match kind {
            EdgeKind::Blocks => "blocks",
            EdgeKind::ParentChild => "parent_child",
            EdgeKind::Related => "related",
        };

        let count: i64 = self.db.query_row(
            "SELECT COUNT(*) FROM edges WHERE from_id = ? AND to_id = ? AND kind = ?",
            params![from_id, to_id, kind_str],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    /// Get blocking edges from an item.
    pub fn get_blocking_edges_from(&self, from_id: &str) -> Result<Vec<Edge>> {
        let mut stmt = self.db.prepare(
            r#"
            SELECT from_id, to_id, kind, created_at
            FROM edges
            WHERE from_id = ? AND kind IN ('blocks', 'parent_child')
            "#,
        )?;

        let edges: Vec<Edge> = stmt
            .query_map(params![from_id], |row| {
                let kind_str: String = row.get(2)?;
                let kind = match kind_str.as_str() {
                    "blocks" => EdgeKind::Blocks,
                    "parent_child" => EdgeKind::ParentChild,
                    _ => EdgeKind::Related,
                };
                let created_at_str: String = row.get(3)?;

                Ok(Edge {
                    from_id: row.get(0)?,
                    to_id: row.get(1)?,
                    kind,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    deleted: false,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(edges)
    }

    /// Convert a database row to an Item.
    fn row_to_item(row: &rusqlite::Row) -> rusqlite::Result<Item> {
        let status_str: String = row.get(3)?;
        let status = match status_str.as_str() {
            "open" => Status::Open,
            "in_progress" => Status::InProgress,
            "blocked" => Status::Blocked,
            "closed" => Status::Closed,
            _ => Status::Open,
        };

        let created_at_str: String = row.get(5)?;
        let updated_at_str: String = row.get(6)?;
        let closed_at_str: Option<String> = row.get(7)?;

        Ok(Item {
            id: row.get(0)?,
            title: row.get(1)?,
            description: row.get(2)?,
            status,
            priority: row.get(4)?,
            labels: vec![],
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            closed_at: closed_at_str.and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .ok()
            }),
            close_reason: row.get(8)?,
        })
    }
}

/// Count lines in a file.
fn count_lines(path: &Path) -> Result<usize> {
    if !path.exists() {
        return Ok(0);
    }
    let file = File::open(path).context("Failed to open file for line count")?;
    let reader = BufReader::new(file);
    Ok(reader.lines().count())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_storage() -> (TempDir, Storage) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::init(temp_dir.path()).unwrap();
        (temp_dir, storage)
    }

    #[test]
    fn test_init_creates_files() {
        let temp_dir = TempDir::new().unwrap();
        let _storage = Storage::init(temp_dir.path()).unwrap();

        assert!(temp_dir.path().join(ENGRAM_DIR).exists());
        assert!(temp_dir.path().join(ENGRAM_DIR).join(ITEMS_FILE).exists());
        assert!(temp_dir.path().join(ENGRAM_DIR).join(EDGES_FILE).exists());
        assert!(temp_dir.path().join(ENGRAM_DIR).join(DB_FILE).exists());
    }

    #[test]
    fn test_append_and_get_item() {
        let (_temp_dir, mut storage) = setup_test_storage();

        let now = chrono::Utc::now();
        let item = Item {
            id: "eg-test000001".to_string(),
            title: "Test item".to_string(),
            description: Some("A test description".to_string()),
            status: Status::Open,
            priority: 2,
            labels: vec!["test".to_string(), "example".to_string()],
            created_at: now,
            updated_at: now,
            closed_at: None,
            close_reason: None,
        };

        storage.append_item(&item).unwrap();

        let retrieved = storage.get_item("eg-test000001").unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.title, "Test item");
        assert_eq!(retrieved.labels, vec!["example", "test"]);
    }

    #[test]
    fn test_list_items() {
        let (_temp_dir, mut storage) = setup_test_storage();

        let now = chrono::Utc::now();
        for i in 0..3 {
            let item = Item {
                id: format!("eg-test00000{}", i),
                title: format!("Test item {}", i),
                description: None,
                status: if i == 2 { Status::Closed } else { Status::Open },
                priority: i as u8,
                labels: vec![],
                created_at: now,
                updated_at: now,
                closed_at: if i == 2 { Some(now) } else { None },
                close_reason: None,
            };
            storage.append_item(&item).unwrap();
        }

        let all_items = storage.list_items(None).unwrap();
        assert_eq!(all_items.len(), 3);

        let open_items = storage.list_items(Some(Status::Open)).unwrap();
        assert_eq!(open_items.len(), 2);

        let closed_items = storage.list_items(Some(Status::Closed)).unwrap();
        assert_eq!(closed_items.len(), 1);
    }

    #[test]
    fn test_ready_query() {
        let (_temp_dir, mut storage) = setup_test_storage();

        let now = chrono::Utc::now();

        // Create a blocker item (open)
        let blocker = Item {
            id: "eg-blocker001".to_string(),
            title: "Blocker".to_string(),
            description: None,
            status: Status::Open,
            priority: 0,
            labels: vec![],
            created_at: now,
            updated_at: now,
            closed_at: None,
            close_reason: None,
        };
        storage.append_item(&blocker).unwrap();

        // Create a blocked item
        let blocked = Item {
            id: "eg-blocked001".to_string(),
            title: "Blocked item".to_string(),
            description: None,
            status: Status::Open,
            priority: 1,
            labels: vec![],
            created_at: now,
            updated_at: now,
            closed_at: None,
            close_reason: None,
        };
        storage.append_item(&blocked).unwrap();

        // Create blocking edge
        let edge = Edge {
            from_id: "eg-blocked001".to_string(),
            to_id: "eg-blocker001".to_string(),
            kind: EdgeKind::Blocks,
            created_at: now,
            deleted: false,
        };
        storage.append_edge(&edge).unwrap();

        // Only blocker should be ready
        let ready = storage.ready().unwrap();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "eg-blocker001");
    }
}
