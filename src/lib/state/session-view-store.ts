import { create } from "zustand";

import type { MeetingDetailView } from "@/features/meetings/models";
import type {
  DesktopMeetingRecord,
  SessionConnectionState,
  SessionViewState,
  SessionViewStatus,
} from "@/features/session/models";
import type { TranscriptSegmentView } from "@/features/transcript/models";
import type { SummaryViewState } from "@/features/summary/models";

export function createInitialSessionViewState(): SessionViewState {
  return {
    activeMeetingId: null,
    title: "未开始会议",
    status: "idle",
    connectionState: "disconnected",
    startedAtLabel: null,
    elapsedLabel: "00:00:00",
    transcript: [],
    summary: {
      version: 0,
      isFinal: false,
      abstract: "会议开始后，这里会持续刷新摘要。",
      keyPoints: { title: "关键要点", items: [] },
      decisions: { title: "决策", items: [] },
      risks: { title: "风险", items: [] },
      actionItems: { title: "行动项", items: [] },
      lastUpdatedLabel: "尚未生成",
    },
    flags: {
      isTranscribing: false,
      isSummarizing: false,
      isFlushing: false,
    },
  };
}

function buildEmptyLiveContent() {
  const state = createInitialSessionViewState();
  return {
    transcript: state.transcript,
    summary: state.summary,
    elapsedLabel: state.elapsedLabel,
    flags: state.flags,
  };
}

function deriveConnectionState(
  meeting: DesktopMeetingRecord,
  previousConnectionState: SessionConnectionState,
): SessionConnectionState {
  if (meeting.status === "idle" || meeting.status === "completed") {
    return "disconnected";
  }

  if (meeting.status === "connecting") {
    return "connecting";
  }

  if (meeting.status === "error") {
    return previousConnectionState === "disconnected" ? "reconnecting" : previousConnectionState;
  }

  return previousConnectionState === "disconnected" ? "connected" : previousConnectionState;
}

function deriveFlags(status: SessionViewStatus, summary: SummaryViewState) {
  return {
    isTranscribing: status === "recording",
    isSummarizing: status === "recording" && !summary.isFinal,
    isFlushing: status === "stopping",
  };
}

type SessionViewStore = SessionViewState & {
  setStatus: (status: SessionViewStatus) => void;
  setConnectionState: (connectionState: SessionConnectionState) => void;
  hydrateMeetingShell: (meetingId: string, title: string, startedAtLabel: string) => void;
  syncFromMeetingRecord: (meeting: DesktopMeetingRecord) => void;
  hydrateRecoveredMeetingDetail: (detail: MeetingDetailView) => void;
  applyTranscriptSegment: (segment: TranscriptSegmentView) => void;
  applySummarySnapshot: (summary: SummaryViewState) => void;
  applyActionItems: (version: number, items: string[], isFinal: boolean, updatedAtLabel: string) => void;
};

export const useSessionViewStore = create<SessionViewStore>((set) => ({
  ...createInitialSessionViewState(),
  setStatus: (status) => {
    set({ status });
  },
  setConnectionState: (connectionState) => {
    set((state) => ({
      connectionState:
        state.status === "idle" || state.status === "completed"
          ? "disconnected"
          : connectionState,
    }));
  },
  hydrateMeetingShell: (meetingId, title, startedAtLabel) => {
    set((state) => ({
      ...(state.activeMeetingId !== meetingId ? buildEmptyLiveContent() : {}),
      activeMeetingId: meetingId,
      title,
      startedAtLabel,
      connectionState: "connecting",
      status: "connecting",
    }));
  },
  syncFromMeetingRecord: (meeting) => {
    set((state) => ({
      ...(state.activeMeetingId !== meeting.id ? buildEmptyLiveContent() : {}),
      activeMeetingId: meeting.id,
      title: meeting.title,
      startedAtLabel: meeting.started_at,
      status: meeting.status,
      connectionState: deriveConnectionState(meeting, state.connectionState),
      flags: deriveFlags(meeting.status, state.summary),
    }));
  },
  hydrateRecoveredMeetingDetail: (detail) => {
    set((state) => {
      const nextSummary = {
        ...detail.summary,
        actionItems: {
          ...detail.summary.actionItems,
          items: detail.actionItems.length > 0 ? detail.actionItems : detail.summary.actionItems.items,
        },
      };
      const nextTranscript = [...detail.transcriptSegments].sort((left, right) => left.startMs - right.startMs);

      return {
        ...(state.activeMeetingId !== detail.meeting.id ? buildEmptyLiveContent() : {}),
        activeMeetingId: detail.meeting.id,
        title: detail.meeting.title,
        startedAtLabel: detail.meeting.started_at,
        status: detail.meeting.status,
        connectionState: deriveConnectionState(detail.meeting, state.connectionState),
        transcript: nextTranscript,
        summary: nextSummary,
        flags: deriveFlags(detail.meeting.status, nextSummary),
      };
    });
  },
  applyTranscriptSegment: (segment) => {
    set((state) => {
      const existingIndex = state.transcript.findIndex((item) => item.id === segment.id);
      const nextTranscript = [...state.transcript];

      if (existingIndex === -1) {
        nextTranscript.push(segment);
      } else if (segment.revision >= nextTranscript[existingIndex].revision) {
        nextTranscript[existingIndex] = segment;
      }

      nextTranscript.sort((left, right) => left.startMs - right.startMs);

      return {
        transcript: nextTranscript,
        flags: {
          ...state.flags,
          isTranscribing: true,
        },
      };
    });
  },
  applySummarySnapshot: (summary) => {
    set((state) => {
      if (summary.version < state.summary.version) {
        return state;
      }

      return {
        summary,
        flags: {
          ...state.flags,
          isSummarizing: !summary.isFinal,
        },
      };
    });
  },
  applyActionItems: (version, items, isFinal, updatedAtLabel) => {
    set((state) => {
      if (version < state.summary.version) {
        return state;
      }

      return {
        summary: {
          ...state.summary,
          version,
          isFinal,
          actionItems: {
            ...state.summary.actionItems,
            items,
          },
          lastUpdatedLabel: updatedAtLabel,
        },
        flags: {
          ...state.flags,
          isSummarizing: !isFinal,
        },
      };
    });
  },
}));
