# Backend-Only Meeting Persistence Design

## Goal

Remove desktop-side SQLite business persistence and make the backend database the only durable source of meeting state, while keeping local audio files on the desktop for replay, traceability, and later file management.

## Existing State

The desktop client currently persists these categories into local SQLite:

- meetings
- transcript segments
- summary snapshots and action items
- session checkpoints
- audio asset paths

The backend already persists:

- `meetings`
- `meeting_transcripts` (single latest transcript snapshot used by server-side runtime)

The backend does not yet persist the desktop detail model:

- full transcript segment timeline
- latest summary snapshot for detail pages
- desktop recovery checkpoint state
- local audio asset metadata

## Decision

Use the backend database as the only durable source for meeting business state.

Keep local desktop files for:

- `mic-original.wav`
- `system-original.wav`
- `mixed-uplink.wav`

Do not move audio file contents into the backend. Persist only their metadata and recovery offsets.

## Architecture

### Desktop

The desktop app will no longer open or write SQLite.

Instead it will:

- keep active meeting state in memory
- keep local audio files on disk
- sync meeting lifecycle, transcript deltas, summary deltas, action-item deltas, checkpoint updates, and audio asset metadata to backend HTTP APIs
- fetch meeting history and detail directly from backend APIs

### Backend

The backend remains the single durable store for:

- meeting rows in existing `meetings`
- transcript segment rows in a new desktop-focused table
- latest summary snapshot in a new table
- latest checkpoint in a new table
- local audio asset metadata in a new table

The existing `meeting_transcripts` table remains in place because it already serves the server runtime and is not a replacement for full desktop segment history.

## Data Model

### Reuse Existing

- `meetings`
- `meeting_transcripts`

### Add New

- `meeting_transcript_segments`
- `meeting_summary_snapshots`
- `meeting_session_checkpoints`
- `meeting_audio_assets`

These new tables mirror the business state that used to live in desktop SQLite.

## API Shape

Authenticated desktop app routes will expose:

- `GET /api/app/meetings`
- `GET /api/app/meetings/recoverable`
- `GET /api/app/meetings/:meetingID`
- `PUT /api/app/meetings/:meetingID`
- `PUT /api/app/meetings/:meetingID/transcript-segments/:segmentID`
- `PUT /api/app/meetings/:meetingID/summary`
- `PUT /api/app/meetings/:meetingID/action-items`
- `GET /api/app/meetings/:meetingID/checkpoint`
- `PUT /api/app/meetings/:meetingID/checkpoint`
- `PUT /api/app/meetings/:meetingID/audio-assets`

## Desktop Sync Strategy

Meeting lifecycle and audio-runtime persistence should not depend on the React route being mounted.

So the durable sync path for:

- transcript deltas
- summary deltas
- action-item deltas
- checkpoint updates
- audio asset metadata

will live in Rust.

The frontend will still use authenticated fetch calls for:

- meeting history
- recoverable meetings
- meeting detail
- markdown export data loading

The frontend login flow will push the backend auth token into Rust memory so Rust-side sync can call authenticated backend APIs.

## Recovery

Recovery stays split across two truth sources by design:

- backend checkpoint decides replay offsets and upload progress
- local mixed audio file provides the replayable samples

That preserves local traceability without keeping any SQLite state.

## Migration Scope

Remove these desktop concerns from compiled code:

- `rusqlite`
- `storage` module usage
- app-state SQLite initialization
- SQLite-backed history/detail/export commands
- SQLite-backed event persistence

Keep:

- local audio directory creation
- local wave writing
- session manager and runtime diagnostics

## Risks

### Checkpoint Write Frequency

SQLite writes were local and cheap. Backend checkpoint writes will now be HTTP requests. The first version should keep behavior correct before optimizing write frequency.

### Authentication Availability

Rust-side sync now depends on the desktop auth token being available in memory after login. The login and logout flow must update Rust state reliably.

### Existing Tests

Many desktop Rust tests currently assume `Database::open_in_memory()`. Those tests need to be rewritten around an in-memory persistence implementation instead of SQLite.
