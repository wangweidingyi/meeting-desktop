import { render, screen } from "@testing-library/react";

import type { TranscriptSegmentView } from "@/features/transcript/models";
import { TranscriptStreamPanel } from "@/features/transcript/components/transcript-stream-panel";

function makeSegment(overrides: Partial<TranscriptSegmentView> = {}): TranscriptSegmentView {
  return {
    id: "segment-1",
    startMs: 0,
    endMs: 1200,
    text: "先记录增量版本",
    isFinal: false,
    speakerId: null,
    revision: 1,
    ...overrides,
  };
}

describe("TranscriptStreamPanel", () => {
  it("shows live transcript snapshots as covered progress instead of a raw range", () => {
    render(
      <TranscriptStreamPanel
        segments={[
          makeSegment({
            id: "meeting-1-transcript",
            startMs: 0,
            endMs: 61_200,
            revision: 3,
          }),
        ]}
      />,
    );

    expect(screen.getByText("已转写至 01:01")).toBeInTheDocument();
    expect(screen.queryByText("00:00 - 01:01")).not.toBeInTheDocument();
  });

  it("shows discrete transcript segments as formatted time ranges", () => {
    render(
      <TranscriptStreamPanel
        segments={[
          makeSegment({
            id: "segment-2",
            startMs: 5_000,
            endMs: 8_400,
          }),
        ]}
      />,
    );

    expect(screen.getByText("00:05 - 00:08")).toBeInTheDocument();
  });

  it("renders the latest segment revision in place", () => {
    const { rerender } = render(
      <TranscriptStreamPanel segments={[makeSegment()]} />,
    );

    expect(screen.getByText("先记录增量版本")).toBeInTheDocument();

    rerender(
      <TranscriptStreamPanel
        segments={[
          makeSegment({
            text: "这是最终版本",
            isFinal: true,
            revision: 2,
            endMs: 1400,
          }),
        ]}
      />,
    );

    expect(screen.queryByText("先记录增量版本")).not.toBeInTheDocument();
    expect(screen.getByText("这是最终版本")).toBeInTheDocument();
    expect(screen.getByText("Final")).toBeInTheDocument();
  });
});
