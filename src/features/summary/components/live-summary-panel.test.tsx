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
  it("shows a dedicated waiting state before the first summary version arrives", () => {
    render(
      <LiveSummaryPanel
        summary={makeSummary({
          version: 0,
          abstract: "会议开始后，这里会持续刷新摘要。",
          keyPoints: { title: "关键要点", items: [] },
          decisions: { title: "决策", items: [] },
          risks: { title: "风险", items: [] },
          actionItems: { title: "行动项", items: [] },
          lastUpdatedLabel: "尚未生成",
        })}
      />,
    );

    expect(screen.getByText("正在根据最新转写生成第一版纪要")).toBeInTheDocument();
    expect(screen.getByText("实时转写已经先行显示，结构化纪要会在首个版本完成后出现。")).toBeInTheDocument();
    expect(screen.queryByText("暂无内容")).not.toBeInTheDocument();
  });

  it("explains that live summary refreshes asynchronously from the latest transcript", () => {
    render(<LiveSummaryPanel summary={makeSummary()} />);

    expect(
      screen.getByText("转写先实时显示，纪要会基于最新转写异步刷新。"),
    ).toBeInTheDocument();
  });

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
    expect(screen.getByText("已基于最终转写完成整理。")).toBeInTheDocument();
  });
});
