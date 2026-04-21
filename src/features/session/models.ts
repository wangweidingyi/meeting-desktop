import type { TranscriptSegmentView } from "@/features/transcript/models";
import type { SummaryViewState } from "@/features/summary/models";

export type SessionConnectionState = "disconnected" | "connecting" | "connected" | "reconnecting";

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
};

export type DesktopMeetingRecord = {
  id: string;
  title: string;
  status: SessionViewStatus;
  started_at: string;
  ended_at: string | null;
  duration_ms: number;
};
