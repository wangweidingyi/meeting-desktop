import type { TranscriptSegmentView } from "@/features/transcript/models";
import type { SummaryViewState } from "@/features/summary/models";

export type SessionConnectionState = "disconnected" | "connecting" | "connected" | "reconnecting";

export type AudioUplinkState =
  | "idle"
  | "waiting_for_audio"
  | "replaying"
  | "streaming"
  | "paused"
  | "stopped";

export type SessionViewStatus =
  | "idle"
  | "connecting"
  | "ready"
  | "recording"
  | "paused"
  | "stopping"
  | "completed"
  | "error";

export type SessionViewState = {
  activeMeetingId: string | null;
  title: string;
  status: SessionViewStatus;
  connectionState: SessionConnectionState;
  startedAtLabel: string | null;
  elapsedLabel: string;
  transcript: TranscriptSegmentView[];
  summary: SummaryViewState;
  flags: {
    isTranscribing: boolean;
    isSummarizing: boolean;
    isFlushing: boolean;
  };
  runtimeInfo: {
    audioTargetAddr: string | null;
    audioUplinkState: AudioUplinkState;
    lastUploadedMixedMs: number;
    lastChunkSequence: number | null;
    lastChunkSentAt: string | null;
    replayFromMs: number | null;
    replayUntilMs: number | null;
    lastTransportError: string | null;
    mqttBrokerUrl: string | null;
    controlClientId: string | null;
    adminApiBaseUrl: string | null;
    sttProvider: string | null;
    sttModel: string | null;
    sttResourceId: string | null;
  };
};

export type DesktopMeetingRecord = {
  id: string;
  title: string;
  status: SessionViewStatus;
  started_at: string;
  ended_at: string | null;
  duration_ms: number;
};
