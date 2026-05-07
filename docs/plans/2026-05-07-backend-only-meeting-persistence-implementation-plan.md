# Backend-Only Meeting Persistence Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remove desktop SQLite business persistence and move the persisted meeting model into backend database tables and APIs, while keeping local audio files on disk.

**Architecture:** Reuse backend `meetings` and `meeting_transcripts`, add backend stores for transcript segments, summary snapshot, checkpoint, and audio asset metadata, then replace desktop SQLite access with authenticated backend sync plus in-memory runtime state.

**Tech Stack:** Go, Gin, PostgreSQL, Rust, Tauri, reqwest, TypeScript, React, Vitest, cargo test, go test

---

### Task 1: Add backend meeting detail data model

**Files:**
- Create: `meeting-server/internal/admin/meeting_detail_service.go`
- Create: `meeting-server/internal/admin/meeting_detail_memory_store.go`
- Create: `meeting-server/internal/admin/meeting_detail_postgres_store.go`
- Test: `meeting-server/internal/admin/service_test.go`

**Step 1: Write the failing tests**

Add tests that expect:

- transcript segment upsert and ordered listing
- summary snapshot upsert and action-item merge
- checkpoint upsert and reload
- audio asset upsert and reload

**Step 2: Run test to verify it fails**

Run: `go test ./internal/admin -run MeetingDetail -v`

**Step 3: Write minimal implementation**

Implement:

- detail record structs
- memory store
- postgres store with schema creation only for new tables

**Step 4: Run test to verify it passes**

Run: `go test ./internal/admin -run MeetingDetail -v`

**Step 5: Commit**

```bash
git add meeting-server/internal/admin
git commit -m "feat: add backend meeting detail persistence"
```

### Task 2: Expose authenticated app APIs for desktop detail sync

**Files:**
- Modify: `meeting-server/internal/admin/http.go`
- Modify: `meeting-server/internal/admin/http_auth_test.go`
- Modify: `meeting-server/internal/app/app.go`

**Step 1: Write the failing tests**

Add authenticated app-route tests that expect:

- list current user's meetings
- list current user's recoverable meetings
- fetch meeting detail
- upsert transcript segment
- upsert summary snapshot
- merge action items
- upsert and fetch checkpoint
- upsert audio assets

**Step 2: Run test to verify it fails**

Run: `go test ./internal/admin -run AppMeeting -v`

**Step 3: Write minimal implementation**

Wire the new detail service into app startup and add the app routes.

**Step 4: Run test to verify it passes**

Run: `go test ./internal/admin -run AppMeeting -v`

**Step 5: Commit**

```bash
git add meeting-server/internal/admin meeting-server/internal/app/app.go
git commit -m "feat: add desktop meeting detail app APIs"
```

### Task 3: Replace desktop SQLite runtime persistence with backend sync

**Files:**
- Create: `meeting-desktop/src-tauri/src/backend_sync/mod.rs`
- Modify: `meeting-desktop/src-tauri/src/app_state.rs`
- Modify: `meeting-desktop/src-tauri/src/audio/runtime.rs`
- Modify: `meeting-desktop/src-tauri/src/commands/meeting_commands.rs`
- Modify: `meeting-desktop/src-tauri/src/events/processor.rs`
- Modify: `meeting-desktop/src-tauri/src/lib.rs`
- Modify: `meeting-desktop/src-tauri/Cargo.toml`
- Test: `meeting-desktop/src-tauri/src/audio/runtime.rs`
- Test: `meeting-desktop/src-tauri/src/events/processor.rs`

**Step 1: Write the failing tests**

Add or rewrite tests around an in-memory backend-sync implementation that expect:

- prepare persists audio assets and initial checkpoint
- chunk upload updates checkpoint sequence and last uploaded offset
- runtime event processing persists transcript and summary data without SQLite

**Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path meeting-desktop/src-tauri/Cargo.toml audio::runtime events::processor`

**Step 3: Write minimal implementation**

Implement:

- Rust backend sync trait plus HTTP client and in-memory test implementation
- auth token storage in app state
- meeting audio runtime persistence through the new sync trait
- runtime event persistence through the new sync trait
- Tauri commands to set and clear backend auth token

**Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path meeting-desktop/src-tauri/Cargo.toml audio::runtime events::processor`

**Step 5: Commit**

```bash
git add meeting-desktop/src-tauri
git commit -m "refactor: replace desktop sqlite runtime persistence"
```

### Task 4: Move desktop history, detail, recoverable list, and export to backend APIs

**Files:**
- Modify: `meeting-desktop/src/lib/auth.ts`
- Modify: `meeting-desktop/src/lib/api/commands.ts`
- Modify: `meeting-desktop/src/features/session/hooks/use-live-session.ts`
- Modify: `meeting-desktop/src/routes/meeting-detail-page.tsx`
- Modify: `meeting-desktop/src/lib/api/commands.test.ts`
- Modify: `meeting-desktop/src/routes/home-page.test.tsx`
- Modify: `meeting-desktop/src/routes/meeting-detail-page.test.tsx`

**Step 1: Write the failing tests**

Add or update tests that expect:

- login stores auth token for both JS and Rust
- history and detail fetch from backend app APIs
- export markdown builds from backend detail payload

**Step 2: Run test to verify it fails**

Run: `npm test -- commands routes`

**Step 3: Write minimal implementation**

Update frontend commands to fetch from backend and export markdown in TypeScript.

**Step 4: Run test to verify it passes**

Run: `npm test -- commands routes`

**Step 5: Commit**

```bash
git add meeting-desktop/src
git commit -m "refactor: load desktop meeting history from backend"
```

### Task 5: Remove compiled SQLite path and dead desktop storage layer

**Files:**
- Modify: `meeting-desktop/src-tauri/src/commands/mod.rs`
- Delete: `meeting-desktop/src-tauri/src/commands/history_commands.rs`
- Delete: `meeting-desktop/src-tauri/src/commands/export_commands.rs`
- Delete: `meeting-desktop/src-tauri/src/storage/mod.rs`
- Delete: `meeting-desktop/src-tauri/src/storage/db.rs`
- Delete: `meeting-desktop/src-tauri/src/storage/migrations.rs`
- Delete: `meeting-desktop/src-tauri/src/storage/meetings_repo.rs`
- Delete: `meeting-desktop/src-tauri/src/storage/transcript_repo.rs`
- Delete: `meeting-desktop/src-tauri/src/storage/summary_repo.rs`
- Delete: `meeting-desktop/src-tauri/src/storage/checkpoint_repo.rs`
- Delete: `meeting-desktop/src-tauri/src/storage/audio_repo.rs`
- Modify: `meeting-desktop/src-tauri/src/main.rs`
- Modify: `meeting-desktop/README.md`

**Step 1: Write the failing test**

Run the desktop Rust test suite and confirm leftover SQLite imports or command registrations fail to compile.

**Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path meeting-desktop/src-tauri/Cargo.toml`

**Step 3: Write minimal implementation**

Remove unused modules, command registrations, and the `rusqlite` dependency.

**Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path meeting-desktop/src-tauri/Cargo.toml`

**Step 5: Commit**

```bash
git add meeting-desktop/src-tauri meeting-desktop/README.md
git commit -m "refactor: remove desktop sqlite persistence"
```

### Task 6: Verify end-to-end behavior

**Files:**
- Modify: `meeting-desktop/docs/plans/verification-notes.md`

**Step 1: Run verification**

Run:

- `go test ./...`
- `cargo test --manifest-path meeting-desktop/src-tauri/Cargo.toml`
- `npm test`

Then manually verify:

- desktop login succeeds
- start meeting creates backend meeting row
- transcript and summary updates show in backend-backed detail page
- stop and reopen app still recovers from backend checkpoint plus local mixed wav

**Step 2: Record results**

Update verification notes with what passed, what was not run, and any residual risk.

**Step 3: Commit**

```bash
git add meeting-desktop/docs/plans/verification-notes.md
git commit -m "docs: record backend-only persistence verification"
```
