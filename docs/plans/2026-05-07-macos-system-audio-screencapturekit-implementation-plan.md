# macOS System Audio ScreenCaptureKit Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the macOS system-audio process tap backend with a single ScreenCaptureKit backend and remove the old tap-specific implementation.

**Architecture:** Keep the Rust `MacosSystemAudioCapture` surface and shared PCM worker/resampler path, but swap the Objective-C++ bridge from Core Audio process taps to ScreenCaptureKit `SCStream` audio callbacks. Preserve the rest of the desktop audio pipeline unchanged.

**Tech Stack:** Rust 2021, Tauri 2, Objective-C++, ScreenCaptureKit, CoreMedia, AVFoundation/CoreAudio buffer access, Vitest, Rust unit tests.

---

### Task 1: Add a failing Rust test for ScreenCaptureKit permission messaging

**Files:**
- Modify: `src-tauri/src/audio/platform/macos/system_audio.rs`

**Step 1: Write the failing test**

Add a unit test asserting that a ScreenCaptureKit-style permission-denied message becomes a clear screen-recording permission error.

**Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path /Users/cxc/Documents/open/meeting/meeting-desktop/src-tauri/Cargo.toml format_start_error_explains_screen_recording_permission_denied`

Expected: FAIL because the current formatter does not emit the new message.

**Step 3: Write minimal implementation**

Update `format_start_error(...)` and permission detection helpers to recognize ScreenCaptureKit permission-denied wording.

**Step 4: Run test to verify it passes**

Run the same command and confirm PASS.

### Task 2: Replace the native bridge with ScreenCaptureKit audio capture

**Files:**
- Modify: `src-tauri/native/macos/system_audio_bridge.h`
- Replace: `src-tauri/native/macos/system_audio_bridge.mm`

**Step 1: Preserve the C ABI**

Keep:

- `meeting_system_audio_start(...)`
- `meeting_system_audio_stop(...)`
- `meeting_system_audio_callback`

**Step 2: Remove process-tap setup**

Delete:

- Core Audio tap creation
- aggregate device setup
- IOProc callback path
- tap-specific debug logging

**Step 3: Add ScreenCaptureKit stream startup**

Implement:

- display discovery via `SCShareableContent`
- `SCStreamConfiguration` with audio capture enabled
- current-process audio exclusion
- native output handler receiving audio sample buffers

**Step 4: Forward float PCM frames**

Extract audio data from `CMSampleBuffer` / `AudioBufferList` and invoke the existing Rust callback with:

- `started_at_ms`
- `const float *samples`
- `sample_count`
- `sample_rate_hz`
- `channels`

**Step 5: Implement deterministic stop**

Stop and release:

- `SCStream`
- stream output handler
- dispatch queue
- retained bridge state

### Task 3: Update build wiring for ScreenCaptureKit

**Files:**
- Modify: `src-tauri/build.rs`

**Step 1: Write minimal build change**

Add the needed framework link directives for:

- `ScreenCaptureKit`
- `CoreMedia`
- `CoreAudio`
- `Foundation`

Keep the current Objective-C++ compilation path.

**Step 2: Run compile verification**

Run: `cargo test --manifest-path /Users/cxc/Documents/open/meeting/meeting-desktop/src-tauri/Cargo.toml`

Expected: builds and tests run successfully.

### Task 4: Clean up Rust-side backend language and keep shared worker logic

**Files:**
- Modify: `src-tauri/src/audio/platform/macos/system_audio.rs`
- Modify: `meeting-desktop/README.md`

**Step 1: Remove tap-specific wording**

Update user-facing errors and docs so they describe ScreenCaptureKit / Screen Recording permission instead of Core Audio process taps.

**Step 2: Keep shared conversion path**

Retain:

- `StreamPcmConverter`
- worker queue
- stop semantics
- PCM normalization tests

Do not introduce dual backend branching.

### Task 5: Run verification

**Files:**
- Test: `src-tauri/src/audio/platform/macos/system_audio.rs`

**Step 1: Run focused Rust tests**

Run:

- `cargo test --manifest-path /Users/cxc/Documents/open/meeting/meeting-desktop/src-tauri/Cargo.toml format_start_error_explains_screen_recording_permission_denied`
- `cargo test --manifest-path /Users/cxc/Documents/open/meeting/meeting-desktop/src-tauri/Cargo.toml system_audio`

Expected: PASS.

**Step 2: Run full Rust verification**

Run:

- `cargo test --manifest-path /Users/cxc/Documents/open/meeting/meeting-desktop/src-tauri/Cargo.toml`

Expected: PASS.

**Step 3: Run frontend build verification**

Run:

- `npm run build`

Expected: PASS.
