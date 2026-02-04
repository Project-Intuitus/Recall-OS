<div align="center">

# Support

Need help with RECALL.OS? You're in the right place.

</div>

<br>

## Getting Help

### 1. Documentation

Start with the [README](README.md) for setup instructions and feature overview.

### 2. Common Issues

<details>
<summary><strong>API Key Validation Fails</strong></summary>

**Symptoms:** "Billing not enabled" or "Quota exceeded" error

**Solution:**
1. Visit [Google AI Studio](https://aistudio.google.com/apikey)
2. Ensure billing is enabled on your Google Cloud account
3. Generate a new API key if needed
4. Verify the key works by testing in AI Studio first

</details>

<details>
<summary><strong>Notifications Appear in Wrong Position</strong></summary>

**Symptoms:** Notification windows show up off-screen or in corners

**Solution:**
- Update to version 1.0.1 or later (fixes high-DPI scaling)
- Restart the application after changing display settings

</details>

<details>
<summary><strong>Documents Not Indexing</strong></summary>

**Symptoms:** Files added but not appearing in search

**Solution:**
1. Check the ingestion progress in the sidebar
2. Verify the file type is supported (PDF, TXT, DOCX, images, video, audio)
3. For scanned PDFs, ensure OCR is completing (check logs)
4. Restart if ingestion appears stuck

</details>

<details>
<summary><strong>Search Returns No Results</strong></summary>

**Symptoms:** Queries return empty even with indexed documents

**Solution:**
1. Wait for indexing to complete (check progress indicator)
2. Try simpler, broader search terms
3. Verify documents contain the text you're searching for

</details>

<details>
<summary><strong>Application Won't Start</strong></summary>

**Symptoms:** App crashes on launch or shows blank window

**Solution:**
1. Run as Administrator once to initialize
2. Check Windows Event Viewer for error details
3. Delete `%APPDATA%/com.recallos.app` and restart (resets settings)
4. Reinstall from latest release

</details>

<br>

## Reporting Issues

### Bug Reports

Open a [GitHub Issue](https://github.com/Project-Intuitus/Recall-OS/issues/new) with:

- **RECALL.OS version** (Settings → About)
- **Windows version** (e.g., Windows 11 22H2)
- **Display scaling** (e.g., 150%)
- **Steps to reproduce**
- **Expected vs actual behavior**
- **Screenshots** if applicable

### Feature Requests

We welcome ideas! Open an issue describing:

- The problem you're trying to solve
- Your proposed solution
- Any alternatives you've considered

<br>

## Community

### GitHub Discussions

For questions, ideas, and general conversation:

[**github.com/Project-Intuitus/Recall-OS/discussions**](https://github.com/Project-Intuitus/Recall-OS/discussions)

### Stay Updated

- Star the repo to get release notifications
- Follow [@ProjectIntuitus](https://projectintuitus.com) for announcements

<br>

## Response Times

We're a small team building in public. Please be patient:

| Type | Expected Response |
|------|-------------------|
| Critical bugs | 24-48 hours |
| Regular bugs | 3-5 days |
| Feature requests | 1-2 weeks |
| Questions | 3-5 days |

<br>

## Security Issues

Found a security vulnerability? **Do not open a public issue.**

Email security concerns to the maintainers directly through GitHub's private vulnerability reporting.

<br>

---

<div align="center">

**Still stuck?** [Open an issue](https://github.com/Project-Intuitus/Recall-OS/issues) and we'll help.

*Part of the [Project Intuitus](https://projectintuitus.com) ecosystem*

</div>
