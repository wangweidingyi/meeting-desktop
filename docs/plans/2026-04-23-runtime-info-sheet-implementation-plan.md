# Runtime Info Sheet Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a persistent bottom-right runtime info button on the live meeting page that opens a sheet showing detailed session, control transport, and audio uplink diagnostics.

**Architecture:** Extend the Rust runtime event stream with a dedicated diagnostics payload for audio uplink state and transport target metadata, then mirror that payload into the frontend session store. The live meeting page will render a reusable sheet-style diagnostics panel fed entirely from store state so more runtime sections can be added later without reshaping the page.

**Tech Stack:** React 19, Zustand, Vitest, Tauri v2, Rust, serde

---

### Task 1: Lock in frontend diagnostics state handling

**Files:**
- Modify: `meeting-desktop/src/features/session/hooks/use-live-session.test.tsx`
- Modify: `meeting-desktop/src/lib/state/session-view-store.ts`
- Modify: `meeting-desktop/src/features/session/models.ts`
- Modify: `meeting-desktop/src/lib/events/desktop-events.ts`

**Step 1: Write the failing test**

Add a hook test proving a new runtime diagnostics desktop event updates the store with audio uplink state, target address, last uploaded offset, and last transport error metadata.

**Step 2: Run test to verify it fails**

Run: `npm test -- src/features/session/hooks/use-live-session.test.tsx`
Expected: FAIL because no runtime diagnostics event or store fields exist yet.

**Step 3: Write minimal implementation**

Introduce typed runtime diagnostics payloads and store update methods, then subscribe to the new event inside `useLiveSession`.

**Step 4: Run test to verify it passes**

Run: `npm test -- src/features/session/hooks/use-live-session.test.tsx`
Expected: PASS

### Task 2: Lock in the sheet UI

**Files:**
- Create: `meeting-desktop/src/components/ui/sheet.tsx`
- Create: `meeting-desktop/src/features/session/components/runtime-info-sheet.tsx`
- Create: `meeting-desktop/src/features/session/components/runtime-info-sheet.test.tsx`
- Modify: `meeting-desktop/src/routes/live-meeting-page.tsx`

**Step 1: Write the failing test**

Add a component test proving the runtime info button opens a sheet and shows session info, control status, audio uplink status, target address, and upload progress.

**Step 2: Run test to verify it fails**

Run: `npm test -- src/features/session/components/runtime-info-sheet.test.tsx`
Expected: FAIL because the button and sheet component do not exist.

**Step 3: Write minimal implementation**

Create a lightweight right-side sheet component and render the runtime info panel from store-backed diagnostics state on the live meeting page.

**Step 4: Run test to verify it passes**

Run: `npm test -- src/features/session/components/runtime-info-sheet.test.tsx`
Expected: PASS

### Task 3: Lock in Rust diagnostics events

**Files:**
- Modify: `meeting-desktop/src-tauri/src/events/types.rs`
- Modify: `meeting-desktop/src-tauri/src/events/processor.rs`
- Modify: `meeting-desktop/src-tauri/src/audio/runtime.rs`
- Modify: `meeting-desktop/src-tauri/src/commands/meeting_commands.rs`
- Modify: `meeting-desktop/src-tauri/src/transport/runtime.rs`

**Step 1: Write the failing test**

Add a Rust test proving the audio runtime publishes runtime diagnostics events when audio chunks are uploaded and that pause/stop transitions can surface updated uplink state.

**Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml audio_runtime_publishes_runtime_diagnostics_on_upload`
Expected: FAIL because no diagnostics event is emitted yet.

**Step 3: Write minimal implementation**

Add a runtime diagnostics event payload, emit it from audio runtime and meeting command transitions, and expose it through the Tauri event processor.

**Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml audio_runtime_publishes_runtime_diagnostics_on_upload`
Expected: PASS

### Task 4: Verify the integrated flow

**Files:**
- Modify: `meeting-desktop/src/features/session/hooks/use-live-session.test.tsx`
- Modify: `meeting-desktop/src/features/session/components/runtime-info-sheet.test.tsx`
- Modify: `meeting-desktop/src-tauri/src/audio/runtime.rs`

**Step 1: Run focused frontend tests**

Run: `npm test -- src/features/session/hooks/use-live-session.test.tsx src/features/session/components/runtime-info-sheet.test.tsx`
Expected: PASS

**Step 2: Run focused Rust tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml audio_runtime_publishes_runtime_diagnostics_on_upload`
Expected: PASS

**Step 3: Run broader verification**

Run: `npm test`
Run: `npm run test:desktop-rust`
Expected: PASS unless there are unrelated pre-existing failures
