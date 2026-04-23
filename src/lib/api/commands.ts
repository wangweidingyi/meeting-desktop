import { invoke } from "@tauri-apps/api/core";
import type { DesktopMeetingRecord } from "@/features/session/models";
import type { MeetingDetailView, MeetingListItem } from "@/features/meetings/models";
import type { SummaryViewState } from "@/features/summary/models";
import type { TranscriptSegmentView } from "@/features/transcript/models";

type RawTranscriptSegment = {
  segment_id: string;
  start_ms: number;
  end_ms: number;
  text: string;
  is_final: boolean;
  speaker_id: string | null;
  revision: number;
};

type RawSummarySnapshot = {
  version: number;
  updated_at: string;
  abstract_text: string;
  key_points: string[];
  decisions: string[];
  risks: string[];
  action_items: string[];
  is_final: boolean;
} | null;

type RawMeetingDetail = {
  meeting: DesktopMeetingRecord;
  transcript_segments: RawTranscriptSegment[];
  summary: RawSummarySnapshot;
  action_items: string[];
};

function formatDuration(durationMs: number) {
  if (durationMs <= 0) {
    return "刚开始";
  }

  const minutes = Math.round(durationMs / 60_000);
  return `${minutes} 分钟`;
}

function mapTranscriptSegment(segment: RawTranscriptSegment): TranscriptSegmentView {
  return {
    id: segment.segment_id,
    startMs: segment.start_ms,
    endMs: segment.end_ms,
    text: segment.text,
    isFinal: segment.is_final,
    speakerId: segment.speaker_id,
    revision: segment.revision,
  };
}

function mapSummary(summary: RawSummarySnapshot): SummaryViewState {
  if (!summary) {
    return {
      version: 0,
      isFinal: false,
      abstract: "尚未生成最终纪要",
      keyPoints: { title: "关键要点", items: [] },
      decisions: { title: "决策", items: [] },
      risks: { title: "风险", items: [] },
      actionItems: { title: "行动项", items: [] },
      lastUpdatedLabel: "尚未生成",
    };
  }

  return {
    version: summary.version,
    isFinal: summary.is_final,
    abstract: summary.abstract_text,
    keyPoints: { title: "关键要点", items: summary.key_points },
    decisions: { title: "决策", items: summary.decisions },
    risks: { title: "风险", items: summary.risks },
    actionItems: { title: "行动项", items: summary.action_items },
    lastUpdatedLabel: summary.updated_at,
  };
}

export async function createMeeting(title: string) {
  return invoke<DesktopMeetingRecord>("create_meeting", { title });
}

export async function listRecoverableMeetings() {
  return invoke<DesktopMeetingRecord[]>("list_recoverable_meetings");
}

export async function startActiveMeeting() {
  return invoke<DesktopMeetingRecord>("start_active_meeting");
}

export async function resumeRecoverableMeeting(meetingId: string) {
  return invoke<DesktopMeetingRecord>("resume_recoverable_meeting", { meetingId });
}

export async function pauseActiveMeeting() {
  return invoke<DesktopMeetingRecord>("pause_active_meeting");
}

export async function resumeActiveMeeting() {
  return invoke<DesktopMeetingRecord>("resume_active_meeting");
}

export async function stopActiveMeeting() {
  return invoke<DesktopMeetingRecord>("stop_active_meeting");
}

export async function listMeetingHistory() {
  const meetings = await invoke<DesktopMeetingRecord[]>("list_meeting_history");

  return meetings.map<MeetingListItem>((meeting) => ({
    id: meeting.id,
    title: meeting.title,
    startedAt: meeting.started_at,
    endedAt: meeting.ended_at,
    durationLabel: formatDuration(meeting.duration_ms),
    status: meeting.status,
    transcriptPreview: "会后详情页将展示完整转写与结构化纪要。",
  }));
}

export async function exportMarkdown(meetingId: string) {
  return invoke<string>("export_markdown", { meetingId });
}

export async function getMeetingDetail(meetingId: string): Promise<MeetingDetailView> {
  const detail = await invoke<RawMeetingDetail>("get_meeting_detail", { meetingId });

  return {
    meeting: detail.meeting,
    transcriptSegments: detail.transcript_segments.map(mapTranscriptSegment),
    summary: mapSummary(detail.summary),
    actionItems: detail.action_items,
  };
}
