<div align="center">

<img src="src-tauri/icons/128x128.png" alt="Recall.OS" width="80">

# Recall.OS

**Personal AI Memory — Own Your Intelligence**

[![Windows](https://img.shields.io/badge/Windows-10%2F11-0078D6?style=flat&logo=windows&logoColor=white)](https://github.com/Project-Intuitus/Recall-OS/releases)
[![License](https://img.shields.io/badge/License-Proprietary-06b6d4?style=flat)](LICENSE)
[![Version](https://img.shields.io/badge/Version-1.0.1-10b981?style=flat)](https://github.com/Project-Intuitus/Recall-OS/releases)

<br>

[**Download**](https://github.com/Project-Intuitus/Recall-OS/releases) · [**Website**](https://projectintuitus.com) · [**Documentation**](#quick-start)

</div>

<br>

## The Problem

Every "free" AI is a funnel for your data. Every cloud service is a subscription that watches.

**Recall.OS** is different. It's a sovereign AI tool that runs entirely on your machine, indexes your documents locally, and answers your questions with precise citations — all without your data ever leaving your device.

<br>

## Features

<table>
<tr>
<td width="50%">

### 📄 Universal Document Support
Index PDFs, text files, Word documents, images, videos, and audio. Scanned documents? No problem — built-in OCR extracts every word.

### 🔍 Hybrid AI Search
Vector similarity + full-text search working together. Find what you need even when you can't remember the exact words.

### 💬 Q&A with Citations
Ask questions in plain English. Get answers with exact sources — page numbers, timestamps, document links.

</td>
<td width="50%">

### 📸 Screen Capture Intelligence
Capture screenshots with a hotkey. Recall.OS extracts and indexes the text automatically.

### 📁 Folder Sync
Point Recall.OS at your folders. New files are indexed automatically in the background.

### 🔒 100% Local & Private
Your data never leaves your machine. No cloud. No telemetry. No corporate surveillance.

</td>
</tr>
</table>

<br>

## Quick Start

### 1. Download & Install

Get the latest installer from [**Releases**](https://github.com/Project-Intuitus/Recall-OS/releases).

### 2. Get Your API Key

Recall.OS uses a **Bring Your Own Key (BYOK)** model. Get a free API key from [Google AI Studio](https://aistudio.google.com/apikey).

### 3. Start Indexing

Drag and drop files, or set up folder sync to automatically index new documents.

<br>

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Recall.OS                                 │
├─────────────────────────────────────────────────────────────────┤
│  Frontend                                                        │
│  ├── React 19 + TypeScript                                       │
│  ├── TanStack Query (async state)                               │
│  └── Tailwind CSS (glassmorphism UI)                            │
├─────────────────────────────────────────────────────────────────┤
│  Backend (Tauri v2 / Rust)                                      │
│  ├── Document Ingestion Pipeline                                │
│  │   ├── PDF extraction + OCR fallback                          │
│  │   ├── Video/Audio transcription (FFmpeg + Gemini)            │
│  │   └── Image OCR (Gemini Vision)                              │
│  ├── Hybrid Search Engine                                       │
│  │   ├── Vector search (sqlite-vec, 768-dim embeddings)         │
│  │   └── Full-text search (FTS5 with BM25)                      │
│  └── RAG Pipeline                                               │
│       ├── Query embedding                                        │
│       ├── Reciprocal rank fusion                                │
│       └── Context-aware generation with citations               │
├─────────────────────────────────────────────────────────────────┤
│  Storage                                                         │
│  └── SQLite (documents, chunks, vectors, conversations)         │
└─────────────────────────────────────────────────────────────────┘
```

<br>

## Tech Stack

| Layer | Technology |
|-------|-----------|
| **Frontend** | React 19, TypeScript, TanStack Query, Tailwind CSS |
| **Backend** | Rust, Tauri v2 |
| **Database** | SQLite + sqlite-vec (vectors) + FTS5 (full-text) |
| **AI** | Google Gemini API (text-embedding-004, gemini-2.0-flash) |
| **Media** | FFmpeg (video/audio processing) |

<br>

## Build from Source

**Prerequisites:** Node.js 18+, Rust 1.84+, Windows 10/11

```bash
# Clone the repository
git clone https://github.com/Project-Intuitus/Recall-OS.git
cd Recall-OS

# Install dependencies
npm install

# Development mode
npm run tauri:dev

# Build production installer
npm run tauri:build
```

### Required Resources

Place these in `src-tauri/resources/`:

| File | Source |
|------|--------|
| `vec0.dll` | [sqlite-vec releases](https://github.com/asg017/sqlite-vec/releases) |
| `ffmpeg.exe` | [gyan.dev FFmpeg builds](https://www.gyan.dev/ffmpeg/builds/) |

<br>

## Pricing

| | Trial | Licensed |
|---|-------|----------|
| **Documents** | 25 | Unlimited |
| **Features** | Full access | Full access |
| **Price** | Free | $29.99 (one-time) |

<br>

## Contributing

We welcome contributions. Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

<br>

## Support

- [**Support Guide**](SUPPORT.md) — Troubleshooting & common issues
- [**GitHub Issues**](https://github.com/Project-Intuitus/Recall-OS/issues) — Bug reports & feature requests
- [**Website**](https://projectintuitus.com) — Product information

<br>

---

<div align="center">

**Part of the [Project Intuitus](https://projectintuitus.com) ecosystem**

*Sovereign AI tools that run locally, respect privacy, and work for you.*

<br>

Built with [Tauri](https://tauri.app/) · [React](https://react.dev/) · [Gemini](https://ai.google.dev/)

</div>
