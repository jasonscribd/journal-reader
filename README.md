# Journal Reader

Local-first desktop app (Tauri + React) to import, store, and browse personal journal entries in SQLite, with fast full-text search and optional local AI.

## Features

- Import .txt and .doc/.docx files with user-entered dates (MM-YYYY)
- Browse by year → month; click a month to see entries
- Two-color month grid: filled vs empty
- Full-text search (SQLite FTS5) across saved entries
- Entry viewer modal (click an entry)
- Settings persisted locally (SQLite)
- Optional AI (local Ollama) for tagging/semantic search/chat (wiring in progress)

## Project Structure

- `src/` – React UI (Vite, Tailwind)
- `src-tauri/` – Tauri (Rust) backend
- `prisma/` – Legacy schema (not used in current backend)

## Requirements

- Node.js 18+ and npm (or pnpm/yarn)
- Rust (stable) and Tauri prerequisites
  - macOS: Xcode Command Line Tools (`xcode-select --install`)
  - Windows: Visual Studio Build Tools (C++), WebView2 runtime
  - Linux: GTK/WebKit packages per Tauri docs

## Quick Start (Dev)

```bash
# clone
git clone <your-repo-url>
cd "Journal Reader"

# install deps
npm install

# run app (Vite + Tauri)
npm run tauri dev
```

Dev server runs on `http://localhost:1421` and launches the Tauri window.

## Build a Local App Bundle

```bash
npm run tauri build
```

Artifacts (by OS):
- macOS: `src-tauri/target/release/bundle/macos/Journal Reader.app`
- Windows: `src-tauri/target/release/bundle/msi/*.msi` (or `.exe`)
- Linux: `src-tauri/target/release/bundle/<format>/`

## Using the App

1) Import
- Go to Import → select files or a folder
- Assign a month/year (bulk or per file)
- Start import

2) Browse
- Timeline → select a year
- Click a month to see entries
- Click an entry to open the modal (full text)

3) Search
- Go to Search, enter terms, press Enter or click Search
- Results are powered by SQLite FTS5

## Database & Storage

- SQLite DB location (macOS):
  `~/Library/Application Support/com.jasonb.journal-reader/journal-reader/journal.db`
- Similar app-data paths for Windows/Linux via Tauri
- To reset: close app and delete `journal.db`

## Optional: Local AI with Ollama

1) Install & start Ollama
```bash
brew install ollama        # macOS (or see ollama.com for other OS)
ollama serve
```

2) Pull models
```bash
ollama pull llama3.1:8b
ollama pull nomic-embed-text
```

3) Settings in the app
- AI Provider: Ollama (Local)
- Ollama URL: `http://localhost:11434`
- Default Model: `llama3.1:8b`
- Embedding Model: `nomic-embed-text`
- Test Connection (should report reachable)

Note: Tagging/semantic search/chat endpoints are being wired up; once enabled, embeddings will be generated on import and semantic search will be available.

## Troubleshooting

- Vite Port in use (1421)
  - Kill the process using the port or change the dev port in `package.json` and `src-tauri/tauri.conf.json`.
- Search returns nothing
  - Ensure entries were imported (Timeline shows total count)
  - FTS backfill runs on startup; try restarting the app after import
- Ollama unreachable
  - Confirm `ollama serve` is running and URL is correct (`http://localhost:11434`)

## Contributing / License

- Open issues and PRs welcome
- License: MIT (choose or update as you prefer)
