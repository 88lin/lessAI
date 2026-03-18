# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

LessAI is a desktop application for AI-assisted Chinese text rewriting. Users import articles, the app chunks the text into segments, calls an OpenAI-compatible LLM API to rewrite each chunk to sound more naturally human-written, and presents inline diffs for approval/rejection. The goal is to reduce AI-detection scores on existing text.

Built with **Tauri 2** (Rust backend + React/TypeScript frontend).

## Build and Development Commands

```bash
# Install frontend dependencies (pnpm is the package manager)
pnpm install

# Run in development mode (starts both Vite dev server and Tauri window)
pnpm run tauri:dev

# Build production binary
pnpm run tauri:build

# Frontend-only dev server (no Tauri shell, useful for UI iteration)
pnpm dev

# TypeScript type-check
pnpm run typecheck

# Run Rust tests (backend unit tests in src-tauri/src/rewrite.rs)
cd src-tauri && cargo test

# Run a single Rust test
cd src-tauri && cargo test <test_name>
# e.g.: cargo test normalizes_line_endings_and_blank_lines
```

There is no frontend test framework configured. There are no lint or format scripts in package.json. The Rust backend has a small test suite in `src-tauri/src/rewrite.rs` covering text normalization, segmentation, and diff generation.

## Architecture

### Two-Process Model

The app follows the standard Tauri 2 architecture: a Rust process hosts the native window and business logic; a webview renders the React UI. All communication flows through Tauri's IPC command system.

```
Frontend (React/TS)                    Backend (Rust)
  src/App.tsx          --invoke-->       src-tauri/src/main.rs (commands)
  src/lib/api.ts       <--events--       src-tauri/src/rewrite.rs (LLM + diff)
  src/lib/types.ts                       src-tauri/src/models.rs (shared types)
                                         src-tauri/src/storage.rs (JSON file I/O)
```

### Backend Modules (src-tauri/src/)

- **main.rs** -- Tauri command handlers and app state. Manages `AppState` which tracks running rewrite jobs via `HashMap<String, Arc<JobControl>>`. Contains the core workflow orchestration: `process_chunk`, `run_manual_rewrite`, `run_auto_loop`. All Tauri commands are registered here.
- **models.rs** -- All shared data types (`AppSettings`, `DocumentSession`, `ChunkTask`, enums). All structs use `#[serde(rename_all = "camelCase")]` so field names match between Rust and TypeScript automatically.
- **rewrite.rs** -- LLM integration and text processing. `rewrite_chunk()` calls an OpenAI-compatible `/chat/completions` endpoint. `normalize_text()` cleans line endings. `segment_text()` splits text into chunks with a configurable preset (`clause` / `sentence` / `paragraph`). `build_diff()` produces character-level inline diffs using an LCS-based algorithm.
- **storage.rs** -- Persistence layer. Settings and sessions are stored as JSON files under Tauri's `app_data_dir()`. Sessions live in a `sessions/` subdirectory, one JSON file per session.

### Frontend Structure (src/)

The frontend is a minimal editor-like layout (single workbench). Key files:

- **App shell** (`src/App.tsx`) -- Top bar + frameless window controls + settings modal mount + global state.
- **Workbench** (`src/stages/WorkbenchStage.tsx`) -- Document panel (full-text views + primary actions) + Review timeline (ordered suggestions, apply/dismiss/delete).
- **IPC layer** (`src/lib/api.ts`) -- Thin wrapper around `@tauri-apps/api/core invoke()`. Each function maps 1:1 to a Tauri command.
- **Types** (`src/lib/types.ts`) -- TypeScript interfaces mirroring the Rust models.
- **Event listeners** (`src/hooks/useTauriEvents.ts`) -- Subscribes to Tauri events: `rewrite_progress`, `chunk_completed`, `rewrite_finished`, `rewrite_failed`.
- **Styling** (`src/styles.css`) -- Single CSS file, no framework. Uses CSS custom properties for theming.

### Data Flow for Rewriting

1. Open a file: Frontend calls `open_document(path)` -> backend normalizes text, segments into chunks, and creates/loads a session (JSON persisted).
2. Start rewrite: `start_rewrite(session_id, mode)` -> backend rewrites next chunk via LLM API, and creates an `EditSuggestion` entry.
3. Review timeline: suggestions stay visible in order. User can:
   - `apply_suggestion` (accept)
   - `dismiss_suggestion` (ignore)
   - `delete_suggestion` (remove from timeline)
   - `retry_chunk` (re-run LLM for a specific chunk)
4. Export: `export_document` merges applied suggestions into a final text file.
5. Finalize: `finalize_document` writes the merged result back to the original file and deletes the session JSON (so the file looks “never edited” on next open).

### IPC Commands (Tauri Commands)

Settings: `load_settings`, `save_settings`, `test_provider`
Sessions: `open_document`, `load_session`
Rewrite: `start_rewrite`, `pause_rewrite`, `resume_rewrite`, `cancel_rewrite`, `retry_chunk`
Suggestions: `apply_suggestion`, `dismiss_suggestion`, `delete_suggestion`
Export: `export_document`, `finalize_document`

## Design System

The UI follows a **Bauhaus-inspired** design system (see `src/styles.css` for the source of truth):

- Color palette: paper backgrounds (#f5efe4), black ink (#141414), red (#d62d20), blue (#1744cf), yellow (#efc122)
- Hard offset box shadows (never blurred), thick 3px black borders, large border radii
- Typography: Newsreader (serif) for headings, Roboto (sans-serif) for body
- Icons: lucide-react
- No CSS framework -- all custom CSS in `src/styles.css`

## Key Conventions

- All user-facing strings are in **Simplified Chinese**
- Rust structs use `snake_case` with `#[serde(rename_all = "camelCase")]` for automatic JS/TS interop
- Prompt presets live under `prompt/` and are selectable in the Settings modal
- The app connects to any OpenAI-compatible API (configurable base URL, API key, model name)
- Session persistence is file-based JSON (no database)
 
