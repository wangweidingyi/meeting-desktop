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
