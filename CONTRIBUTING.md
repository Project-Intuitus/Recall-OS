<div align="center">

# Contributing to RECALL.OS

Thank you for your interest in contributing to RECALL.OS.

</div>

<br>

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment. We expect all contributors to:

- Be respectful and considerate in all interactions
- Welcome newcomers and help them get started
- Focus on constructive feedback
- Accept responsibility for mistakes and learn from them

<br>

## How to Contribute

### Reporting Bugs

Found a bug? Please open an issue with:

1. **Clear title** — Summarize the issue in one line
2. **Environment** — Windows version, display scaling, RECALL.OS version
3. **Steps to reproduce** — Numbered list of actions to trigger the bug
4. **Expected behavior** — What should happen
5. **Actual behavior** — What actually happens
6. **Screenshots** — If applicable, add visuals

```markdown
**Environment:**
- OS: Windows 11 (22H2)
- Display Scaling: 150%
- RECALL.OS Version: 1.0.1

**Steps to Reproduce:**
1. Open Settings
2. Enter API key
3. Click Validate

**Expected:** Validation succeeds
**Actual:** Error message appears
```

### Suggesting Features

Have an idea? Open an issue with:

1. **Problem statement** — What problem does this solve?
2. **Proposed solution** — How should it work?
3. **Alternatives considered** — Other approaches you thought about
4. **Additional context** — Mockups, examples, references

### Pull Requests

Ready to contribute code?

1. **Fork** the repository
2. **Create a branch** from `main`
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. **Make your changes** following our code style
4. **Test thoroughly** on Windows 10 and 11
5. **Commit** with clear messages
   ```bash
   git commit -m "Add feature: description of what it does"
   ```
6. **Push** to your fork
7. **Open a PR** with a clear description

<br>

## Development Setup

### Prerequisites

| Requirement | Version |
|-------------|---------|
| Node.js | 18+ |
| Rust | 1.84+ |
| Windows | 10/11 |

### Quick Start

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/Recall-OS.git
cd Recall-OS

# Install dependencies
npm install

# Run in development mode
npm run tauri:dev
```

### Required Resources

Download and place in `src-tauri/resources/`:

| File | Source |
|------|--------|
| `vec0.dll` | [sqlite-vec releases](https://github.com/asg017/sqlite-vec/releases) |
| `ffmpeg.exe` | [gyan.dev builds](https://www.gyan.dev/ffmpeg/builds/) |

### Project Structure

```
Recall-OS/
├── src/                    # React frontend
│   ├── components/         # UI components
│   ├── hooks/              # React hooks
│   └── types/              # TypeScript types
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── commands/       # Tauri commands (IPC)
│   │   ├── database/       # SQLite operations
│   │   ├── ingestion/      # Document processing
│   │   ├── llm/            # Gemini API client
│   │   ├── rag/            # RAG pipeline
│   │   └── notifications/  # Custom notifications
│   └── resources/          # Runtime dependencies
└── docs/                   # Documentation
```

<br>

## Code Style

### TypeScript/React

- Use functional components with hooks
- Prefer `const` over `let`
- Use TypeScript strict mode
- Format with Prettier defaults

### Rust

- Follow Rust conventions (`snake_case` for functions, `CamelCase` for types)
- Use `clippy` for linting
- Handle errors with `Result<T, RecallError>`
- Document public functions

### Commits

- Use present tense ("Add feature" not "Added feature")
- Keep first line under 72 characters
- Reference issues when applicable (`Fixes #123`)

<br>

## Testing

Before submitting a PR:

1. **Build successfully**
   ```bash
   npm run tauri:build
   ```

2. **Test on both Windows 10 and 11** if possible

3. **Test with different display scaling** (100%, 125%, 150%)

4. **Verify no regressions** in existing functionality

<br>

## Questions?

- Open a [GitHub Discussion](https://github.com/Project-Intuitus/Recall-OS/discussions)
- Check existing [Issues](https://github.com/Project-Intuitus/Recall-OS/issues)

<br>

---

<div align="center">

**Thank you for helping make RECALL.OS better.**

*Part of the [Project Intuitus](https://projectintuitus.com) ecosystem*

</div>
