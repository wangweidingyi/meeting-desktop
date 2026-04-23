import { render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { MeetingDetailPage } from "@/routes/meeting-detail-page";

const commandMocks = vi.hoisted(() => ({
  getMeetingDetailMock: vi.fn(),
  exportMarkdownMock: vi.fn(),
}));

vi.mock("@/lib/api/commands", () => ({
  getMeetingDetail: commandMocks.getMeetingDetailMock,
  exportMarkdown: commandMocks.exportMarkdownMock,
}));

describe("MeetingDetailPage", () => {
  beforeEach(() => {
    commandMocks.getMeetingDetailMock.mockReset();
    commandMocks.exportMarkdownMock.mockReset();
  });

  it("renders transcript, summary, and action items from loaded meeting detail", async () => {
    commandMocks.getMeetingDetailMock.mockResolvedValue({
      meeting: {
        id: "meeting-1",
        title: "产品策略例会",
        status: "completed",
        started_at: "2026-04-21 09:30",
        ended_at: "2026-04-21 10:18",
        duration_ms: 2880000,
      },
      transcriptSegments: [
        {
          id: "segment-1",
          startMs: 0,
          endMs: 1200,
          text: "主持人：先确认音频链路。",
          isFinal: true,
          speakerId: null,
          revision: 2,
        },
      ],
      summary: {
        version: 3,
        isFinal: true,
        abstract: "会议明确 Rust 主控与 mixed 单流方案。",
        keyPoints: { title: "关键要点", items: ["控制链路使用 MQTT"] },
        decisions: { title: "决策", items: ["音频链路使用 UDP"] },
        risks: { title: "风险", items: ["需要补断线恢复"] },
        actionItems: { title: "行动项", items: ["完善导出能力"] },
        lastUpdatedLabel: "2026-04-21 10:18",
      },
      actionItems: ["完善导出能力"],
    });

    render(
      <MemoryRouter initialEntries={["/meetings/meeting-1"]}>
        <Routes>
          <Route path="/meetings/:meetingId" element={<MeetingDetailPage />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(await screen.findByText("产品策略例会")).toBeInTheDocument();
    expect(screen.getByText("会议明确 Rust 主控与 mixed 单流方案。")).toBeInTheDocument();
    expect(screen.getByText("主持人：先确认音频链路。")).toBeInTheDocument();
    expect(screen.getAllByText("完善导出能力")).toHaveLength(2);
  });
});
