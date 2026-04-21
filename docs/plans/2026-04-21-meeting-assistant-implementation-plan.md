# Meeting Assistant Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a Windows-first Tauri desktop meeting assistant with Rust-owned session runtime, MQTT control, UDP mixed-audio uplink, WAV preservation, realtime transcript and summary rendering, and crash recovery.

**Architecture:** The desktop client keeps React as a pure UI layer while Rust owns the session state machine, transport, audio capture, local persistence, and recovery. Audio capture records microphone and system loopback separately, writes WAV assets locally, mixes them into a single realtime uplink stream, and replays unsent mixed audio during recovery. The backend evolves in parallel to expose typed MQTT control messages and a UDP audio ingest path inspired by the XiaoZhi separation of control and media channels.

**Tech Stack:** Tauri v2, Rust, React 19, React Router 7, Zustand, Tailwind/shadcn, SQLite, Windows WASAPI, MQTT, UDP, Go

---

### Task 1: Replace desktop starter routes with meeting-oriented route skeletons

**Files:**
- Modify: `meeting-desktop/src/App.tsx`
- Modify: `meeting-desktop/src/routes/home-page.tsx`
- Create: `meeting-desktop/src/routes/live-meeting-page.tsx`
- Create: `meeting-desktop/src/routes/meeting-detail-page.tsx`
- Modify: `meeting-desktop/src/routes/not-found-page.tsx`
- Test: `meeting-desktop/src/App.tsx`

**Step 1: Write the failing test**

Document the intended route map in a small route smoke test after a test runner is introduced:

```tsx
render(<App />);
expect(screen.getByRole("link", { name: /会议/i })).toBeInTheDocument();
```

**Step 2: Run test to verify it fails**

Run: `npm test -- App`
Expected: FAIL because no frontend test runner is configured yet.

**Step 3: Write minimal implementation**

- Replace the demo navigation with meeting navigation.
- Keep the existing React stack.
- Add route shells for:
  - home
  - live meeting
  - meeting detail

**Step 4: Run test to verify it passes**

Run: `npm test -- App`
Expected: PASS after the frontend test runner is added in later tasks.

**Step 5: Commit**

```bash
git -C /Users/cxc/Documents/open/meeting/meeting-desktop add src/App.tsx src/routes/home-page.tsx src/routes/live-meeting-page.tsx src/routes/meeting-detail-page.tsx src/routes/not-found-page.tsx
git -C /Users/cxc/Documents/open/meeting/meeting-desktop commit -m "feat: add meeting route skeletons"
```

### Task 2: Introduce frontend feature directories and typed client-side view models

**Files:**
- Create: `meeting-desktop/src/features/meetings/models.ts`
- Create: `meeting-desktop/src/features/transcript/models.ts`
- Create: `meeting-desktop/src/features/summary/models.ts`
- Create: `meeting-desktop/src/features/session/models.ts`
- Create: `meeting-desktop/src/lib/api/commands.ts`
- Create: `meeting-desktop/src/lib/events/desktop-events.ts`
- Create: `meeting-desktop/src/lib/state/session-view-store.ts`
- Modify: `meeting-desktop/src/stores/meeting-store.ts`
- Test: `meeting-desktop/src/lib/state/session-view-store.ts`

**Step 1: Write the failing test**

Create a store test that expects a typed session view state with transcript and summary slices.

```ts
expect(createInitialSessionViewState().status).toBe("idle");
```

**Step 2: Run test to verify it fails**

Run: `npm test -- session-view-store`
Expected: FAIL because the models and test runner do not exist yet.

**Step 3: Write minimal implementation**

- Add typed view models for meetings, transcript segments, summary panels, and session status.
- Replace the sample Zustand meeting store with a UI-oriented store.
- Add Tauri command and event wrappers without transport logic.

**Step 4: Run test to verify it passes**

Run: `npm test -- session-view-store`
Expected: PASS.

**Step 5: Commit**

```bash
git -C /Users/cxc/Documents/open/meeting/meeting-desktop add src/features src/lib/api src/lib/events src/lib/state src/stores/meeting-store.ts
git -C /Users/cxc/Documents/open/meeting/meeting-desktop commit -m "feat: add typed frontend meeting view models"
```

### Task 3: Add Rust module skeletons for app state, commands, events, protocol, session, transport, audio, storage, and export

**Files:**
- Modify: `meeting-desktop/src-tauri/src/lib.rs`
- Create: `meeting-desktop/src-tauri/src/app_state.rs`
- Create: `meeting-desktop/src-tauri/src/commands/mod.rs`
- Create: `meeting-desktop/src-tauri/src/commands/meeting_commands.rs`
- Create: `meeting-desktop/src-tauri/src/commands/history_commands.rs`
- Create: `meeting-desktop/src-tauri/src/commands/export_commands.rs`
- Create: `meeting-desktop/src-tauri/src/events/mod.rs`
- Create: `meeting-desktop/src-tauri/src/events/types.rs`
- Create: `meeting-desktop/src-tauri/src/events/bus.rs`
- Create: `meeting-desktop/src-tauri/src/protocol/mod.rs`
- Create: `meeting-desktop/src-tauri/src/protocol/messages.rs`
- Create: `meeting-desktop/src-tauri/src/protocol/topics.rs`
- Create: `meeting-desktop/src-tauri/src/protocol/schema.rs`
- Create: `meeting-desktop/src-tauri/src/session/mod.rs`
- Create: `meeting-desktop/src-tauri/src/session/models.rs`
- Create: `meeting-desktop/src-tauri/src/session/state_machine.rs`
- Create: `meeting-desktop/src-tauri/src/session/manager.rs`
- Create: `meeting-desktop/src-tauri/src/transport/mod.rs`
- Create: `meeting-desktop/src-tauri/src/transport/control_transport.rs`
- Create: `meeting-desktop/src-tauri/src/transport/audio_transport.rs`
- Create: `meeting-desktop/src-tauri/src/audio/mod.rs`
- Create: `meeting-desktop/src-tauri/src/storage/mod.rs`
- Create: `meeting-desktop/src-tauri/src/export/mod.rs`
- Test: `meeting-desktop/src-tauri/src/session/state_machine.rs`

**Step 1: Write the failing test**

Add a Rust unit test that verifies the default session state starts at `Idle`.

```rust
#[test]
fn session_state_defaults_to_idle() {
    let machine = SessionStateMachine::default();
    assert_eq!(machine.current(), SessionStatus::Idle);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test session_state_defaults_to_idle`
Expected: FAIL because the modules do not exist yet.

**Step 3: Write minimal implementation**

- Split the Tauri starter into modules.
- Register stub commands.
- Add initial session models and a minimal state machine.

**Step 4: Run test to verify it passes**

Run: `cargo test session_state_defaults_to_idle`
Expected: PASS.

**Step 5: Commit**

```bash
git -C /Users/cxc/Documents/open/meeting/meeting-desktop add src-tauri/src
git -C /Users/cxc/Documents/open/meeting/meeting-desktop commit -m "feat: scaffold rust runtime modules"
```

### Task 4: Add SQLite schema, repositories, and incomplete-session recovery queries

**Files:**
- Modify: `meeting-desktop/src-tauri/Cargo.toml`
- Create: `meeting-desktop/src-tauri/src/storage/db.rs`
- Create: `meeting-desktop/src-tauri/src/storage/migrations.rs`
- Create: `meeting-desktop/src-tauri/src/storage/meetings_repo.rs`
- Create: `meeting-desktop/src-tauri/src/storage/transcript_repo.rs`
- Create: `meeting-desktop/src-tauri/src/storage/summary_repo.rs`
- Create: `meeting-desktop/src-tauri/src/storage/audio_repo.rs`
- Create: `meeting-desktop/src-tauri/src/storage/checkpoint_repo.rs`
- Test: `meeting-desktop/src-tauri/src/storage/migrations.rs`
- Test: `meeting-desktop/src-tauri/src/storage/meetings_repo.rs`

**Step 1: Write the failing test**

Add a migration test that expects the required tables to exist.

```rust
#[test]
fn migrations_create_required_tables() {
    let tables = list_tables(run_test_migrations());
    assert!(tables.contains(&"meetings".to_string()));
    assert!(tables.contains(&"session_checkpoints".to_string()));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test migrations_create_required_tables`
Expected: FAIL because SQLite schema code is missing.

**Step 3: Write minimal implementation**

- Add SQLite dependency and setup.
- Create the initial schema for meetings, transcript segments, summary snapshots, action items, audio assets, and checkpoints.
- Add repository methods for creating meetings and loading incomplete ones.

**Step 4: Run test to verify it passes**

Run: `cargo test migrations_create_required_tables`
Expected: PASS.

**Step 5: Commit**

```bash
git -C /Users/cxc/Documents/open/meeting/meeting-desktop add src-tauri/Cargo.toml src-tauri/src/storage
git -C /Users/cxc/Documents/open/meeting/meeting-desktop commit -m "feat: add meeting sqlite schema and repositories"
```

### Task 5: Implement typed protocol envelopes and MQTT topic builders in Rust and TypeScript

**Files:**
- Modify: `meeting-desktop/src-tauri/src/protocol/messages.rs`
- Modify: `meeting-desktop/src-tauri/src/protocol/topics.rs`
- Modify: `meeting-desktop/src-tauri/src/protocol/schema.rs`
- Create: `meeting-desktop/src/features/session/protocol.ts`
- Create: `meeting-desktop/src/features/session/topics.ts`
- Test: `meeting-desktop/src-tauri/src/protocol/topics.rs`
- Test: `meeting-desktop/src/features/session/topics.ts`

**Step 1: Write the failing test**

Write a Rust test and a TypeScript test that both expect the same topic format.

```rust
assert_eq!(
    control_topic("client-a", "session-1"),
    "meetings/client-a/session/session-1/control"
);
```

**Step 2: Run test to verify it fails**

Run: `cargo test protocol_topics_build_expected_paths`
Expected: FAIL because topic builders are not implemented.

**Step 3: Write minimal implementation**

- Implement typed protocol enums and payload structs.
- Implement topic builders on both sides.
- Keep frontend protocol code type-only and presentation-facing.

**Step 4: Run test to verify it passes**

Run: `cargo test protocol_topics_build_expected_paths`
Expected: PASS.

**Step 5: Commit**

```bash
git -C /Users/cxc/Documents/open/meeting/meeting-desktop add src-tauri/src/protocol src/features/session/protocol.ts src/features/session/topics.ts
git -C /Users/cxc/Documents/open/meeting/meeting-desktop commit -m "feat: add typed meeting protocol envelopes"
```

### Task 6: Implement session lifecycle commands and state transitions for create, start, pause, resume, stop, and recover

**Files:**
- Modify: `meeting-desktop/src-tauri/src/commands/meeting_commands.rs`
- Modify: `meeting-desktop/src-tauri/src/session/models.rs`
- Modify: `meeting-desktop/src-tauri/src/session/state_machine.rs`
- Modify: `meeting-desktop/src-tauri/src/session/manager.rs`
- Modify: `meeting-desktop/src-tauri/src/events/types.rs`
- Test: `meeting-desktop/src-tauri/src/session/state_machine.rs`
- Test: `meeting-desktop/src-tauri/src/session/manager.rs`

**Step 1: Write the failing test**

Write a Rust test covering the happy-path state transition sequence.

```rust
#[test]
fn start_pause_resume_stop_follows_valid_transitions() {
    let mut machine = SessionStateMachine::default();
    machine.transition(SessionEvent::ConnectSucceeded).unwrap();
    machine.transition(SessionEvent::RecordingStarted).unwrap();
    machine.transition(SessionEvent::PauseRequested).unwrap();
    machine.transition(SessionEvent::ResumeRequested).unwrap();
    machine.transition(SessionEvent::StopRequested).unwrap();
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test start_pause_resume_stop_follows_valid_transitions`
Expected: FAIL until lifecycle events are implemented.

**Step 3: Write minimal implementation**

- Define the session event enum.
- Implement transition guards.
- Expose Tauri commands that call into `SessionManager`.
- Emit typed runtime updates for the UI.

**Step 4: Run test to verify it passes**

Run: `cargo test start_pause_resume_stop_follows_valid_transitions`
Expected: PASS.

**Step 5: Commit**

```bash
git -C /Users/cxc/Documents/open/meeting/meeting-desktop add src-tauri/src/commands src-tauri/src/session src-tauri/src/events
git -C /Users/cxc/Documents/open/meeting/meeting-desktop commit -m "feat: add meeting lifecycle state machine"
```

### Task 7: Implement frontend live session views that consume Rust events instead of owning business logic

**Files:**
- Modify: `meeting-desktop/src/routes/home-page.tsx`
- Modify: `meeting-desktop/src/routes/live-meeting-page.tsx`
- Modify: `meeting-desktop/src/routes/meeting-detail-page.tsx`
- Create: `meeting-desktop/src/features/meetings/components/meeting-history-list.tsx`
- Create: `meeting-desktop/src/features/session/components/live-session-header.tsx`
- Create: `meeting-desktop/src/features/transcript/components/transcript-stream-panel.tsx`
- Create: `meeting-desktop/src/features/summary/components/live-summary-panel.tsx`
- Create: `meeting-desktop/src/features/session/hooks/use-live-session.ts`
- Test: `meeting-desktop/src/features/session/hooks/use-live-session.ts`

**Step 1: Write the failing test**

Write a hook test that verifies UI state updates when a mocked desktop event arrives.

```ts
expect(result.current.status).toBe("recording");
```

**Step 2: Run test to verify it fails**

Run: `npm test -- use-live-session`
Expected: FAIL because the hook and mocks are missing.

**Step 3: Write minimal implementation**

- Build the three required pages.
- Subscribe to typed desktop events.
- Update only UI-facing store state.
- Keep transport and persistence logic out of React.

**Step 4: Run test to verify it passes**

Run: `npm test -- use-live-session`
Expected: PASS.

**Step 5: Commit**

```bash
git -C /Users/cxc/Documents/open/meeting/meeting-desktop add src/routes src/features
git -C /Users/cxc/Documents/open/meeting/meeting-desktop commit -m "feat: add meeting workspace ui"
```

### Task 8: Add MQTT control transport with typed message dispatch, heartbeats, and reconnect hooks

**Files:**
- Modify: `meeting-desktop/src-tauri/Cargo.toml`
- Create: `meeting-desktop/src-tauri/src/transport/mqtt_control.rs`
- Create: `meeting-desktop/src-tauri/src/transport/reconnect.rs`
- Modify: `meeting-desktop/src-tauri/src/transport/control_transport.rs`
- Modify: `meeting-desktop/src-tauri/src/session/manager.rs`
- Modify: `meeting-desktop/src-tauri/src/events/bus.rs`
- Test: `meeting-desktop/src-tauri/src/transport/mqtt_control.rs`
- Test: `meeting-desktop/src-tauri/src/session/manager.rs`

**Step 1: Write the failing test**

Write a Rust test that verifies an incoming `stt_delta` MQTT message dispatches a typed runtime event.

```rust
assert_eq!(event.kind, RuntimeEventKind::TranscriptDelta);
```

**Step 2: Run test to verify it fails**

Run: `cargo test mqtt_dispatches_transcript_delta`
Expected: FAIL because the MQTT transport is not implemented.

**Step 3: Write minimal implementation**

- Add an MQTT client dependency.
- Build control connect/disconnect logic.
- Subscribe to the agreed topics.
- Dispatch typed events into the runtime bus.
- Add heartbeat and reconnect flags.

**Step 4: Run test to verify it passes**

Run: `cargo test mqtt_dispatches_transcript_delta`
Expected: PASS.

**Step 5: Commit**

```bash
git -C /Users/cxc/Documents/open/meeting/meeting-desktop add src-tauri/Cargo.toml src-tauri/src/transport src-tauri/src/session/manager.rs src-tauri/src/events/bus.rs
git -C /Users/cxc/Documents/open/meeting/meeting-desktop commit -m "feat: add mqtt control transport"
```

### Task 9: Implement Windows audio device enumeration and dual capture skeletons

**Files:**
- Modify: `meeting-desktop/src-tauri/Cargo.toml`
- Create: `meeting-desktop/src-tauri/src/audio/coordinator.rs`
- Create: `meeting-desktop/src-tauri/src/audio/platform/windows/device_enumerator.rs`
- Create: `meeting-desktop/src-tauri/src/audio/platform/windows/mic_capture.rs`
- Create: `meeting-desktop/src-tauri/src/audio/platform/windows/loopback_capture.rs`
- Modify: `meeting-desktop/src-tauri/src/audio/mod.rs`
- Test: `meeting-desktop/src-tauri/src/audio/coordinator.rs`

**Step 1: Write the failing test**

Write a Rust test for the coordinator that expects two capture sources to register before start.

```rust
assert_eq!(coordinator.source_count(), 2);
```

**Step 2: Run test to verify it fails**

Run: `cargo test audio_coordinator_requires_mic_and_system_sources`
Expected: FAIL because the audio coordinator does not exist.

**Step 3: Write minimal implementation**

- Add a Windows-only audio platform module.
- Stub device enumeration.
- Add mic and system capture traits and startup wiring.
- Keep macOS behind placeholder modules only.

**Step 4: Run test to verify it passes**

Run: `cargo test audio_coordinator_requires_mic_and_system_sources`
Expected: PASS.

**Step 5: Commit**

```bash
git -C /Users/cxc/Documents/open/meeting/meeting-desktop add src-tauri/Cargo.toml src-tauri/src/audio
git -C /Users/cxc/Documents/open/meeting/meeting-desktop commit -m "feat: add windows dual capture skeleton"
```

### Task 10: Add WAV writers, timeline alignment, and mixed uplink chunk generation

**Files:**
- Create: `meeting-desktop/src-tauri/src/audio/writer.rs`
- Create: `meeting-desktop/src-tauri/src/audio/timeline.rs`
- Create: `meeting-desktop/src-tauri/src/audio/mixer.rs`
- Create: `meeting-desktop/src-tauri/src/audio/chunker.rs`
- Create: `meeting-desktop/src-tauri/src/audio/buffer.rs`
- Modify: `meeting-desktop/src-tauri/src/audio/coordinator.rs`
- Modify: `meeting-desktop/src-tauri/src/storage/audio_repo.rs`
- Test: `meeting-desktop/src-tauri/src/audio/mixer.rs`
- Test: `meeting-desktop/src-tauri/src/audio/chunker.rs`

**Step 1: Write the failing test**

Write a mixer test that expects aligned mic and system PCM inputs to produce a mixed mono frame.

```rust
assert_eq!(mixed.len(), expected_len);
```

**Step 2: Run test to verify it fails**

Run: `cargo test mixer_combines_aligned_sources_into_mono_frame`
Expected: FAIL because the mixer is missing.

**Step 3: Write minimal implementation**

- Add WAV writing for mic, system, and mixed assets.
- Add timeline alignment for dual-source PCM.
- Build mixed frames and 200ms chunks.
- Persist audio asset paths.

**Step 4: Run test to verify it passes**

Run: `cargo test mixer_combines_aligned_sources_into_mono_frame`
Expected: PASS.

**Step 5: Commit**

```bash
git -C /Users/cxc/Documents/open/meeting/meeting-desktop add src-tauri/src/audio src-tauri/src/storage/audio_repo.rs
git -C /Users/cxc/Documents/open/meeting/meeting-desktop commit -m "feat: add wav writers and mixed uplink pipeline"
```

### Task 11: Implement UDP mixed-audio transport and upload checkpoint tracking

**Files:**
- Create: `meeting-desktop/src-tauri/src/protocol/udp_packet.rs`
- Create: `meeting-desktop/src-tauri/src/transport/udp_audio.rs`
- Modify: `meeting-desktop/src-tauri/src/transport/audio_transport.rs`
- Modify: `meeting-desktop/src-tauri/src/audio/coordinator.rs`
- Modify: `meeting-desktop/src-tauri/src/storage/checkpoint_repo.rs`
- Test: `meeting-desktop/src-tauri/src/transport/udp_audio.rs`
- Test: `meeting-desktop/src-tauri/src/protocol/udp_packet.rs`

**Step 1: Write the failing test**

Write a packet encoding test that expects the chunk metadata to survive serialization.

```rust
assert_eq!(decoded.seq, 42);
assert_eq!(decoded.source_type, AudioSourceType::Mixed);
```

**Step 2: Run test to verify it fails**

Run: `cargo test udp_packet_round_trips_mixed_chunk_metadata`
Expected: FAIL because UDP packet encoding is missing.

**Step 3: Write minimal implementation**

- Define the UDP packet format.
- Implement async send logic.
- Track last uploaded mixed offset in checkpoints.
- Hook send completion back into recovery metadata.

**Step 4: Run test to verify it passes**

Run: `cargo test udp_packet_round_trips_mixed_chunk_metadata`
Expected: PASS.

**Step 5: Commit**

```bash
git -C /Users/cxc/Documents/open/meeting/meeting-desktop add src-tauri/src/protocol/udp_packet.rs src-tauri/src/transport/udp_audio.rs src-tauri/src/storage/checkpoint_repo.rs src-tauri/src/audio/coordinator.rs
git -C /Users/cxc/Documents/open/meeting/meeting-desktop commit -m "feat: add udp mixed audio uplink"
```

### Task 12: Persist transcript deltas and finals and render incremental transcript updates

**Files:**
- Modify: `meeting-desktop/src-tauri/src/session/manager.rs`
- Modify: `meeting-desktop/src-tauri/src/storage/transcript_repo.rs`
- Modify: `meeting-desktop/src-tauri/src/events/types.rs`
- Modify: `meeting-desktop/src/features/transcript/models.ts`
- Modify: `meeting-desktop/src/features/transcript/components/transcript-stream-panel.tsx`
- Test: `meeting-desktop/src-tauri/src/storage/transcript_repo.rs`
- Test: `meeting-desktop/src/features/transcript/components/transcript-stream-panel.tsx`

**Step 1: Write the failing test**

Write a Rust test that expects a delta update and final update to collapse into one persisted segment revision chain.

```rust
assert_eq!(segment.revision, 2);
assert!(segment.is_final);
```

**Step 2: Run test to verify it fails**

Run: `cargo test transcript_repo_updates_segment_revision_and_final_state`
Expected: FAIL because transcript persistence logic is incomplete.

**Step 3: Write minimal implementation**

- Persist transcript delta and final messages.
- Emit typed UI events.
- Update the transcript panel with local patching rather than full replacement.

**Step 4: Run test to verify it passes**

Run: `cargo test transcript_repo_updates_segment_revision_and_final_state`
Expected: PASS.

**Step 5: Commit**

```bash
git -C /Users/cxc/Documents/open/meeting/meeting-desktop add src-tauri/src/session/manager.rs src-tauri/src/storage/transcript_repo.rs src-tauri/src/events/types.rs src/features/transcript
git -C /Users/cxc/Documents/open/meeting/meeting-desktop commit -m "feat: add realtime transcript persistence"
```

### Task 13: Persist summary snapshots and action items and render live meeting notes

**Files:**
- Modify: `meeting-desktop/src-tauri/src/storage/summary_repo.rs`
- Modify: `meeting-desktop/src-tauri/src/session/manager.rs`
- Modify: `meeting-desktop/src/features/summary/models.ts`
- Modify: `meeting-desktop/src/features/summary/components/live-summary-panel.tsx`
- Test: `meeting-desktop/src-tauri/src/storage/summary_repo.rs`
- Test: `meeting-desktop/src/features/summary/components/live-summary-panel.tsx`

**Step 1: Write the failing test**

Write a Rust test that expects summary deltas to create snapshots and a final summary to mark the last snapshot final.

```rust
assert!(snapshot.is_final);
assert_eq!(snapshot.version, 3);
```

**Step 2: Run test to verify it fails**

Run: `cargo test summary_repo_stores_delta_versions_and_final_snapshot`
Expected: FAIL because summary persistence is incomplete.

**Step 3: Write minimal implementation**

- Persist summary snapshots and action items.
- Merge live and final note states cleanly.
- Update the UI summary panel with in-place refreshes.

**Step 4: Run test to verify it passes**

Run: `cargo test summary_repo_stores_delta_versions_and_final_snapshot`
Expected: PASS.

**Step 5: Commit**

```bash
git -C /Users/cxc/Documents/open/meeting/meeting-desktop add src-tauri/src/storage/summary_repo.rs src-tauri/src/session/manager.rs src/features/summary
git -C /Users/cxc/Documents/open/meeting/meeting-desktop commit -m "feat: add live meeting summary persistence"
```

### Task 14: Implement crash recovery with local mixed-audio replay and incomplete-session resume flows

**Files:**
- Create: `meeting-desktop/src-tauri/src/session/recovery.rs`
- Modify: `meeting-desktop/src-tauri/src/session/manager.rs`
- Modify: `meeting-desktop/src-tauri/src/storage/checkpoint_repo.rs`
- Modify: `meeting-desktop/src-tauri/src/commands/history_commands.rs`
- Modify: `meeting-desktop/src/routes/home-page.tsx`
- Test: `meeting-desktop/src-tauri/src/session/recovery.rs`
- Test: `meeting-desktop/src/routes/home-page.tsx`

**Step 1: Write the failing test**

Write a Rust test that expects the recovery service to compute the unuploaded mixed range from checkpoint data and local file duration.

```rust
assert_eq!(recovery_plan.replay_from_ms, 180_000);
```

**Step 2: Run test to verify it fails**

Run: `cargo test recovery_plan_uses_checkpoint_and_local_audio_duration`
Expected: FAIL because recovery planning is not implemented.

**Step 3: Write minimal implementation**

- Add recovery planning from checkpoints.
- Add a command to load incomplete meetings.
- Show recovery prompts on the home page.
- Replay the unuploaded mixed audio range before returning to live capture.

**Step 4: Run test to verify it passes**

Run: `cargo test recovery_plan_uses_checkpoint_and_local_audio_duration`
Expected: PASS.

**Step 5: Commit**

```bash
git -C /Users/cxc/Documents/open/meeting/meeting-desktop add src-tauri/src/session/recovery.rs src-tauri/src/session/manager.rs src-tauri/src/storage/checkpoint_repo.rs src-tauri/src/commands/history_commands.rs src/routes/home-page.tsx
git -C /Users/cxc/Documents/open/meeting/meeting-desktop commit -m "feat: add meeting crash recovery"
```

### Task 15: Add Markdown export and meeting detail history views

**Files:**
- Create: `meeting-desktop/src-tauri/src/export/markdown.rs`
- Modify: `meeting-desktop/src-tauri/src/commands/export_commands.rs`
- Modify: `meeting-desktop/src/routes/meeting-detail-page.tsx`
- Create: `meeting-desktop/src/features/summary/components/action-items-panel.tsx`
- Test: `meeting-desktop/src-tauri/src/export/markdown.rs`
- Test: `meeting-desktop/src/routes/meeting-detail-page.tsx`

**Step 1: Write the failing test**

Write a Rust test that expects the Markdown exporter to include transcript, summary, and action item sections.

```rust
assert!(markdown.contains("## 行动项"));
```

**Step 2: Run test to verify it fails**

Run: `cargo test markdown_export_contains_summary_transcript_and_action_items`
Expected: FAIL because export logic is missing.

**Step 3: Write minimal implementation**

- Build the Markdown formatter.
- Expose an export command.
- Render the meeting detail page with transcript, summary, and action items.

**Step 4: Run test to verify it passes**

Run: `cargo test markdown_export_contains_summary_transcript_and_action_items`
Expected: PASS.

**Step 5: Commit**

```bash
git -C /Users/cxc/Documents/open/meeting/meeting-desktop add src-tauri/src/export/markdown.rs src-tauri/src/commands/export_commands.rs src/routes/meeting-detail-page.tsx src/features/summary/components/action-items-panel.tsx
git -C /Users/cxc/Documents/open/meeting/meeting-desktop commit -m "feat: add markdown export and meeting detail view"
```

### Task 16: Evolve meeting-server from Go starter into session, transport, and realtime pipeline skeletons

**Files:**
- Modify: `meeting-server/main.go`
- Create: `meeting-server/cmd/server/main.go`
- Create: `meeting-server/internal/app/app.go`
- Create: `meeting-server/internal/protocol/messages.go`
- Create: `meeting-server/internal/protocol/topics.go`
- Create: `meeting-server/internal/transport/mqtt/server.go`
- Create: `meeting-server/internal/transport/udp/server.go`
- Create: `meeting-server/internal/session/manager.go`
- Create: `meeting-server/internal/pipeline/stt/service.go`
- Create: `meeting-server/internal/pipeline/summary/service.go`
- Create: `meeting-server/internal/pipeline/action_items/service.go`
- Test: `meeting-server/internal/protocol/topics_test.go`
- Test: `meeting-server/internal/session/manager_test.go`

**Step 1: Write the failing test**

Write a Go test that expects the server topic builder to match the desktop contract.

```go
func TestControlTopic(t *testing.T) {
  got := ControlTopic("client-a", "session-1")
  want := "meetings/client-a/session/session-1/control"
  if got != want {
    t.Fatalf("got %s want %s", got, want)
  }
}
```

**Step 2: Run test to verify it fails**

Run: `go test ./...`
Expected: FAIL because the server protocol package does not exist.

**Step 3: Write minimal implementation**

- Move the starter executable under `cmd/server`.
- Add server-side protocol and topic builders.
- Add MQTT and UDP transport skeletons.
- Add a session manager and placeholder STT and summary pipelines.

**Step 4: Run test to verify it passes**

Run: `go test ./...`
Expected: PASS.

**Step 5: Commit**

```bash
git -C /Users/cxc/Documents/open/meeting/meeting-server add .
git -C /Users/cxc/Documents/open/meeting/meeting-server commit -m "feat: scaffold meeting server runtime"
```

### Task 17: Add backend control-session flow for hello, start, stop, heartbeat, and typed event fanout

**Files:**
- Modify: `meeting-server/internal/transport/mqtt/server.go`
- Modify: `meeting-server/internal/session/manager.go`
- Modify: `meeting-server/internal/protocol/messages.go`
- Test: `meeting-server/internal/session/manager_test.go`

**Step 1: Write the failing test**

Write a Go test that expects `session/hello` to allocate session metadata and return UDP connection details.

```go
if reply.Type != "session/hello" {
  t.Fatalf("unexpected reply type %s", reply.Type)
}
```

**Step 2: Run test to verify it fails**

Run: `go test ./internal/session ./internal/transport/mqtt`
Expected: FAIL because hello handling is not implemented.

**Step 3: Write minimal implementation**

- Handle hello, start, stop, and heartbeat requests.
- Create session metadata and UDP handoff details.
- Publish typed control replies and lifecycle events.

**Step 4: Run test to verify it passes**

Run: `go test ./internal/session ./internal/transport/mqtt`
Expected: PASS.

**Step 5: Commit**

```bash
git -C /Users/cxc/Documents/open/meeting/meeting-server add internal/transport/mqtt/server.go internal/session/manager.go internal/protocol/messages.go
git -C /Users/cxc/Documents/open/meeting/meeting-server commit -m "feat: add backend session control flow"
```

### Task 18: Add backend UDP ingest, transcript event emission, summary event emission, and end-of-session flush

**Files:**
- Modify: `meeting-server/internal/transport/udp/server.go`
- Modify: `meeting-server/internal/pipeline/stt/service.go`
- Modify: `meeting-server/internal/pipeline/summary/service.go`
- Modify: `meeting-server/internal/pipeline/action_items/service.go`
- Modify: `meeting-server/internal/session/manager.go`
- Test: `meeting-server/internal/transport/udp/server_test.go`
- Test: `meeting-server/internal/session/manager_test.go`

**Step 1: Write the failing test**

Write a Go test that expects mixed audio packets to produce transcript deltas and a final summary on stop.

```go
if len(events) == 0 {
  t.Fatal("expected transcript or summary events")
}
```

**Step 2: Run test to verify it fails**

Run: `go test ./internal/transport/udp ./internal/session`
Expected: FAIL because UDP ingest and pipeline fanout are incomplete.

**Step 3: Write minimal implementation**

- Accept UDP mixed audio packets.
- Feed STT and summary services.
- Publish `stt_delta`, `stt_final`, `summary_delta`, `summary_final`, `action_item_delta`, and `action_item_final`.
- Flush final events on stop.

**Step 4: Run test to verify it passes**

Run: `go test ./internal/transport/udp ./internal/session`
Expected: PASS.

**Step 5: Commit**

```bash
git -C /Users/cxc/Documents/open/meeting/meeting-server add internal/transport/udp/server.go internal/pipeline/stt/service.go internal/pipeline/summary/service.go internal/pipeline/action_items/service.go internal/session/manager.go
git -C /Users/cxc/Documents/open/meeting/meeting-server commit -m "feat: add backend realtime meeting pipeline"
```

### Task 19: Add verification scripts and end-to-end smoke checks for desktop and backend integration

**Files:**
- Create: `meeting-desktop/package.json` updates for tests
- Create: `meeting-desktop/src-tauri/tests/session_smoke.rs`
- Create: `meeting-server/test/integration/session_flow_test.go`
- Create: `meeting-desktop/docs/plans/verification-notes.md`

**Step 1: Write the failing test**

Create one desktop smoke test and one backend integration test for a short meeting lifecycle.

```rust
assert_eq!(status, SessionStatus::Completed);
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test session_smoke`
Expected: FAIL before the full pipeline is integrated.

**Step 3: Write minimal implementation**

- Add test scripts to `package.json`.
- Add a Rust smoke test for lifecycle completion.
- Add a Go integration test for session flow.
- Document manual verification for Windows audio capture and local file output.

**Step 4: Run test to verify it passes**

Run: `cargo test --test session_smoke`
Expected: PASS after the lifecycle is wired end-to-end.

**Step 5: Commit**

```bash
git -C /Users/cxc/Documents/open/meeting/meeting-desktop add package.json src-tauri/tests meeting-desktop/docs/plans/verification-notes.md
git -C /Users/cxc/Documents/open/meeting/meeting-desktop commit -m "test: add desktop verification coverage"
git -C /Users/cxc/Documents/open/meeting/meeting-server add test/integration/session_flow_test.go
git -C /Users/cxc/Documents/open/meeting/meeting-server commit -m "test: add backend integration coverage"
```

## Notes for Execution

- Keep React presentation-only. Do not let pages talk directly to MQTT, UDP, or SQLite.
- Prefer Rust unit tests for protocol, state, storage, recovery, and audio chunk logic.
- Prefer Go unit tests for protocol, session flow, and typed event publication.
- Leave macOS as explicit extension points only; do not implement it in MVP tasks.
- Preserve local WAV assets before attempting aggressive optimization.
- Do not introduce Opus until the PCM pipeline is stable end-to-end.
