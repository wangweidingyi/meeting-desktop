import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { HomePage } from "@/routes/home-page";
import { createInitialSessionViewState, useSessionViewStore } from "@/lib/state/session-view-store";

const commandMocks = vi.hoisted(() => ({
  listRecoverableMeetingsMock: vi.fn(),
  listMeetingHistoryMock: vi.fn(),
  resumeRecoverableMeetingMock: vi.fn(),
  getMeetingDetailMock: vi.fn(),
}));

vi.mock("@/lib/api/commands", () => ({
  listRecoverableMeetings: commandMocks.listRecoverableMeetingsMock,
  listMeetingHistory: commandMocks.listMeetingHistoryMock,
  resumeRecoverableMeeting: commandMocks.resumeRecoverableMeetingMock,
  getMeetingDetail: commandMocks.getMeetingDetailMock,
}));

describe("HomePage", () => {
  beforeEach(() => {
    useSessionViewStore.setState(createInitialSessionViewState());
    commandMocks.listRecoverableMeetingsMock.mockReset();
    commandMocks.listMeetingHistoryMock.mockReset();
    commandMocks.resumeRecoverableMeetingMock.mockReset();
    commandMocks.getMeetingDetailMock.mockReset();
  });

  it("shows recoverable meeting prompt and resumes the unfinished meeting", async () => {
    commandMocks.listRecoverableMeetingsMock.mockResolvedValue([
      {
        id: "meeting-1",
        title: "客户复盘会",
        status: "recording",
        started_at: "2026-04-20 16:00",
        ended_at: null,
        duration_ms: 0,
      },
    ]);
    commandMocks.listMeetingHistoryMock.mockResolvedValue([
      {
        id: "meeting-1",
        title: "客户复盘会",
        startedAt: "2026-04-20 16:00",
        endedAt: null,
        durationLabel: "进行中",
        status: "recording",
        transcriptPreview: "正在持续接收转写和纪要增量。",
      },
    ]);
    commandMocks.resumeRecoverableMeetingMock.mockResolvedValue({
      id: "meeting-1",
      title: "客户复盘会",
      status: "recording",
      started_at: "2026-04-20 16:00",
      ended_at: null,
      duration_ms: 0,
    });
    commandMocks.getMeetingDetailMock.mockResolvedValue({
      meeting: {
        id: "meeting-1",
        title: "客户复盘会",
        status: "recording",
        started_at: "2026-04-20 16:00",
        ended_at: null,
        duration_ms: 0,
      },
      transcriptSegments: [
        {
          id: "segment-1",
          startMs: 0,
          endMs: 1000,
          text: "恢复后的转写内容",
          isFinal: true,
          speakerId: null,
          revision: 2,
        },
      ],
      summary: {
        version: 2,
        isFinal: false,
        abstract: "恢复后的纪要摘要",
        keyPoints: { title: "关键要点", items: ["恢复补传 mixed 音频"] },
        decisions: { title: "决策", items: [] },
        risks: { title: "风险", items: [] },
        actionItems: { title: "行动项", items: ["继续会议"] },
        lastUpdatedLabel: "2026-04-20T16:05:00Z",
      },
      actionItems: ["继续会议"],
    });

    render(
      <MemoryRouter>
        <HomePage />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByText(/检测到最近一次会议支持恢复/i)).toBeInTheDocument();
      expect(screen.getAllByText("客户复盘会").length).toBeGreaterThan(0);
      expect(screen.getByRole("button", { name: /继续未完成会议/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /继续未完成会议/i }));

    await waitFor(() => {
      expect(commandMocks.resumeRecoverableMeetingMock).toHaveBeenCalledWith("meeting-1");
      expect(commandMocks.getMeetingDetailMock).toHaveBeenCalledWith("meeting-1");
      expect(useSessionViewStore.getState().transcript[0].text).toBe("恢复后的转写内容");
      expect(useSessionViewStore.getState().summary.abstract).toBe("恢复后的纪要摘要");
    });
  });

  it("loads meeting history from desktop commands and supports local search filtering", async () => {
    commandMocks.listRecoverableMeetingsMock.mockResolvedValue([]);
    commandMocks.listMeetingHistoryMock.mockResolvedValue([
      {
        id: "meeting-1",
        title: "预算评审会",
        startedAt: "2026-04-20 16:00",
        endedAt: null,
        durationLabel: "进行中",
        status: "recording",
        transcriptPreview: "讨论预算和上线窗口。",
      },
      {
        id: "meeting-2",
        title: "法务对齐会",
        startedAt: "2026-04-19 14:00",
        endedAt: "2026-04-19 15:06",
        durationLabel: "66 分钟",
        status: "completed",
        transcriptPreview: "确认合同风险和行动项。",
      },
    ]);

    render(
      <MemoryRouter>
        <HomePage />
      </MemoryRouter>,
    );

    expect(commandMocks.listMeetingHistoryMock).toHaveBeenCalledTimes(1);
    expect(await screen.findByText("预算评审会")).toBeInTheDocument();
    expect(screen.getByText("法务对齐会")).toBeInTheDocument();

    const searchInput = screen.getByPlaceholderText("搜索会议标题或转写内容");
    fireEvent.change(searchInput, { target: { value: "法务" } });

    expect(screen.queryByText("预算评审会")).not.toBeInTheDocument();
    expect(screen.getByText("法务对齐会")).toBeInTheDocument();
  });
});
