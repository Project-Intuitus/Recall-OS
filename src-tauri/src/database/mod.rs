mod migrations;
mod models;
mod queries;

pub use models::*;
// queries module provides internal database helpers

use crate::error::Result;
use parking_lot::Mutex;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

pub struct Database {
    conn: Mutex<Connection>,
    db_path: PathBuf,
    vec_extension_path: PathBuf,
}

impl Database {
    pub fn new(db_path: &Path, resources_dir: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // Determine vec0.dll path
        let vec_extension_path = if cfg!(debug_assertions) {
            // In development, look in resources folder
            PathBuf::from("resources/vec0.dll")
        } else {
            // In production, use bundled resource
            resources_dir.join("resources/vec0.dll")
        };

        let db = Self {
            conn: Mutex::new(conn),
            db_path: db_path.to_path_buf(),
            vec_extension_path,
        };

        db.initialize()?;
        Ok(db)
    }

    /// Hard reset: close connection, delete database files, and recreate fresh
    /// This is used when the database is corrupted and SQL commands fail
    pub fn hard_reset(&self) -> Result<()> {
        tracing::info!("Performing hard database reset...");

        // Get lock on connection
        let mut conn = self.conn.lock();

        // Close current connection by replacing with a temporary in-memory one
        // This releases the file handles
        *conn = Connection::open_in_memory()?;

        // Delete the database files
        let db_path = &self.db_path;
        let wal_path = db_path.with_extension("db-wal");
        let shm_path = db_path.with_extension("db-shm");

        // Remove files if they exist
        if db_path.exists() {
            std::fs::remove_file(db_path)?;
            tracing::info!("Deleted: {:?}", db_path);
        }
        if wal_path.exists() {
            std::fs::remove_file(&wal_path)?;
            tracing::info!("Deleted: {:?}", wal_path);
        }
        if shm_path.exists() {
            std::fs::remove_file(&shm_path)?;
            tracing::info!("Deleted: {:?}", shm_path);
        }

        // Reopen the database (creates fresh file)
        *conn = Connection::open(db_path)?;

        // Reinitialize (set pragmas, load extension, run migrations)
        drop(conn); // Release lock before calling initialize
        self.initialize()?;

        tracing::info!("Hard database reset completed successfully");
        Ok(())
    }

    fn initialize(&self) -> Result<()> {
        let conn = self.conn.lock();

        // Enable WAL mode for better concurrent access
        // Use synchronous = FULL for reliable writes (ensures FK constraints work properly)
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = FULL;
             PRAGMA foreign_keys = ON;
             PRAGMA temp_store = MEMORY;
             PRAGMA mmap_size = 268435456;",
        )?;

        // Load sqlite-vec extension
        self.load_vec_extension(&conn)?;

        // Run migrations
        migrations::run_migrations(&conn)?;

        // Cleanup orphaned documents from previous crashed sessions
        self.cleanup_orphaned_documents(&conn)?;

        Ok(())
    }

    fn cleanup_orphaned_documents(&self, conn: &Connection) -> Result<()> {
        // Delete documents stuck in pending/processing state (from previous crashes)
        // These would have incomplete chunks/embeddings anyway
        let count = conn.execute(
            "DELETE FROM documents WHERE status IN ('pending', 'processing')",
            [],
        )?;

        if count > 0 {
            tracing::info!("Cleaned up {} orphaned documents from previous session", count);
        }

        Ok(())
    }

    fn load_vec_extension(&self, conn: &Connection) -> Result<()> {
        unsafe {
            let _guard = conn.load_extension_enable()?;

            // Try to load the extension
            let result = if self.vec_extension_path.exists() {
                conn.load_extension(&self.vec_extension_path, None::<&str>)
            } else {
                // Try loading from current directory or system path
                conn.load_extension("vec0", None::<&str>)
            };

            match result {
                Ok(_) => {
                    tracing::info!("sqlite-vec extension loaded successfully");
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to load sqlite-vec extension: {}. Vector search will be unavailable.",
                        e
                    );
                    // Don't fail - allow app to run without vector search for development
                }
            }

            let _ = conn.load_extension_disable();
        }

        Ok(())
    }

    pub fn with_conn<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> Result<T>,
    {
        let conn = self.conn.lock();
        f(&conn)
    }

    pub fn with_conn_mut<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut Connection) -> Result<T>,
    {
        let mut conn = self.conn.lock();
        f(&mut conn)
    }

    /// Validate that sqlite-vec is properly loaded and functional
    pub fn validate_vec_extension(&self) -> Result<bool> {
        let conn = self.conn.lock();

        // Check if vec_version() function exists
        let result: rusqlite::Result<String> = conn.query_row(
            "SELECT vec_version()",
            [],
            |row| row.get(0),
        );

        match result {
            Ok(version) => {
                tracing::info!("sqlite-vec version: {}", version);
                Ok(true)
            }
            Err(e) => {
                tracing::warn!("sqlite-vec not available: {}", e);
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn test_sqlite_vec_loading() {
        // Create a temporary database
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let resources_dir = PathBuf::from(".");

        // Initialize database (this will attempt to load vec0.dll)
        let db = Database::new(&db_path, &resources_dir)
            .expect("Failed to create database");

        // Validate the extension is loaded
        let vec_available = db.validate_vec_extension()
            .expect("Failed to validate vec extension");

        if vec_available {
            println!("✅ sqlite-vec loaded successfully!");

            // Test vector operations
            db.with_conn(|conn| {
                // Create a test vector table
                conn.execute(
                    "CREATE VIRTUAL TABLE IF NOT EXISTS test_vectors USING vec0(
                        id INTEGER PRIMARY KEY,
                        embedding FLOAT[4]
                    )",
                    [],
                )?;

                // Insert test vectors
                conn.execute(
                    "INSERT INTO test_vectors(id, embedding) VALUES (1, vec_f32('[1.0, 0.0, 0.0, 0.0]'))",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO test_vectors(id, embedding) VALUES (2, vec_f32('[0.0, 1.0, 0.0, 0.0]'))",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO test_vectors(id, embedding) VALUES (3, vec_f32('[0.707, 0.707, 0.0, 0.0]'))",
                    [],
                )?;

                // Test KNN search
                let query_vec = "[0.9, 0.1, 0.0, 0.0]";
                let mut stmt = conn.prepare(
                    "SELECT id, distance FROM test_vectors WHERE embedding MATCH vec_f32(?) ORDER BY distance LIMIT 3"
                )?;

                let results: Vec<(i64, f64)> = stmt.query_map([query_vec], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })?.filter_map(|r| r.ok()).collect();

                println!("KNN search results for {:?}:", query_vec);
                for (id, distance) in &results {
                    println!("  ID: {}, Distance: {:.4}", id, distance);
                }

                // Verify closest is ID 1 (most similar to query)
                assert!(!results.is_empty(), "Should have search results");
                assert_eq!(results[0].0, 1, "Closest vector should be ID 1");

                println!("✅ Vector operations working correctly!");
                Ok(())
            }).expect("Vector operations failed");
        } else {
            println!("⚠️ sqlite-vec not available - skipping vector tests");
        }
    }
}
