import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { createInitialSessionViewState } from "@/lib/state/session-view-store";
import { LiveMeetingPage } from "@/routes/live-meeting-page";

const hookMocks = vi.hoisted(() => ({
  useLiveSessionMock: vi.fn(),
}));

vi.mock("@/features/session/hooks/use-live-session", () => ({
  useLiveSession: hookMocks.useLiveSessionMock,
}));

vi.mock("@/features/session/components/runtime-info-sheet", () => ({
  RuntimeInfoSheet: () => <div>Runtime Sheet</div>,
}));

function makeSessionState() {
  return {
    ...createInitialSessionViewState(),
    activeMeetingId: "meeting-1",
    title: "客户复盘会",
    status: "recording" as const,
    connectionState: "connected" as const,
    startedAtLabel: "2026-04-28 10:00",
    transcript: [
      {
        id: "meeting-1-transcript",
        startMs: 0,
        endMs: 1200,
        text: "先确认预算和发布时间。",
        isFinal: false,
        speakerId: null,
        revision: 2,
      },
    ],
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
      isTranscribing: true,
      isSummarizing: true,
      isFlushing: false,
    },
  };
}

describe("LiveMeetingPage", () => {
  beforeEach(() => {
    hookMocks.useLiveSessionMock.mockReset();
    hookMocks.useLiveSessionMock.mockReturnValue({
      session: makeSessionState(),
      pauseMeeting: vi.fn(),
      resumeMeeting: vi.fn(),
      startMeeting: vi.fn(),
      stopMeeting: vi.fn(),
    });
  });

  it("surfaces that transcript is realtime while summary refreshes asynchronously", () => {
    render(<LiveMeetingPage />);

    expect(screen.getByText("转写实时回流中")).toBeInTheDocument();
    expect(screen.getByText("纪要异步整理中")).toBeInTheDocument();
    expect(
      screen.getByText("转写会优先实时显示，纪要会基于最新转写异步补齐。"),
    ).toBeInTheDocument();
    expect(
      screen.getAllByText("转写先实时显示，纪要会基于最新转写异步刷新。"),
    ).toHaveLength(2);
  });
});
