# RECALL.OS - Project Roadmap

**Last Updated**: 2026-02-01
**Version**: 1.0.0
**Status**: Production Ready

---

## Quick Start for New Sessions

```bash
# Install dependencies
npm install

# Start development server
npm run tauri:dev

# Build production installer
npm run tauri:build
```

**Key documentation files**:
- `CLAUDE.md` - Technical architecture and commands
- `PROJECT_ROADMAP.md` - This file (status and next steps)
- `README.md` - User documentation

---

## What's Working (Completed Features)

### Core Infrastructure
| Component | Status | Key Files |
|-----------|--------|-----------|
| Tauri v2 + React 19 | ✅ | `src-tauri/src/lib.rs`, `src/App.tsx` |
| TanStack Query v5 | ✅ | `src/hooks/*.ts` |
| sqlite-vec (vector search) | ✅ | `src-tauri/src/database/mod.rs` |
| FTS5 (full-text search) | ✅ | `src-tauri/src/database/mod.rs` |
| Hybrid retrieval (RRF) | ✅ | `src-tauri/src/rag/retriever.rs` |

### Ingestion Engine
| Feature | Status | Key Files |
|---------|--------|-----------|
| PDF text extraction | ✅ | `src-tauri/src/ingestion/extractor.rs` |
| Windows OCR fallback | ✅ | `src-tauri/src/ingestion/windows_ocr.rs` |
| Token-based chunking | ✅ | `src-tauri/src/ingestion/chunker.rs` |
| Gemini embeddings | ✅ | `src-tauri/src/llm/client.rs` |
| File watcher | ✅ | `src-tauri/src/ingestion/watcher.rs` |
| FFmpeg sidecar | ✅ | `src-tauri/src/ingestion/ffmpeg.rs` |

### RAG Pipeline
| Feature | Status | Key Files |
|---------|--------|-----------|
| Gemini API client | ✅ | `src-tauri/src/llm/client.rs` |
| Rate limiting | ✅ | `src-tauri/src/llm/rate_limiter.rs` |
| Context assembly | ✅ | `src-tauri/src/rag/mod.rs` |
| Citation parsing | ✅ | `src-tauri/src/rag/mod.rs` |
| Conversation history | ✅ | `src-tauri/src/database/models.rs` |

### UI Components
| Component | Status | Key Files |
|-----------|--------|-----------|
| Split-pane layout | ✅ | `src/App.tsx` |
| PDF viewer | ✅ | `src/components/PdfViewer.tsx` |
| PDF highlighting | ✅ | `src/components/PdfViewer.tsx` |
| Citation chips | ✅ | `src/components/CitationChip.tsx` |
| Settings panel | ✅ | `src/components/SettingsModal.tsx` |
| License modal | ✅ | `src/components/LicenseModal.tsx` |
| Dark mode | ✅ | Tailwind config |
| BYOK API key | ✅ | Settings UI |
| Custom notifications | ✅ | `src-tauri/src/notifications/` |

### Security (2026-02-01)
| Fix | Status | Details |
|-----|--------|---------|
| XSS in ChatPanel | ✅ | Replaced dangerouslySetInnerHTML with react-markdown + rehype-sanitize |
| Tauri capabilities | ✅ | Restricted to minimal required permissions |
| CSP hardening | ✅ | Removed unsafe-eval, unsafe-inline where possible |
| FTS5 query escaping | ✅ | Sanitize special characters to prevent injection |
| .env removal | ✅ | Deleted exposed API key file |

### Monetization (2026-02-01)
| Feature | Status | Details |
|---------|--------|---------|
| License validation | ✅ | `src-tauri/src/commands/license.rs` |
| Trial mode (25 docs) | ✅ | Limit enforced in ingestion commands |
| License UI | ✅ | `src/components/LicenseModal.tsx` |
| Trial usage display | ✅ | Shows docs used/limit in sidebar |

---

## Production Release Checklist

### Completed
- [x] Security audit and fixes
- [x] License validation system
- [x] Trial mode with document limits
- [x] README documentation
- [x] Production build configuration

### Pending (User Actions)
- [ ] Enable GitHub Pages (Settings > Pages > Deploy from branch main, folder /docs)
- [ ] Close running dev server and test build
- [ ] Set up LemonSqueezy account for payments
- [ ] Purchase code signing certificate (optional)
- [ ] Test installer on clean Windows machine

---

## Architecture Reference

### File Structure
```
RECALL.OS/
├── src/                          # React frontend
│   ├── components/
│   │   ├── SourcePanel.tsx       # Source viewer
│   │   ├── PdfViewer.tsx         # PDF viewer
│   │   ├── ChatPanel.tsx         # Chat interface
│   │   ├── LicenseModal.tsx      # License management
│   │   └── SettingsModal.tsx     # Settings UI
│   ├── hooks/
│   │   ├── useDocuments.ts       # Document queries
│   │   ├── useRag.ts             # RAG queries
│   │   ├── useLicense.ts         # License queries
│   │   └── useSettings.ts        # Settings queries
│   └── types.ts                  # TypeScript types
│
├── src-tauri/                    # Rust backend
│   ├── src/
│   │   ├── commands/             # Tauri IPC handlers
│   │   │   ├── license.rs        # License validation
│   │   │   └── ingestion.rs      # File ingestion (with trial limits)
│   │   ├── database/             # SQLite + vec0 + FTS5
│   │   ├── ingestion/            # PDF/video/audio extraction
│   │   ├── llm/                  # Gemini API client
│   │   ├── rag/                  # Retrieval + generation
│   │   └── lib.rs                # Entry point
│   ├── capabilities/
│   │   └── default.json          # Restricted permissions
│   ├── resources/
│   │   ├── vec0.dll              # sqlite-vec extension
│   │   └── ffmpeg.exe            # Media processing
│   └── tauri.conf.json           # Tauri configuration
│
├── CLAUDE.md                     # Technical docs
├── PROJECT_ROADMAP.md            # This file
└── README.md                     # User documentation
```

### License Key Format
```
RO-XXXX-XXXX-XXXX

- Prefix: "RO" (RECALL.OS)
- Segments: 4 alphanumeric characters each
- Validation: Local checksum + future API validation
```

### Trial Limits
- **Documents**: 25 maximum
- **Features**: All features available
- **Upgrade**: Purchase license for unlimited

---

## Development Commands

```bash
# Install dependencies
npm install

# Development (hot reload)
npm run tauri:dev

# Build production installer
npm run tauri:build

# Build debug version
npm run tauri:build:debug

# Type check only
npm run build
```

---

## Next Steps

### Immediate
1. **Test production build** - Close dev server, run `npm run tauri:build`
2. **Set up LemonSqueezy** - For license key sales
3. **Enable GitHub Pages** - Settings > Pages > Deploy from /docs folder

### Future Enhancements
- Tiered retrieval (top 10 fast, top 50 deep)
- Image file ingestion via Gemini vision
- Cloud backup/sync (optional)
- macOS/Linux support

---

## Success Criteria ✅

- [x] PDF ingestion with OCR fallback
- [x] Hybrid search returns relevant results
- [x] RAG generates accurate answers with citations
- [x] PDF viewer with highlighting
- [x] Video/audio playback with timestamp seeking
- [x] Custom notification windows
- [x] Security audit completed
- [x] License system implemented
- [x] README documentation
- [ ] Code signed installer (optional)
