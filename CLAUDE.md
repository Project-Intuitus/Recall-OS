# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```bash
# Install dependencies
npm install

# Development (runs both Vite frontend and Tauri backend)
npm run tauri:dev

# Build production Windows installer
npm run tauri:build

# Frontend only (for UI development without backend)
npm run dev

# Type check frontend
npm run build
```

**Required binaries in `src-tauri/resources/`:**
- `vec0.dll` - sqlite-vec extension from https://github.com/asg017/sqlite-vec/releases
- `ffmpeg.exe` - static build from https://www.gyan.dev/ffmpeg/builds/

## Architecture Overview

RECALL.OS is a Tauri v2 desktop app with a Rust backend and React 19 frontend. It provides local-first document ingestion, hybrid search (vector + FTS), and RAG-powered Q&A using the Gemini API (BYOK model).

### Frontend-Backend Communication

Tauri commands in `src-tauri/src/commands/` are invoked from React via `@tauri-apps/api/core`:
```typescript
import { invoke } from "@tauri-apps/api/core";
const result = await invoke<Document>("ingest_file", { path });
```

Commands registered in `src-tauri/src/lib.rs` via `tauri::generate_handler![]`.

Backend emits events to frontend via `app_handle.emit()`:
- `"ingestion-progress"` - real-time ingestion status updates
- `"auto-ingest-complete"` - when file watcher completes auto-ingestion
- `"document-deleted"` - when a document is removed

Frontend listens in `App.tsx` via `@tauri-apps/api/event.listen()`.

### State Management

- **Backend**: `AppState` in `state.rs` holds shared state wrapped in `Arc<RwLock<>>`:
  - `database`, `llm_client`, `ingestion_engine`, `rag_engine`, `settings`, `watcher_manager`
- **Frontend**: TanStack Query v5 with hooks in `src/hooks/`
  - Query keys: `["documents"]`, `["stats"]`, `["settings"]`, `["chunks", documentId]`, `["license"]`

### Database Layer

SQLite with two virtual table extensions loaded in `database/mod.rs`:
- **FTS5** - full-text search on `chunks_fts` table with BM25 scoring
- **sqlite-vec** - vector similarity on `vec_chunks` table (768-dim embeddings)

Tables: `documents`, `chunks`, `chunks_fts`, `vec_chunks`, `conversations`, `conversation_messages`

### Ingestion Pipeline (`src-tauri/src/ingestion/`)

1. **extractor.rs** - content extraction:
   - PDF: `pdf-extract` crate for text, Windows OCR fallback for scanned docs
   - Video/Audio: FFmpeg sidecar for frame extraction and audio conversion
   - Images: Gemini vision API for descriptions
2. **windows_ocr.rs** - Windows.Data.Pdf + Windows.Media.Ocr APIs (no external DLLs)
3. **chunker.rs** - token-based chunking (~512 tokens, 50 overlap) with tiktoken
4. **watcher.rs** / **watcher_manager.rs** - file system watching with auto-ingest

Progress stages: `queued` → `extracting` → `chunking` → `embedding` → `indexing` → `completed`

### RAG Pipeline (`src-tauri/src/rag/`)

1. Query embedding via Gemini `text-embedding-004`
2. **HybridRetriever** in `retriever.rs`:
   - Parallel vector search (sqlite-vec KNN) + FTS5 search
   - Reciprocal rank fusion with k=60
   - FTS5 queries are sanitized to prevent injection
3. Context assembly from top chunks
4. LLM generation with conversation history support
5. Citation extraction from response

### LLM Module (`src-tauri/src/llm/`)

- **client.rs** - Gemini API client with resumable file uploads
- **rate_limiter.rs** - leaky bucket algorithm (60 RPM default)

Models:
- `gemini-2.0-flash` - ingestion, transcription, RAG generation
- `text-embedding-004` - embeddings (768 dimensions)

### License System (`src-tauri/src/commands/license.rs`)

- License key format: `RO-XXXX-XXXX-XXXX` with checksum validation
- Trial mode: 25 document limit enforced in `ingestion.rs`
- License status stored in settings.json

## Key Tauri Commands

**Ingestion**: `ingest_file`, `ingest_directory`, `reingest_document`, `cancel_ingestion`, `get_ingestion_progress`

**Database**: `get_documents`, `get_document`, `delete_document`, `get_chunks_for_document`, `get_ingestion_stats`

**Search**: `search_documents`, `hybrid_search`

**RAG**: `query`, `query_with_sources`

**Settings**: `get_settings`, `update_settings`, `validate_api_key`

**License**: `get_license_status`, `activate_license`, `deactivate_license`

**Watcher**: `get_watcher_status`, `start_watcher`, `stop_watcher`, `add_watched_folder`, `remove_watched_folder`, `toggle_auto_ingest`

## Key Patterns

- **Error handling**: `RecallError` enum in `error.rs` implements `Serialize` for Tauri IPC
- **Tauri permissions**: ACL in `capabilities/default.json`, restricted to minimal permissions
- **Network**: Only allows `generativelanguage.googleapis.com` in CSP
- **Async locks**: Use `parking_lot::RwLock` with short critical sections, clone data before `.await`
- **Windows-specific**: `#[cfg(windows)]` blocks for Windows OCR in `windows_ocr.rs`
- **Security**: Markdown rendered with react-markdown + rehype-sanitize to prevent XSS

## Settings

Stored in `%APPDATA%/com.recallos.app/settings.json`:
- `gemini_api_key` - required for operation
- `chunk_size` (512), `chunk_overlap` (50), `max_context_chunks` (20)
- `watched_folders`, `auto_ingest_enabled`
- `embedding_model`, `ingestion_model`, `reasoning_model`
- `license_key`, `license_activated_at` - license validation

## Custom Notification System

`src-tauri/src/notifications/`:
- Creates transparent, frameless popup windows for related document alerts
- Uses pull-based data retrieval (window requests data via `notification_window_ready` command)
- Windows-specific: `shadow(false)` removes border outline on transparent windows
- Auto-dismisses after 5 seconds with animated progress bar
