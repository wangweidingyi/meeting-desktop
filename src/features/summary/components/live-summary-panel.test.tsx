import { render, screen } from "@testing-library/react";

import { LiveSummaryPanel } from "@/features/summary/components/live-summary-panel";
import type { SummaryViewState } from "@/features/summary/models";

function makeSummary(overrides: Partial<SummaryViewState> = {}): SummaryViewState {
  return {
    version: 1,
    isFinal: false,
    abstract: "先展示会中纪要摘要",
    keyPoints: { title: "关键要点", items: ["双路采集"] },
    decisions: { title: "决策", items: [] },
    risks: { title: "风险", items: ["网络抖动"] },
    actionItems: { title: "行动项", items: ["补齐恢复逻辑"] },
    lastUpdatedLabel: "10:00:00",
    ...overrides,
  };
}

describe("LiveSummaryPanel", () => {
  it("renders the latest summary version and final badge", () => {
    const { rerender } = render(<LiveSummaryPanel summary={makeSummary()} />);

    expect(screen.getByText("先展示会中纪要摘要")).toBeInTheDocument();

    rerender(
      <LiveSummaryPanel
        summary={makeSummary({
          version: 3,
          isFinal: true,
          abstract: "最终纪要已经完成",
          decisions: { title: "决策", items: ["保持 mixed 单流上传"] },
        })}
      />,
    );

    expect(screen.queryByText("先展示会中纪要摘要")).not.toBeInTheDocument();
    expect(screen.getByText("最终纪要已经完成")).toBeInTheDocument();
    expect(screen.getByText("Final v3")).toBeInTheDocument();
    expect(screen.getByText("保持 mixed 单流上传")).toBeInTheDocument();
  });
});
