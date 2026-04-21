# Meeting Assistant Design

## Goal

Build a Windows-first desktop meeting assistant on top of the existing Tauri v2 + React project. The app should record meetings, capture both microphone and system audio, stream a mixed audio track to the backend in real time, display incremental STT results, continuously refresh structured meeting notes, and preserve enough local state to recover from crashes or connection loss.

## Project Context

The current desktop app already uses:

- Tauri v2 for the desktop shell
- React 19 + Vite for the frontend
- React Router 7 for navigation
- Zustand for lightweight UI state
- Tailwind/shadcn for the UI layer

The current Rust side is still close to the Tauri starter template, so the runtime architecture should be introduced without replacing the existing frontend stack.

The backend `meeting-server` is currently an initialized Go skeleton and can evolve together with the desktop client. The overall architecture should borrow the separation-of-concerns ideas from the XiaoZhi ESP32 projects without copying embedded implementation details directly.

## Confirmed Product Constraints

- Rust is the runtime authority; React only renders UI and sends user intent.
- First release targets Windows only.
- macOS must remain a future extension point in the architecture.
- Audio must use MQTT for control and UDP for streaming.
- The app must capture both microphone input and system loopback audio.
- The app must preserve original local audio files.
- Real-time uplink uses a single mixed stream.
- Recovery should support re-uploading the not-yet-uploaded mixed audio interval after a crash or disconnect.

## Architecture Overview

The system is split into six layers:

1. App Shell
2. Transport
3. Audio Pipeline
4. Realtime AI Pipeline
5. Domain
6. Persistence and Recovery

React never connects to MQTT, UDP, or SQLite directly. Rust owns the live session runtime, emits strongly typed events to the frontend, and persists all durable meeting state.

### App Shell

- Tauri v2 hosts the desktop application.
- Rust owns:
  - application state
  - session runtime lifecycle
  - transport setup and reconnection
  - audio coordination
  - database access
  - recovery checkpoints
  - background flush on shutdown
- React owns:
  - meeting list page
  - live meeting workspace
  - meeting detail page
  - view-only UI state such as scroll position, active tabs, and filters

### Transport

Transport is explicitly split into:

- Control transport: MQTT
- Audio transport: UDP

The desktop code will expose a common transport-oriented interface so the rest of the runtime depends on contracts instead of concrete clients.

Required capabilities:

- `connect`
- `disconnect`
- `open_session`
- `close_session`
- `send_control_message`
- `send_audio_chunk`
- `on_message`
- `on_error`

The concrete implementation is still two-channel:

- `MqttControlTransport`
- `UdpAudioTransport`

This preserves the XiaoZhi-style separation between control flow and high-frequency media flow while adapting it to the desktop meeting use case.

### Audio Pipeline

The desktop app manages three audio products at once:

1. raw microphone recording
2. raw system loopback recording
3. mixed uplink stream used for real-time STT and summary generation

Pipeline stages:

1. Capture `mic_input` via Windows WASAPI capture.
2. Capture `system_loopback` via Windows WASAPI loopback.
3. Normalize both streams into a shared internal timeline.
4. Write raw WAV assets locally.
5. Mix both streams into a mono uplink track.
6. Chunk the mixed track.
7. Send mixed chunks over UDP asynchronously.
8. Track upload progress for replay and recovery.

First-version defaults:

- capture format: platform-native PCM, converted internally as needed
- uplink format: PCM16, 16kHz, mono
- local preserved assets: WAV
- internal capture frame: 20ms
- uplink chunk: 200ms

The audio path must never block the UI thread. The runtime needs:

- bounded buffers
- overflow protection
- flush-on-stop behavior
- checkpointed upload progress
- graceful close on session stop

### Realtime AI Pipeline

The backend emits structured MQTT messages instead of ad hoc strings. Rust owns the message dispatcher and transforms incoming events into domain updates plus frontend events.

Supported event types:

- `session/hello`
- `recording_started`
- `recording_stopped`
- `stt_delta`
- `stt_final`
- `summary_delta`
- `summary_final`
- `action_item_delta`
- `action_item_final`
- `error`
- `heartbeat`

Frontend rendering rules:

- show incremental and final transcript states
- refresh summary blocks in place
- merge meeting-time summary deltas with final post-stop summary output
- avoid whole-page rerenders for live content

### Domain Model

`MeetingSession` is the aggregate root.

Required fields:

- `id`
- `title`
- `started_at`
- `ended_at`
- `status`
- `duration_ms`
- `client_id`
- `transport_strategy`
- `transcript_segments`
- `summary_snapshots`
- `action_items`
- `participants`
- `audio_assets`
- `capture_devices`
- `checkpoint`

`TranscriptSegment` fields:

- `segment_id`
- `meeting_id`
- `start_ms`
- `end_ms`
- `text`
- `is_final`
- `speaker_id`
- `revision`

`SummarySnapshot` fields:

- `meeting_id`
- `version`
- `updated_at`
- `abstract`
- `key_points`
- `decisions`
- `risks`
- `action_items`
- `is_final`

`AudioAssets` fields:

- `mic_original_path`
- `system_original_path`
- `mixed_uplink_path`
- `storage_status`

`SessionCheckpoint` fields:

- `meeting_id`
- `last_control_seq`
- `last_udp_seq_sent`
- `last_uploaded_mixed_ms`
- `last_transcript_segment_revision`
- `last_summary_version`
- `last_action_item_version`
- `local_recording_state`
- `recovery_token`
- `updated_at`

### Persistence and Recovery

Rust exclusively owns SQLite access.

Required durable data:

- meeting metadata
- transcript segments
- summary snapshots
- final summary result
- action items
- audio asset paths
- session checkpoints

Required tables:

- `meetings`
- `transcript_segments`
- `summary_snapshots`
- `action_items`
- `audio_assets`
- `session_checkpoints`

Crash recovery strategy:

1. Detect incomplete meetings on startup.
2. Load the last checkpoint and audio asset metadata.
3. Offer the user a recovery prompt.
4. Reconnect MQTT control.
5. Re-establish the UDP audio session.
6. Replay the mixed audio interval that was recorded locally but not acknowledged as uploaded.
7. Resume live capture if the user chooses to continue.

This design avoids depending on MQTT retain for recovery and instead uses local durability plus explicit `resume` semantics.

## State Machine Design

The runtime uses one centralized Rust state machine. React does not construct hidden business rules of its own.

### Primary Session State

- `idle`
- `connecting`
- `ready`
- `recording`
- `paused`
- `stopping`
- `completed`
- `error`

### Parallel Activity Flags

- `is_transcribing`
- `is_summarizing`
- `is_reconnecting`
- `is_flushing`
- `has_unpersisted_results`

This allows the UI to display rich status labels such as:

- connecting
- recording
- transcribing
- summarizing
- paused
- stopping
- completed
- error

without collapsing multiple concurrent activities into one flat enum.

### State Transitions

- `idle -> connecting`
  - when a meeting is created and start is requested
- `connecting -> ready`
  - after MQTT connect, hello success, and UDP session setup
- `ready -> recording`
  - after audio capture and local file writers start successfully
- `recording -> paused`
  - on user pause
- `paused -> recording`
  - on user resume
- `recording -> stopping`
  - on stop request
- `paused -> stopping`
  - on stop request while paused
- `stopping -> completed`
  - after final AI results arrive and all pending writes flush
- `connecting|ready|recording|paused|stopping -> error`
  - on fatal transport, device, protocol, or persistence failures
- `error -> connecting`
  - on user retry

### Required Failure Handling

- MQTT disconnect:
  - remain in the current meeting session
  - set reconnect activity flag
  - continue local recording and file writes
  - buffer mixed audio for replay
- backend unavailable during recording:
  - do not drop local capture
  - surface degraded status in the UI
- stop while pending final results exist:
  - keep state as `stopping`
  - flush final transcript and summary writes before `completed`
- app close:
  - trigger stop and flush
  - persist checkpoint before exit

## MQTT Message Model

All MQTT messages should use a shared envelope in both Rust and TypeScript.

Example conceptual envelope:

```ts
type Envelope<TType extends string, TPayload> = {
  version: "v1";
  messageId: string;
  correlationId?: string;
  clientId: string;
  sessionId: string;
  seq: number;
  sentAt: string;
  type: TType;
  payload: TPayload;
};
```

Message groups:

- control
- transcript
- summary
- action items
- generic events

Control message types:

- `session/hello`
- `session/resume`
- `session/close`
- `recording/start`
- `recording/pause`
- `recording/resume`
- `recording/stop`
- `session/flush`
- `heartbeat`
- `ack`
- `error`

Server event types:

- `recording_started`
- `recording_paused`
- `recording_resumed`
- `recording_stopped`
- `stt_delta`
- `stt_final`
- `summary_delta`
- `summary_final`
- `action_item_delta`
- `action_item_final`
- `heartbeat`
- `error`

### MQTT Topics

- `meetings/{client_id}/session/{session_id}/control`
- `meetings/{client_id}/session/{session_id}/control/reply`
- `meetings/{client_id}/session/{session_id}/events`
- `meetings/{client_id}/session/{session_id}/stt`
- `meetings/{client_id}/session/{session_id}/summary`
- `meetings/{client_id}/session/{session_id}/action-items`
- `meetings/{client_id}/presence`

### QoS Strategy

Use `QoS 1` for:

- session open/close and resume
- recording start/stop
- control replies
- final transcript events
- final summary events
- final action item events
- errors

Use `QoS 0` for:

- heartbeat
- `stt_delta`
- `summary_delta`
- `action_item_delta`

### Retain Strategy

- session topics: `retain = false`
- live result topics: `retain = false`
- presence topic may use retain if needed

Recovery uses checkpoints plus resume semantics, not retained live data.

## UDP Audio Model

UDP is used for the high-frequency mixed audio uplink.

`AudioChunk` fields:

- `session_id`
- `chunk_id`
- `seq`
- `capture_started_at_ms`
- `duration_ms`
- `source_type`
- `sample_rate`
- `channels`
- `format`
- `payload`

`source_type` supports:

- `mixed`
- `mic`
- `system`

First-version realtime upload only sends `mixed`, but the protocol leaves room for future dual-track uplink.

Recommended packet cadence:

- internal frame: 20ms
- persisted write batch: 200ms
- UDP uplink chunk: 200ms

## Frontend Information Architecture

### Home Page

- create meeting
- list meeting history
- search by title or content hints
- sort by recency
- recover unfinished meeting prompt

### Live Meeting Workspace

- header with title, start time, timer, connection state
- left transcript stream
- right live summary panel
- bottom controls for start, pause, resume, stop
- visible runtime status labels
- smooth local updates instead of full rerenders

### Meeting Detail Page

- full transcript by segment
- final summary
- action items
- audio asset metadata
- export Markdown
- copy summary

## Repository Layout

### Desktop Client

- `src/routes/`
- `src/features/meetings/`
- `src/features/transcript/`
- `src/features/summary/`
- `src/features/session/`
- `src/lib/api/`
- `src/lib/events/`
- `src/lib/state/`
- `src/components/`
- `src-tauri/src/app_state.rs`
- `src-tauri/src/commands/`
- `src-tauri/src/events/`
- `src-tauri/src/protocol/`
- `src-tauri/src/session/`
- `src-tauri/src/transport/`
- `src-tauri/src/audio/`
- `src-tauri/src/audio/platform/windows/`
- `src-tauri/src/audio/platform/macos/`
- `src-tauri/src/storage/`
- `src-tauri/src/export/`

### Backend Server

- `meeting-server/cmd/server/`
- `meeting-server/internal/app/`
- `meeting-server/internal/protocol/`
- `meeting-server/internal/transport/mqtt/`
- `meeting-server/internal/transport/udp/`
- `meeting-server/internal/session/`
- `meeting-server/internal/pipeline/stt/`
- `meeting-server/internal/pipeline/summary/`
- `meeting-server/internal/pipeline/action_items/`
- `meeting-server/internal/storage/`
- `meeting-server/internal/recovery/`

## MVP Scope

Included in MVP:

- Windows-only desktop runtime
- Rust-controlled meeting lifecycle
- MQTT control channel
- UDP mixed audio uplink
- microphone + system audio capture
- WAV preservation for mic, system, and mixed tracks
- live transcript rendering
- live structured summary rendering
- action item extraction
- SQLite-backed meeting history
- crash recovery using local replay of the mixed uplink interval
- Markdown export

Explicitly out of scope for MVP:

- macOS implementation
- device switching UI
- dual-track realtime server processing
- advanced AEC
- speaker diarization
- Opus encoding
- PDF or DOCX export
- cloud sync

## Implementation Priorities

Phase order:

1. project structure, types, schema, state machine skeleton
2. meeting lifecycle and persistence basics
3. MQTT control transport
4. Windows audio capture and chunk pipeline
5. realtime transcript integration
6. realtime summary integration
7. history, recovery, and Markdown export

The key quality target is sustainable development rather than a demo: clear module boundaries, recoverable state, typed protocols, and a desktop runtime that can run long meetings reliably.
