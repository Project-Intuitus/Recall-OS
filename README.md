# RECALL.OS

<p align="center">
  <img src="src-tauri/icons/128x128.png" alt="RECALL.OS Logo" width="128">
</p>

<p align="center">
  <strong>Your Personal AI Memory</strong><br>
  A local-first desktop app that indexes your documents for intelligent AI-powered search and Q&A.
</p>

<p align="center">
  <a href="https://project-intuitus.github.io/Recall-OS/">Website</a> •
  <a href="https://github.com/Project-Intuitus/Recall-OS/releases">Download</a> •
  <a href="#installation">Installation</a> •
  <a href="#features">Features</a>
</p>

---

## Features

- **Smart Document Indexing** - PDFs, text, video, audio, images with OCR support
- **Hybrid AI Search** - Vector similarity + full-text search for best results
- **AI Q&A with Citations** - Ask questions, get answers with source links
- **Screen Capture** - Index screenshots with hotkey capture
- **Folder Sync** - Auto-index new files in watched folders
- **100% Local** - Your data never leaves your device

## Installation

### Download

Get the latest installer from [Releases](https://github.com/Project-Intuitus/Recall-OS/releases).

### Build from Source

**Prerequisites:** Node.js 18+, Rust 1.84+, Windows 10/11

```bash
# Clone
git clone https://github.com/Project-Intuitus/Recall-OS.git
cd Recall-OS

# Install dependencies
npm install

# Development
npm run tauri:dev

# Build installer
npm run tauri:build
```

**Required resources** in `src-tauri/resources/`:
- [`vec0.dll`](https://github.com/asg017/sqlite-vec/releases) - sqlite-vec extension
- [`ffmpeg.exe`](https://www.gyan.dev/ffmpeg/builds/) - Media processing

## Setup

1. **Get a Gemini API Key** from [Google AI Studio](https://aistudio.google.com/apikey)
2. **Launch RECALL.OS** and paste your API key in Settings
3. **Add documents** via drag-drop or folder sync

## Tech Stack

- **Frontend:** React 19, TanStack Query, Tailwind CSS
- **Backend:** Rust, Tauri v2
- **Database:** SQLite + sqlite-vec (vectors) + FTS5 (full-text)
- **AI:** Google Gemini API (BYOK)

## License

Trial: 25 documents free | Licensed: $29.99 one-time for unlimited

## Contributing

Issues and PRs welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Support

- [GitHub Issues](https://github.com/Project-Intuitus/Recall-OS/issues)
- [Website](https://project-intuitus.github.io/Recall-OS/)

---

<p align="center">
  Built with <a href="https://tauri.app/">Tauri</a> • <a href="https://react.dev/">React</a> • <a href="https://ai.google.dev/">Gemini AI</a>
</p>

<p align="center">
  <em>Last updated: February 2026</em>
</p>
