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

const desktopEventMocks = vi.hoisted(() => ({
  listenSessionUpdatedMock: vi.fn(),
  listenTranscriptDeltaMock: vi.fn(),
  listenSummaryDeltaMock: vi.fn(),
  listenActionItemsDeltaMock: vi.fn(),
  listenTransportStateMock: vi.fn(),
  listenTransportErrorMock: vi.fn(),
}));

vi.mock("@/lib/api/commands", () => ({
  createMeeting: commandMocks.createMeetingMock,
  startActiveMeeting: commandMocks.startActiveMeetingMock,
  pauseActiveMeeting: commandMocks.pauseActiveMeetingMock,
  resumeActiveMeeting: commandMocks.resumeActiveMeetingMock,
  stopActiveMeeting: commandMocks.stopActiveMeetingMock,
}));

vi.mock("@/lib/events/desktop-events", () => ({
  listenSessionUpdated: desktopEventMocks.listenSessionUpdatedMock,
  listenTranscriptDelta: desktopEventMocks.listenTranscriptDeltaMock,
  listenSummaryDelta: desktopEventMocks.listenSummaryDeltaMock,
  listenActionItemsDelta: desktopEventMocks.listenActionItemsDeltaMock,
  listenTransportState: desktopEventMocks.listenTransportStateMock,
  listenTransportError: desktopEventMocks.listenTransportErrorMock,
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
    desktopEventMocks.listenSessionUpdatedMock.mockReset();
    desktopEventMocks.listenTranscriptDeltaMock.mockReset();
    desktopEventMocks.listenSummaryDeltaMock.mockReset();
    desktopEventMocks.listenActionItemsDeltaMock.mockReset();
    desktopEventMocks.listenTransportStateMock.mockReset();
    desktopEventMocks.listenTransportErrorMock.mockReset();
    desktopEventMocks.listenSessionUpdatedMock.mockResolvedValue(() => {});
    desktopEventMocks.listenTranscriptDeltaMock.mockResolvedValue(() => {});
    desktopEventMocks.listenSummaryDeltaMock.mockResolvedValue(() => {});
    desktopEventMocks.listenActionItemsDeltaMock.mockResolvedValue(() => {});
    desktopEventMocks.listenTransportStateMock.mockResolvedValue(() => {});
    desktopEventMocks.listenTransportErrorMock.mockResolvedValue(() => {});
  });

  it("hydrates the meeting shell and advances to recording when start succeeds", async () => {
    let transportStateHandler: ((payload: Record<string, unknown>) => void) | undefined;
    desktopEventMocks.listenTransportStateMock.mockImplementation(async (handler) => {
      transportStateHandler = handler;
      return () => {};
    });
    commandMocks.createMeetingMock.mockResolvedValue(makeMeetingRecord("idle"));
    commandMocks.startActiveMeetingMock.mockResolvedValue(makeMeetingRecord("recording"));

    const { result } = renderHook(() => useLiveSession());

    await act(async () => {
      await result.current.startMeeting("设计评审会");
      transportStateHandler?.({
        session_id: "meeting-1",
        state: "connected",
        message: null,
      });
    });

    expect(result.current.session.activeMeetingId).toBe("meeting-1");
    expect(result.current.session.title).toBe("设计评审会");
    expect(result.current.session.status).toBe("recording");
    expect(result.current.session.connectionState).toBe("connected");
  });

  it("applies transcript deltas from desktop events into the view store", async () => {
    let transcriptHandler: ((payload: Record<string, unknown>) => void) | undefined;
    desktopEventMocks.listenTranscriptDeltaMock.mockImplementation(async (handler) => {
      transcriptHandler = handler;
      return () => {};
    });

    renderHook(() => useLiveSession());

    await act(async () => {
      transcriptHandler?.({
        session_id: "meeting-1",
        segment_id: "segment-1",
        start_ms: 0,
        end_ms: 1200,
        text: "这是最终版本",
        is_final: true,
        speaker_id: null,
        revision: 2,
      });
    });

    const state = useSessionViewStore.getState();
    expect(state.transcript).toHaveLength(1);
    expect(state.transcript[0].text).toBe("这是最终版本");
    expect(state.transcript[0].revision).toBe(2);
  });

  it("applies summary deltas from desktop events into the view store", async () => {
    let summaryHandler: ((payload: Record<string, unknown>) => void) | undefined;
    desktopEventMocks.listenSummaryDeltaMock.mockImplementation(async (handler) => {
      summaryHandler = handler;
      return () => {};
    });

    renderHook(() => useLiveSession());

    await act(async () => {
      summaryHandler?.({
        session_id: "meeting-1",
        version: 2,
        updated_at: "2026-04-22T10:00:00Z",
        abstract_text: "纪要持续生成中",
        key_points: ["Rust 主控"],
        decisions: ["首版使用 MQTT + UDP"],
        risks: ["仍需补恢复"],
        action_items: ["继续联调服务端"],
        is_final: true,
      });
    });

    const state = useSessionViewStore.getState();
    expect(state.summary.version).toBe(2);
    expect(state.summary.isFinal).toBe(true);
    expect(state.summary.decisions.items[0]).toBe("首版使用 MQTT + UDP");
  });

  it("applies action-item deltas without dropping the existing summary body", async () => {
    let summaryHandler: ((payload: Record<string, unknown>) => void) | undefined;
    let actionItemsHandler: ((payload: Record<string, unknown>) => void) | undefined;
    desktopEventMocks.listenSummaryDeltaMock.mockImplementation(async (handler) => {
      summaryHandler = handler;
      return () => {};
    });
    desktopEventMocks.listenActionItemsDeltaMock.mockImplementation(async (handler) => {
      actionItemsHandler = handler;
      return () => {};
    });

    renderHook(() => useLiveSession());

    await act(async () => {
      summaryHandler?.({
        session_id: "meeting-1",
        version: 2,
        updated_at: "2026-04-22T10:00:00Z",
        abstract_text: "纪要主体已生成",
        key_points: ["Rust 主控"],
        decisions: ["首版使用 MQTT + UDP"],
        risks: [],
        action_items: [],
        is_final: false,
      });
      actionItemsHandler?.({
        session_id: "meeting-1",
        version: 3,
        updated_at: "2026-04-22T10:01:00Z",
        items: ["继续联调 action_item_delta"],
        is_final: false,
      });
    });

    const state = useSessionViewStore.getState();
    expect(state.summary.abstract).toBe("纪要主体已生成");
    expect(state.summary.actionItems.items).toEqual(["继续联调 action_item_delta"]);
    expect(state.summary.version).toBe(3);
  });

  it("keeps recording status while transport switches into reconnecting", async () => {
    let transportStateHandler: ((payload: Record<string, unknown>) => void) | undefined;
    desktopEventMocks.listenTransportStateMock.mockImplementation(async (handler) => {
      transportStateHandler = handler;
      return () => {};
    });

    commandMocks.createMeetingMock.mockResolvedValue(makeMeetingRecord("idle"));
    commandMocks.startActiveMeetingMock.mockResolvedValue(makeMeetingRecord("recording"));

    const { result } = renderHook(() => useLiveSession());

    await act(async () => {
      await result.current.startMeeting("设计评审会");
      transportStateHandler?.({
        session_id: "meeting-1",
        state: "reconnecting",
        message: "connection reset",
      });
    });

    expect(result.current.session.status).toBe("recording");
    expect(result.current.session.connectionState).toBe("reconnecting");
  });
});
