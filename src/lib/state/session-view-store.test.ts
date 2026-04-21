import { createInitialSessionViewState } from "@/lib/state/session-view-store";

describe("createInitialSessionViewState", () => {
  it("starts in idle status with empty live content", () => {
    const state = createInitialSessionViewState();

    expect(state.status).toBe("idle");
    expect(state.connectionState).toBe("disconnected");
    expect(state.transcript).toEqual([]);
    expect(state.summary.abstract).toContain("会议开始后");
  });
});

