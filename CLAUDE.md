# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Related Documentation

- **PROJECT_ROADMAP.md** - Detailed status, remaining tasks, and implementation phases
- **CLAUDE.md** - This file (technical architecture and commands)

---

## Project Status (Updated: 2026-02-01)

### ✅ COMPLETED

**Core Infrastructure**
- [x] Tauri v2 project with React 19 frontend
- [x] TanStack Query v5 for state management
- [x] sqlite-vec extension loading in Rust (`vec0.dll`)
- [x] Database schema (documents, chunks, chunks_fts, vec_chunks)
- [x] Database migration system
- [x] Tauri capabilities/ACL configuration (restricted permissions)
- [x] FFmpeg bundled as sidecar (`ffmpeg.exe`)

**Ingestion Engine**
- [x] PDF text extraction with `pdf-extract` crate
- [x] Windows OCR fallback for scanned PDFs (Windows.Data.Pdf + Windows.Media.Ocr)
- [x] Character-based chunking (~512 tokens, 50 overlap) with tiktoken
- [x] Embedding generation with Gemini `text-embedding-004`
- [x] File watcher for automatic ingestion
- [x] Ingestion progress events via Tauri
- [x] Trial mode document limits (25 docs for trial users)

**RAG Pipeline**
- [x] Gemini API client with rate limiting (leaky bucket)
- [x] Hybrid retrieval (vector KNN + FTS5 BM25)
- [x] Reciprocal rank fusion (k=60)
- [x] Context assembly from top chunks
- [x] Citation parsing and linking
- [x] Conversation history support
- [x] FTS5 query escaping (security fix)

**UI Components**
- [x] Split-pane layout (Chat + Source Inspector)
- [x] PDF viewer with page navigation and zoom (`react-pdf`)
- [x] PDF text highlighting on matched content
- [x] Tauri asset protocol for local file loading
- [x] Citation chip components in chat
- [x] Ingestion progress indicators
- [x] Dark mode (Tailwind)
- [x] BYOK API key configuration screen
- [x] Settings panel with watched folders
- [x] Custom notification windows (transparent, no shadow, auto-dismiss)
- [x] Video player with timestamp seeking (`VideoPlayer.tsx`)
- [x] Audio player with playback controls (`AudioPlayer.tsx`)
- [x] Chunk text display with page section formatting (`FormattedChunkContent`)
- [x] License modal with trial usage display (`LicenseModal.tsx`)
- [x] Secure markdown rendering with react-markdown + rehype-sanitize

**Security (2026-02-01)**
- [x] XSS vulnerability fixed in ChatPanel.tsx
- [x] Tauri capabilities restricted (no broad fs/shell permissions)
- [x] CSP hardened (removed unsafe-eval, external CDNs)
- [x] FTS5 query injection prevention
- [x] Exposed .env file deleted

**Monetization (2026-02-01)**
- [x] License validation system (`src-tauri/src/commands/license.rs`)
- [x] License key format: `RO-XXXX-XXXX-XXXX`
- [x] Trial mode with 25 document limit
- [x] License UI with activation/deactivation
- [x] Trial usage display in sidebar

### ❌ NOT STARTED

**Advanced Features**
- [ ] Tiered retrieval (top 10 fast, top 50 deep)
- [ ] Image file ingestion via Gemini vision
- [ ] LemonSqueezy integration for license sales
- [ ] Code signing certificate

### Known Issues

1. **Dev server PATH**: When starting via Claude, need `export PATH="$PATH:/c/Users/USERNAME/.cargo/bin"` before `npm run tauri dev`.

2. **API Key**: The exposed API key in .env was deleted - user must revoke it at https://aistudio.google.com/apikey

---

## Build & Development Commands

```bash
# Install dependencies
pnpm install

# Development (runs both Vite frontend and Tauri backend)
pnpm tauri dev

# Build production Windows installer
pnpm tauri build

# Frontend only (for UI development without backend)
pnpm dev

# Type check frontend
pnpm build
```

**Required binaries in `src-tauri/resources/`:**
- `vec0.dll` - sqlite-vec extension from https://github.com/asg017/sqlite-vec/releases
- `ffmpeg.exe` - static build from https://www.gyan.dev/ffmpeg/builds/

## Architecture Overview

RECALL.OS is a Tauri v2 desktop app with a Rust backend and React 19 frontend. It provides local-first document ingestion, hybrid search (vector + FTS), and RAG-powered Q&A.

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

**Custom Notification System** (`src-tauri/src/notifications/`):
- Creates transparent, frameless popup windows for related document alerts
- Uses pull-based data retrieval (window requests data via `notification_window_ready` command)
- Windows-specific: `shadow(false)` removes border outline on transparent windows
- Auto-dismisses after 5 seconds with animated progress bar

### State Management

- **Backend**: `AppState` in `state.rs` holds shared state wrapped in `Arc<RwLock<>>`:
  - `database`, `llm_client`, `ingestion_engine`, `rag_engine`, `settings`, `watcher_manager`
- **Frontend**: TanStack Query v5 with hooks in `src/hooks/`
  - Query keys: `["documents"]`, `["stats"]`, `["settings"]`, `["chunks", documentId]`

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
3. **chunker.rs** - character-based chunking (~512 tokens, 50 overlap) with lazy-loaded tiktoken tokenizer
4. **watcher.rs** / **watcher_manager.rs** - file system watching with auto-ingest

Progress stages: `queued` → `extracting` → `chunking` → `embedding` → `indexing` → `completed`

### RAG Pipeline (`src-tauri/src/rag/`)

1. Query embedding via Gemini `text-embedding-004`
2. **HybridRetriever** in `retriever.rs`:
   - Parallel vector search (sqlite-vec KNN) + FTS5 search
   - Reciprocal rank fusion with k=60
3. Context assembly from top chunks
4. LLM generation with conversation history support
5. Citation extraction from response

### LLM Module (`src-tauri/src/llm/`)

- **client.rs** - Gemini API client with resumable file uploads
- **rate_limiter.rs** - leaky bucket algorithm (60 RPM default)
- **LlmProvider** trait for future provider flexibility

Models (currently in use):
- `gemini-2.0-flash` - ingestion, transcription, RAG generation
- `text-embedding-004` - embeddings (768 dimensions)

Note: Plan mentions Gemini 3 models but 2.0-flash is currently implemented. Update when Gemini 3 becomes stable.

API endpoint: `https://generativelanguage.googleapis.com/v1beta/models`

## Key Tauri Commands

**Ingestion**: `ingest_file`, `ingest_directory`, `reingest_document`, `cancel_ingestion`, `get_ingestion_progress`

**Database**: `get_documents`, `get_document`, `delete_document`, `get_chunks_for_document`, `get_ingestion_stats`

**Search**: `search_documents`, `hybrid_search`

**RAG**: `query`, `query_with_sources`

**Settings**: `get_settings`, `update_settings`, `validate_api_key`

**Watcher**: `get_watcher_status`, `start_watcher`, `stop_watcher`, `add_watched_folder`, `remove_watched_folder`, `toggle_auto_ingest`

## Key Patterns

- **Error handling**: `RecallError` enum in `error.rs` implements `Serialize` for Tauri IPC
- **Tauri permissions**: ACL in `capabilities/default.json`, only allows `generativelanguage.googleapis.com`
- **Async locks**: Use `parking_lot::RwLock` with short critical sections, clone data before `.await`
- **Windows-specific**: `#[cfg(windows)]` blocks for Windows OCR in `windows_ocr.rs`

## Settings

Stored in `%APPDATA%/com.recallos.app/settings.json`:
- `gemini_api_key` - required for operation
- `chunk_size` (512), `chunk_overlap` (50), `max_context_chunks` (20)
- `watched_folders`, `auto_ingest_enabled`
- `embedding_model`, `ingestion_model`, `reasoning_model`
- `license_key`, `license_activated_at` - license validation

## Quick Start for Development

```bash
# 1. Ensure cargo is in PATH (Windows)
export PATH="$PATH:/c/Users/USERNAME/.cargo/bin"

# 2. Start dev server
cd C:/Users/USERNAME/Desktop/RECALL.OS
npm run tauri:dev

# 3. Build production installer
npm run tauri:build
```

## Next Session Priority

1. **CRITICAL: Revoke exposed API key**
   - Go to https://aistudio.google.com/apikey
   - Revoke the exposed key `***REDACTED_API_KEY***`

2. **Test production build**
   - Close any running dev servers
   - Run `npm run tauri:build`
   - Test installer on clean Windows system

3. **Set up monetization**
   - Create LemonSqueezy account
   - Configure license key webhook
   - Optional: purchase code signing certificate

4. **Future enhancements**
   - Tiered retrieval (top 10 fast, top 50 deep)
   - Image file ingestion via Gemini vision
   - Backend API for license validation
