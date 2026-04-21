import { create } from "zustand";

import type {
  DesktopMeetingRecord,
  SessionViewState,
  SessionViewStatus,
} from "@/features/session/models";

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

type SessionViewStore = SessionViewState & {
  setStatus: (status: SessionViewStatus) => void;
  hydrateMeetingShell: (meetingId: string, title: string, startedAtLabel: string) => void;
  syncFromMeetingRecord: (meeting: DesktopMeetingRecord) => void;
};

export const useSessionViewStore = create<SessionViewStore>((set) => ({
  ...createInitialSessionViewState(),
  setStatus: (status) => {
    set({ status });
  },
  hydrateMeetingShell: (meetingId, title, startedAtLabel) => {
    set({
      activeMeetingId: meetingId,
      title,
      startedAtLabel,
      connectionState: "connecting",
      status: "connecting",
    });
  },
  syncFromMeetingRecord: (meeting) => {
    set({
      activeMeetingId: meeting.id,
      title: meeting.title,
      startedAtLabel: meeting.started_at,
      status: meeting.status,
      connectionState:
        meeting.status === "recording" || meeting.status === "paused" || meeting.status === "ready"
          ? "connected"
          : meeting.status === "connecting"
            ? "connecting"
            : "disconnected",
      flags: {
        isTranscribing: meeting.status === "recording",
        isSummarizing: meeting.status === "recording",
        isFlushing: meeting.status === "stopping",
      },
    });
  },
}));
