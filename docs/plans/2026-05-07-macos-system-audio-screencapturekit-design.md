# macOS System Audio ScreenCaptureKit Design

## Goal

Replace the current macOS Core Audio process-tap system audio backend with a single ScreenCaptureKit-based backend for `MEETING_MACOS_SYSTEM_AUDIO=system`.

## Why Change

Recent live diagnostics on this machine showed the existing process-tap path binds successfully and receives callback timing, but both callback buffer sides remain all-zero while browser audio is playing. That means the current backend is structurally active but not delivering usable system audio samples in practice.

Keeping both the old tap path and a new ScreenCaptureKit path would increase complexity in the macOS capture startup flow, error handling, and native bridge code. The user explicitly asked to prefer the latest supported approach and remove the old implementation.

## Recommended Approach

Use ScreenCaptureKit `SCStream` with audio capture enabled and current-process audio excluded. The native bridge will own the `SCStream` lifecycle, receive audio `CMSampleBuffer` callbacks, extract float PCM from the sample buffer, and forward those frames through the existing Rust callback boundary.

The Rust-side `MacosSystemAudioCapture` API stays stable:

- `MacosSystemAudioCapture::default()`
- `MacosSystemAudioCapture::start_with_sink(...)`
- `MacosSystemCaptureRuntime::stop()`

This keeps the rest of the desktop pipeline unchanged:

- `CaptureSourceKind::SystemLoopback`
- `MeetingAudioRuntime`
- `system-original.wav`
- `mixed-uplink.wav`
- UDP uplink
- runtime diagnostics UI

## Native Behavior

- On start, the bridge checks for a capturable display and configures `SCStreamConfiguration`.
- Enable audio capture and exclude the current process audio.
- Register an audio-only stream output handler.
- Convert incoming audio sample buffers into interleaved float PCM before invoking the Rust callback.
- On stop, tear down the stream, output handler, and dispatch queue cleanly.

## Permissions

System audio capture will now depend on macOS Screen Recording permission. If permission is missing or the stream cannot start, meeting start should fail clearly with an actionable message instead of falling back silently.

## Cleanup Scope

To avoid backend drift:

- remove the old process-tap implementation details from the native bridge
- remove tap-specific error strings and diagnostics that no longer describe the active backend
- keep only the generic Rust-side buffering, resampling, and callback worker logic that still applies to the new native bridge

## Testing

Automated:

- error formatting for ScreenCaptureKit permission failure
- existing PCM normalization and worker-thread tests remain green
- build verification must confirm ScreenCaptureKit framework linking

Manual:

- start `npm run dev:local`
- grant Screen Recording permission if prompted
- play browser audio through the current output device
- confirm runtime panel shows `系统输入` as `接收中`
- confirm `system-original.wav` contains non-zero samples

