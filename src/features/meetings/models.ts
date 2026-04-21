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

