## What's New in v1.0.3

Bug fixes, stability improvements, and updated licensing flow.

### Bug Fixes

**Path Normalization Fix**
- Fixed "UNIQUE constraint failed: documents.file_path" error when re-ingesting documents
- Windows path canonicalization was adding `\\?\` prefix causing path mismatch

**Document Counter Fix**
- Trial document counter (X/25) now updates immediately after uploading or deleting documents
- Previously required a page refresh to see updated count

**Community Links Fix**
- Links in the Help modal Community tab now correctly open in an external browser
- Fixed issue where links silently failed in Tauri's webview

**Embedding Model Migration**
- Automatically migrates from deprecated `text-embedding-004` to `gemini-embedding-001`
- Users no longer need to manually update their settings

### Changes

**Licensing Coming Soon**
- License purchasing is not yet available — Paddle approval pending
- In-app license modal now shows "Coming Soon" with a waitlist signup link
- All trial features remain fully functional (25 documents, all capabilities)
- Join the waitlist at projectintuitus.com to be notified when licensing launches

### Installation

1. Download `RECALL.OS_1.0.3_x64-setup.exe` below
2. Run the installer
3. Get your API key from [Google AI Studio](https://aistudio.google.com/apikey)
4. Paste in Settings → Validate → Start indexing

### Requirements

- Windows 10/11 (x64)
- 4GB RAM minimum
- Gemini API key (free tier available)

---

**Full Changelog**: https://github.com/Project-Intuitus/Recall-OS/compare/v1.0.2...v1.0.3
