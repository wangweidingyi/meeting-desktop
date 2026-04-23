export type MeetingStatus =
  | "idle"
  | "connecting"
  | "ready"
  | "recording"
  | "paused"
  | "stopping"
  | "completed"
  | "error";

export type MeetingListItem = {
  id: string;
  title: string;
  startedAt: string;
  endedAt: string | null;
  durationLabel: string;
  status: MeetingStatus;
  transcriptPreview: string;
};

import type { DesktopMeetingRecord } from "@/features/session/models";
import type { SummaryViewState } from "@/features/summary/models";
import type { TranscriptSegmentView } from "@/features/transcript/models";

export type MeetingDetailView = {
  meeting: DesktopMeetingRecord;
  transcriptSegments: TranscriptSegmentView[];
  summary: SummaryViewState;
  actionItems: string[];
};
