use crate::error::Result;
use rusqlite::Connection;

const MIGRATIONS: &[&str] = &[
    // Migration 1: Schema version tracking (must be first!)
    r#"
    CREATE TABLE IF NOT EXISTS schema_version (
        version INTEGER PRIMARY KEY,
        applied_at TEXT NOT NULL DEFAULT (datetime('now'))
    );
    "#,
    // Migration 2: Core tables
    r#"
    -- Documents table: stores metadata about ingested files
    CREATE TABLE IF NOT EXISTS documents (
        id TEXT PRIMARY KEY,
        title TEXT NOT NULL,
        file_path TEXT NOT NULL UNIQUE,
        file_type TEXT NOT NULL,
        file_size INTEGER NOT NULL,
        file_hash TEXT NOT NULL,
        mime_type TEXT,
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at TEXT NOT NULL DEFAULT (datetime('now')),
        ingested_at TEXT,
        status TEXT NOT NULL DEFAULT 'pending',
        error_message TEXT,
        metadata TEXT DEFAULT '{}'
    );

    -- Chunks table: stores text chunks from documents
    CREATE TABLE IF NOT EXISTS chunks (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
        chunk_index INTEGER NOT NULL,
        content TEXT NOT NULL,
        token_count INTEGER NOT NULL,
        start_offset INTEGER,
        end_offset INTEGER,
        page_number INTEGER,
        timestamp_start REAL,
        timestamp_end REAL,
        metadata TEXT DEFAULT '{}',
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        UNIQUE(document_id, chunk_index)
    );

    -- Create indices for chunks
    CREATE INDEX IF NOT EXISTS idx_chunks_document_id ON chunks(document_id);
    CREATE INDEX IF NOT EXISTS idx_chunks_page_number ON chunks(page_number);

    -- FTS5 table for full-text search
    CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(
        content,
        content='chunks',
        content_rowid='id',
        tokenize='porter unicode61'
    );

    -- Triggers to keep FTS in sync
    CREATE TRIGGER IF NOT EXISTS chunks_ai AFTER INSERT ON chunks BEGIN
        INSERT INTO chunks_fts(rowid, content) VALUES (new.id, new.content);
    END;

    CREATE TRIGGER IF NOT EXISTS chunks_ad AFTER DELETE ON chunks BEGIN
        INSERT INTO chunks_fts(chunks_fts, rowid, content) VALUES('delete', old.id, old.content);
    END;

    CREATE TRIGGER IF NOT EXISTS chunks_au AFTER UPDATE ON chunks BEGIN
        INSERT INTO chunks_fts(chunks_fts, rowid, content) VALUES('delete', old.id, old.content);
        INSERT INTO chunks_fts(rowid, content) VALUES (new.id, new.content);
    END;
    "#,
    // Migration 3: Vector table using sqlite-vec
    r#"
    -- Vector embeddings table using sqlite-vec
    -- Note: This will fail gracefully if vec0 extension not loaded
    CREATE VIRTUAL TABLE IF NOT EXISTS vec_chunks USING vec0(
        chunk_id INTEGER PRIMARY KEY,
        embedding FLOAT[768]
    );
    "#,
    // Migration 4: Conversations and messages for RAG history
    r#"
    -- Conversations table
    CREATE TABLE IF NOT EXISTS conversations (
        id TEXT PRIMARY KEY,
        title TEXT,
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at TEXT NOT NULL DEFAULT (datetime('now'))
    );

    -- Messages table
    CREATE TABLE IF NOT EXISTS messages (
        id TEXT PRIMARY KEY,
        conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
        role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system')),
        content TEXT NOT NULL,
        citations TEXT DEFAULT '[]',
        created_at TEXT NOT NULL DEFAULT (datetime('now'))
    );

    CREATE INDEX IF NOT EXISTS idx_messages_conversation_id ON messages(conversation_id);
    "#,
];

pub fn run_migrations(conn: &Connection) -> Result<()> {
    // Check if schema_version table exists
    let table_exists: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='schema_version'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);

    let current_version: i32 = if table_exists {
        conn.query_row("SELECT COALESCE(MAX(version), 0) FROM schema_version", [], |row| {
            row.get(0)
        })
        .unwrap_or(0)
    } else {
        0
    };

    for (i, migration) in MIGRATIONS.iter().enumerate() {
        let version = (i + 1) as i32;
        if version > current_version {
            tracing::info!("Running migration {}", version);

            // Execute the entire migration batch
            match conn.execute_batch(migration) {
                Ok(_) => {}
                Err(e) => {
                    // Allow vec_chunks creation to fail if extension not loaded
                    if migration.contains("vec_chunks") {
                        tracing::warn!(
                            "Skipping vec_chunks table creation (extension not loaded): {}",
                            e
                        );
                    } else {
                        return Err(e.into());
                    }
                }
            }

            // Record migration
            conn.execute(
                "INSERT OR REPLACE INTO schema_version (version) VALUES (?)",
                [version],
            )?;
        }
    }

    tracing::info!("Database migrations complete");
    Ok(())
}
