import { invoke } from "@tauri-apps/api/core";
import type { DesktopMeetingRecord } from "@/features/session/models";
import type { MeetingDetailView, MeetingListItem } from "@/features/meetings/models";
import type { SummaryViewState } from "@/features/summary/models";
import type { TranscriptSegmentView } from "@/features/transcript/models";
import { getDesktopAuthSession } from "@/lib/auth";

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

type MeetingListResponse = {
  items: DesktopMeetingRecord[];
};

export type RuntimeBackendInfo = {
  controlClientId: string;
  currentUserId: string | null;
  currentUserName: string | null;
  mqttBrokerUrl: string | null;
  audioTargetAddr: string;
  adminApiBaseUrl: string | null;
  startupSttProvider: string | null;
  startupSttModel: string | null;
  startupSttResourceId: string | null;
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

async function getAuthenticatedRuntime() {
  const runtime = await getRuntimeBackendInfo();
  if (!runtime.adminApiBaseUrl) {
    throw new Error("未配置管理后台地址");
  }

  const session = getDesktopAuthSession();
  if (!session?.token) {
    throw new Error("当前未登录桌面端");
  }

  return {
    runtime,
    token: session.token,
  };
}

async function fetchAppJSON<T>(path: string, init?: RequestInit): Promise<T> {
  const { runtime, token } = await getAuthenticatedRuntime();
  const response = await fetch(`${runtime.adminApiBaseUrl}${path}`, {
    ...init,
    headers: {
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json",
      ...(init?.headers ?? {}),
    },
  });

  if (!response.ok) {
    const body = await response.text();
    throw new Error(body || `backend request failed with status ${response.status}`);
  }

  return (await response.json()) as T;
}

function exportMeetingMarkdown(detail: RawMeetingDetail) {
  const lines = [
    `# ${detail.meeting.title}`,
    "",
    `开始时间：${detail.meeting.started_at}`,
    `结束时间：${detail.meeting.ended_at ?? "未结束"}`,
    "",
  ];

  if (detail.summary) {
    lines.push("## 最终会议纪要", "");
    lines.push(detail.summary.abstract_text, "");
    lines.push("### 关键要点");
    lines.push(...detail.summary.key_points.map((item) => `- ${item}`));
    lines.push("", "### 决策");
    lines.push(...detail.summary.decisions.map((item) => `- ${item}`));
    lines.push("", "### 风险");
    lines.push(...detail.summary.risks.map((item) => `- ${item}`));
    lines.push("", "## 行动项");
    lines.push(...detail.summary.action_items.map((item) => `- ${item}`));
    lines.push("");
  }

  lines.push("## 完整逐段转写", "");
  lines.push(
    ...detail.transcript_segments.map((segment) => `- [${segment.start_ms} - ${segment.end_ms}] ${segment.text}`),
  );
  lines.push("");

  return lines.join("\n");
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

export async function getRuntimeBackendInfo() {
  return invoke<RuntimeBackendInfo>("get_runtime_backend_info");
}

export async function listRecoverableMeetings() {
  const response = await fetchAppJSON<MeetingListResponse>("/api/app/meetings/recoverable");
  return response.items;
}

export async function startActiveMeeting() {
  return invoke<DesktopMeetingRecord>("start_active_meeting");
}

export async function resumeRecoverableMeeting(meetingId: string) {
  const detail = await fetchAppJSON<RawMeetingDetail>(`/api/app/meetings/${meetingId}`);
  return invoke<DesktopMeetingRecord>("resume_recoverable_meeting", { meeting: detail.meeting });
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
  const response = await fetchAppJSON<MeetingListResponse>("/api/app/meetings");
  const meetings = response.items;

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
  const detail = await fetchAppJSON<RawMeetingDetail>(`/api/app/meetings/${meetingId}`);
  return exportMeetingMarkdown(detail);
}

export async function getMeetingDetail(meetingId: string): Promise<MeetingDetailView> {
  const detail = await fetchAppJSON<RawMeetingDetail>(`/api/app/meetings/${meetingId}`);

  return {
    meeting: detail.meeting,
    transcriptSegments: detail.transcript_segments.map(mapTranscriptSegment),
    summary: mapSummary(detail.summary),
    actionItems: detail.action_items,
  };
}
