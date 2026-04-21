import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useLiveSession } from "@/features/session/hooks/use-live-session";
import type { DesktopMeetingRecord } from "@/features/session/models";
import {
  createInitialSessionViewState,
  useSessionViewStore,
} from "@/lib/state/session-view-store";

const commandMocks = vi.hoisted(() => ({
  createMeetingMock: vi.fn(),
  startActiveMeetingMock: vi.fn(),
  pauseActiveMeetingMock: vi.fn(),
  resumeActiveMeetingMock: vi.fn(),
  stopActiveMeetingMock: vi.fn(),
}));

vi.mock("@/lib/api/commands", () => ({
  createMeeting: commandMocks.createMeetingMock,
  startActiveMeeting: commandMocks.startActiveMeetingMock,
  pauseActiveMeeting: commandMocks.pauseActiveMeetingMock,
  resumeActiveMeeting: commandMocks.resumeActiveMeetingMock,
  stopActiveMeeting: commandMocks.stopActiveMeetingMock,
}));

function makeMeetingRecord(status: DesktopMeetingRecord["status"]): DesktopMeetingRecord {
  return {
    id: "meeting-1",
    title: "设计评审会",
    status,
    started_at: "2026-04-21 10:00",
    ended_at: null,
    duration_ms: 0,
  };
}

describe("useLiveSession", () => {
  beforeEach(() => {
    useSessionViewStore.setState(createInitialSessionViewState());
    commandMocks.createMeetingMock.mockReset();
    commandMocks.startActiveMeetingMock.mockReset();
    commandMocks.pauseActiveMeetingMock.mockReset();
    commandMocks.resumeActiveMeetingMock.mockReset();
    commandMocks.stopActiveMeetingMock.mockReset();
  });

  it("hydrates the meeting shell and advances to recording when start succeeds", async () => {
    commandMocks.createMeetingMock.mockResolvedValue(makeMeetingRecord("idle"));
    commandMocks.startActiveMeetingMock.mockResolvedValue(makeMeetingRecord("recording"));

    const { result } = renderHook(() => useLiveSession());

    await act(async () => {
      await result.current.startMeeting("设计评审会");
    });

    expect(result.current.session.activeMeetingId).toBe("meeting-1");
    expect(result.current.session.title).toBe("设计评审会");
    expect(result.current.session.status).toBe("recording");
    expect(result.current.session.connectionState).toBe("connected");
  });
});
