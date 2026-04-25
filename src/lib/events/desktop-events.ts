import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AudioUplinkState,
  DesktopMeetingRecord,
  SessionConnectionState,
  SessionViewStatus,
} from "@/features/session/models";

export const DESKTOP_EVENT_SESSION_UPDATED = "meeting://session-updated";
export const DESKTOP_EVENT_TRANSCRIPT_DELTA = "meeting://transcript-delta";
export const DESKTOP_EVENT_SUMMARY_DELTA = "meeting://summary-delta";
export const DESKTOP_EVENT_ACTION_ITEMS_DELTA = "meeting://action-items-delta";
export const DESKTOP_EVENT_TRANSPORT_STATE = "meeting://transport-state";
export const DESKTOP_EVENT_TRANSPORT_ERROR = "meeting://transport-error";
export const DESKTOP_EVENT_RUNTIME_DIAGNOSTICS = "meeting://runtime-diagnostics";

export type DesktopSessionSnapshotPayload = {
  meeting: DesktopMeetingRecord | null;
  status: SessionViewStatus;
};

export function listenSessionUpdated(
  handler: (payload: DesktopSessionSnapshotPayload) => void,
): Promise<UnlistenFn> {
  return listen<DesktopSessionSnapshotPayload>(DESKTOP_EVENT_SESSION_UPDATED, (event) =>
    handler(event.payload),
  );
}

export type DesktopTranscriptDeltaPayload = {
  session_id: string;
  segment_id: string;
  start_ms: number;
  end_ms: number;
  text: string;
  is_final: boolean;
  speaker_id: string | null;
  revision: number;
};

export function listenTranscriptDelta(
  handler: (payload: DesktopTranscriptDeltaPayload) => void,
): Promise<UnlistenFn> {
  return listen<DesktopTranscriptDeltaPayload>(DESKTOP_EVENT_TRANSCRIPT_DELTA, (event) =>
    handler(event.payload),
  );
}

export type DesktopSummaryDeltaPayload = {
  session_id: string;
  version: number;
  updated_at: string;
  abstract_text: string;
  key_points: string[];
  decisions: string[];
  risks: string[];
  action_items: string[];
  is_final: boolean;
};

export function listenSummaryDelta(
  handler: (payload: DesktopSummaryDeltaPayload) => void,
): Promise<UnlistenFn> {
  return listen<DesktopSummaryDeltaPayload>(DESKTOP_EVENT_SUMMARY_DELTA, (event) =>
    handler(event.payload),
  );
}

export type DesktopActionItemsDeltaPayload = {
  session_id: string;
  version: number;
  updated_at: string;
  items: string[];
  is_final: boolean;
};

export function listenActionItemsDelta(
  handler: (payload: DesktopActionItemsDeltaPayload) => void,
): Promise<UnlistenFn> {
  return listen<DesktopActionItemsDeltaPayload>(DESKTOP_EVENT_ACTION_ITEMS_DELTA, (event) =>
    handler(event.payload),
  );
}

export type DesktopTransportStatePayload = {
  session_id: string;
  state: SessionConnectionState;
  message: string | null;
};

export function listenTransportState(
  handler: (payload: DesktopTransportStatePayload) => void,
): Promise<UnlistenFn> {
  return listen<DesktopTransportStatePayload>(DESKTOP_EVENT_TRANSPORT_STATE, (event) =>
    handler(event.payload),
  );
}

export function listenTransportError(handler: (message: string) => void): Promise<UnlistenFn> {
  return listen<string>(DESKTOP_EVENT_TRANSPORT_ERROR, (event) => handler(event.payload));
}

export type DesktopRuntimeDiagnosticsPayload = {
  session_id: string;
  audio_target_addr: string;
  audio_uplink_state: AudioUplinkState;
  last_uploaded_mixed_ms: number;
  last_chunk_sequence: number | null;
  last_chunk_sent_at: string | null;
  replay_from_ms: number | null;
  replay_until_ms: number | null;
};

export function listenRuntimeDiagnostics(
  handler: (payload: DesktopRuntimeDiagnosticsPayload) => void,
): Promise<UnlistenFn> {
  return listen<DesktopRuntimeDiagnosticsPayload>(DESKTOP_EVENT_RUNTIME_DIAGNOSTICS, (event) =>
    handler(event.payload),
  );
}
