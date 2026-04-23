# meeting-desktop

Tauri v2 desktop client for the meeting assistant. Rust owns session lifecycle, persistence, transport coordination, and recovery seams; React only renders the meeting workspace UI.

## Current milestone

- Meeting home, live session, and meeting detail pages are in place.
- SQLite persists meeting history, transcript segments, summary snapshots, action items, and recovery checkpoints.
- The desktop runtime now prepares a per-session transport bundle:
  - real MQTT control runtime when a broker is configured
  - in-memory MQTT-compatible stub when no broker is configured
  - real UDP mixed-audio socket target
- Markdown export and recoverable meeting discovery are available.

## What is real vs. staged today

- Real today:
  - local SQLite persistence
  - Rust session state transitions
  - broker-backed MQTT control transport
  - transport-state and transport-error desktop events for connect / reconnect / disconnect UI feedback
  - meeting-scoped audio runtime that prepares WAV asset paths and upload checkpoints inside Rust
  - unified PCM sample-ingress path that accepts microphone/system frames, writes source WAV files, mixes to mono, and feeds UDP uplink checkpoints
  - startup-time replay of pending `mixed-uplink.wav` ranges based on the persisted upload checkpoint so unuploaded audio can be resent before live capture continues
  - recoverable meetings can now be reopened from the home page, rebound into the active Rust session runtime with the original meeting id, and prefilled with persisted transcript/summary/action-item content before new realtime events arrive
  - Windows capture worker wiring for microphone + system loopback, including chunking, stop/join lifecycle, and PCM payload decoding into the shared Rust runtime
  - Windows capture sink adapters that normalize device frames into 16k mono PCM before handing them to the shared Rust runtime
  - macOS development capture path that records the real default microphone and forwards it through single-source mixed uplink mode for local testing
  - macOS can optionally mirror microphone frames into the persisted `system-original.wav` track and dual-source mixed uplink path when `MEETING_MACOS_DEV_SYSTEM_AUDIO=mirror_microphone` is enabled for local pipeline verification
  - UDP audio transport socket setup and packet encoding
  - runtime event processing into SQLite and Tauri frontend events
  - backend Go runtime with live UDP listener
- Still staged for the next desktop batch:
  - Windows-target compile and manual device verification on an actual Windows environment
  - real macOS system-audio loopback capture instead of the current development-only microphone mirror option
  - real STT / summary AI services

If `MEETING_SERVER_MQTT_BROKER` is set, the desktop control channel uses a real MQTT broker client. If it is empty, the desktop falls back to the in-process typed stub so the rest of the app can still run locally.

## Environment

Copy [`.env.example`](/Users/cxc/Documents/open/meeting/meeting-desktop/.env.example) to `.env`.

Supported desktop variables:

- `MEETING_DESKTOP_CLIENT_ID`
- `MEETING_SERVER_MQTT_BROKER`
- `MEETING_SERVER_MQTT_USERNAME`
- `MEETING_SERVER_MQTT_PASSWORD`
- `MEETING_SERVER_UDP_HOST`
- `MEETING_SERVER_UDP_PORT`
- `MEETING_MACOS_DEV_SYSTEM_AUDIO`

## Local development

Install frontend dependencies:

```bash
npm install
```

Start the backend from the desktop repo:

```bash
npm run backend:run
```

Start the desktop app with `.env` loaded into the Tauri runtime:

```bash
npm run dev:local
```

Run verification:

```bash
npm test
npm run build
npm run test:desktop-rust
npm run test:desktop-network
npm run test:desktop-smoke
```

`npm run test:desktop-network` is optional and reserved for environments that allow local UDP socket bind / connect operations.

## Cross-repo integration notes

- Backend reference implementation lives in [meeting-server](/Users/cxc/Documents/open/meeting/meeting-server).
- The current desktop runtime already targets the configured UDP host and port for mixed-audio uplink.
- The desktop MQTT runtime already consumes the `.env` broker values documented above.
- `npm run dev:local` now also reads `meeting-server/.env` and derives desktop MQTT/UDP targets from the backend env when possible, which is helpful for the embedded-broker local stack.
- Backend startup and protocol notes are documented in [meeting-server/README.md](/Users/cxc/Documents/open/meeting/meeting-server/README.md).
