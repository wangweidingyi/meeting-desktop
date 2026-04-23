# Meeting Assistant Verification Notes

## Automated checks

- Frontend unit tests: `npm test`
- Frontend production build: `npm run build`
- Desktop Rust tests: `npm run test:desktop-rust`
- Desktop opt-in local UDP runtime tests: `npm run test:desktop-network`
- Desktop smoke test: `npm run test:desktop-smoke`
- Desktop local backend launcher: `npm run backend:run`
- Desktop local Tauri launcher with `.env` loading: `npm run dev:local`
- Backend unit and integration tests:
  Run in `/Users/cxc/Documents/open/meeting/meeting-server`
  `go test ./...`

## Current automated coverage

- Session state transitions and meeting lifecycle persistence.
- Audio runtime preparation persists meeting-scoped WAV asset paths and upload checkpoints before live capture begins.
- Dual-source PCM ingress aligns microphone/system sample pushes, appends source and mixed WAV files, and advances UDP upload checkpoints.
- Single-source passthrough mode can advance mixed uplink checkpoints when only the microphone source is available on the current development host.
- Meeting startup replays any pending `mixed-uplink.wav` range after the last uploaded checkpoint so previously unsent audio can be resent before new live chunks continue.
- The home page can reopen a recoverable meeting, rebind the same meeting id into the active Rust session manager, and prefill the live workspace with persisted transcript/summary/action-item content before new realtime events arrive.
- Windows capture worker lifecycle stops cleanly and joins background threads when a meeting ends.
- Windows capture PCM payload decoders convert raw `i16` and `f32` device buffers into runtime PCM16 samples.
- Windows capture sink adapters downmix and resample incoming device frames before forwarding them into the shared meeting audio runtime.
- macOS microphone capture resamples the default input device into 16k mono and feeds the shared runtime through a thread-owned CoreAudio stream handle.
- macOS can opt into a development-only mirror mode that duplicates microphone frames into the `system` source so the dual-source mixed uplink path can be exercised locally without native loopback support.
- SQLite migrations, meeting history queries, transcript storage, summary snapshots, and recovery planning.
- Runtime event processor writes transcript deltas and summary snapshots from the EventBus into SQLite.
- Runtime transport-state events surface MQTT connect / reconnect / disconnect changes into the frontend store.
- Markdown export assembly for transcript, summary, and action items.
- MQTT topic routing, broker-backed control publish / subscribe, and typed realtime event fanout.
- UDP mixed-audio packet encoding and backend decode path.
- Backend session flow from `session/hello` to realtime delta events and final flush events.

## Windows manual verification checklist

- Start the desktop app on Windows and create a new meeting.
- Confirm the meeting enters `connecting -> ready -> recording` without blocking the UI.
- Verify microphone capture and system loopback capture can both be selected or defaulted.
- Confirm local WAV files are created for microphone, system audio, and mixed output.
- While recording, disconnect the backend or broker once to confirm the client surfaces error / reconnect state instead of freezing.
- Stop the meeting and confirm the UI shows completed status, final transcript, final summary, and action items.
- Reopen the app after an abnormal exit during recording and confirm the home page shows a recoverable meeting prompt.
- Click the recoverable meeting CTA on the home page and confirm the live workspace opens with the recovered meeting title instead of creating a new meeting id.
- Restart the meeting runtime for a session that already has local `mixed-uplink.wav` data beyond the upload checkpoint and confirm the pending interval is replayed before new capture packets are sent.
- Export Markdown from the detail page and confirm the exported content includes transcript, summary, decisions, risks, and action items.

## macOS manual verification checklist

- Start the desktop app on macOS and create a new meeting.
- Grant microphone permission when prompted.
- Confirm the meeting enters `connecting -> ready -> recording` and continues to update without freezing the UI.
- Speak into the default microphone and verify `mic-original.wav` and `mixed-uplink.wav` both grow for the active meeting.
- Optionally set `MEETING_MACOS_DEV_SYSTEM_AUDIO=mirror_microphone`, restart the app, and verify `system-original.wav` now grows alongside `mic-original.wav` so the mixed dual-source path can be tested on the current machine.
- Confirm `system-original.wav` stays empty in the current development mode instead of causing the session to block.
- Stop the meeting and verify the history/detail pages still show the persisted transcript, summary, and action items flow.

## Known limitations in the current milestone

- Windows capture runtime is wired in code, but this macOS development environment has not yet compiled or manually exercised the WASAPI path on a real Windows machine.
- macOS currently offers a development-only microphone mirror option for dual-source pipeline testing; native system-audio loopback capture is still a follow-up item.
- Backend STT / summary / action-item generation is still deterministic stub output for contract verification.
- Desktop MQTT uses a real broker client only when `MEETING_SERVER_MQTT_BROKER` is configured; otherwise it intentionally falls back to the in-process stub for local UI/runtime development.
- The real UDP socket tests are opt-in because some sandboxed environments reject local UDP bind / connect during the default `cargo test` run.
- UDP ingest currently has a packet-processing seam and tests, but not yet a long-running socket loop bound to a broker-driven session runtime.
