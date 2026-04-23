import { createInitialSessionViewState } from "@/lib/state/session-view-store";
import { useSessionViewStore } from "@/lib/state/session-view-store";

describe("createInitialSessionViewState", () => {
  it("starts in idle status with empty live content", () => {
    const state = createInitialSessionViewState();

    expect(state.status).toBe("idle");
    expect(state.connectionState).toBe("disconnected");
    expect(state.transcript).toEqual([]);
    expect(state.summary.abstract).toContain("会议开始后");
  });

  it("keeps only the latest transcript revision for the same segment", () => {
    useSessionViewStore.setState(createInitialSessionViewState());

    useSessionViewStore.getState().applyTranscriptSegment({
      id: "segment-1",
      startMs: 0,
      endMs: 1200,
      text: "先记录增量版本",
      isFinal: false,
      speakerId: null,
      revision: 1,
    });
    useSessionViewStore.getState().applyTranscriptSegment({
      id: "segment-1",
      startMs: 0,
      endMs: 1400,
      text: "这是最终版本",
      isFinal: true,
      speakerId: null,
      revision: 2,
    });

    const state = useSessionViewStore.getState();

    expect(state.transcript).toHaveLength(1);
    expect(state.transcript[0].text).toBe("这是最终版本");
    expect(state.transcript[0].revision).toBe(2);
  });

  it("keeps only the latest summary version", () => {
    useSessionViewStore.setState(createInitialSessionViewState());

    useSessionViewStore.getState().applySummarySnapshot({
      version: 1,
      isFinal: false,
      abstract: "先保存增量摘要",
      keyPoints: { title: "关键要点", items: ["双路采集"] },
      decisions: { title: "决策", items: [] },
      risks: { title: "风险", items: [] },
      actionItems: { title: "行动项", items: ["继续联调"] },
      lastUpdatedLabel: "10:00:00",
    });
    useSessionViewStore.getState().applySummarySnapshot({
      version: 3,
      isFinal: true,
      abstract: "最终纪要完成",
      keyPoints: { title: "关键要点", items: ["Rust 主控"] },
      decisions: { title: "决策", items: ["首版采用 MQTT + UDP"] },
      risks: { title: "风险", items: [] },
      actionItems: { title: "行动项", items: ["推进联调"] },
      lastUpdatedLabel: "10:05:00",
    });

    const state = useSessionViewStore.getState();

    expect(state.summary.version).toBe(3);
    expect(state.summary.isFinal).toBe(true);
    expect(state.summary.abstract).toBe("最终纪要完成");
  });

  it("updates action items without dropping the existing summary body", () => {
    useSessionViewStore.setState(createInitialSessionViewState());

    useSessionViewStore.getState().applySummarySnapshot({
      version: 1,
      isFinal: false,
      abstract: "纪要主体已经生成",
      keyPoints: { title: "关键要点", items: ["保留 MQTT 控制链路"] },
      decisions: { title: "决策", items: ["继续推进联调"] },
      risks: { title: "风险", items: [] },
      actionItems: { title: "行动项", items: [] },
      lastUpdatedLabel: "10:00:00",
    });
    useSessionViewStore
      .getState()
      .applyActionItems(2, ["同步会议结论"], false, "10:01:00");

    const state = useSessionViewStore.getState();

    expect(state.summary.version).toBe(2);
    expect(state.summary.abstract).toBe("纪要主体已经生成");
    expect(state.summary.actionItems.items).toEqual(["同步会议结论"]);
  });

  it("allows transport state to move independently from meeting status", () => {
    useSessionViewStore.setState(createInitialSessionViewState());

    useSessionViewStore.getState().syncFromMeetingRecord({
      id: "meeting-1",
      title: "周会",
      status: "recording",
      started_at: "2026-04-22 10:00",
      ended_at: null,
      duration_ms: 0,
    });
    useSessionViewStore.getState().setConnectionState("reconnecting");

    const state = useSessionViewStore.getState();

    expect(state.status).toBe("recording");
    expect(state.connectionState).toBe("reconnecting");
  });

  it("keeps completed meetings disconnected even if a late transport error arrives", () => {
    useSessionViewStore.setState(createInitialSessionViewState());

    useSessionViewStore.getState().syncFromMeetingRecord({
      id: "meeting-1",
      title: "周会",
      status: "completed",
      started_at: "2026-04-22 10:00",
      ended_at: "2026-04-22 11:00",
      duration_ms: 3_600_000,
    });
    useSessionViewStore.getState().setConnectionState("reconnecting");

    const state = useSessionViewStore.getState();

    expect(state.status).toBe("completed");
    expect(state.connectionState).toBe("disconnected");
  });

  it("resets transcript and summary when switching to a different meeting id", () => {
    useSessionViewStore.setState(createInitialSessionViewState());

    useSessionViewStore.getState().syncFromMeetingRecord({
      id: "meeting-1",
      title: "周会",
      status: "recording",
      started_at: "2026-04-22 10:00",
      ended_at: null,
      duration_ms: 0,
    });
    useSessionViewStore.getState().applyTranscriptSegment({
      id: "segment-1",
      startMs: 0,
      endMs: 1200,
      text: "上一场会议的转写",
      isFinal: true,
      speakerId: null,
      revision: 1,
    });
    useSessionViewStore.getState().applySummarySnapshot({
      version: 2,
      isFinal: false,
      abstract: "上一场会议纪要",
      keyPoints: { title: "关键要点", items: ["上一场"] },
      decisions: { title: "决策", items: [] },
      risks: { title: "风险", items: [] },
      actionItems: { title: "行动项", items: [] },
      lastUpdatedLabel: "10:10:00",
    });

    useSessionViewStore.getState().syncFromMeetingRecord({
      id: "meeting-2",
      title: "恢复会议",
      status: "recording",
      started_at: "2026-04-22 11:00",
      ended_at: null,
      duration_ms: 0,
    });

    const state = useSessionViewStore.getState();

    expect(state.activeMeetingId).toBe("meeting-2");
    expect(state.transcript).toEqual([]);
    expect(state.summary.version).toBe(0);
    expect(state.summary.abstract).toContain("会议开始后");
  });

  it("hydrates recovered meeting detail into the live workspace", () => {
    useSessionViewStore.setState(createInitialSessionViewState());

    useSessionViewStore.getState().hydrateRecoveredMeetingDetail({
      meeting: {
        id: "meeting-recover",
        title: "恢复会议",
        status: "recording",
        started_at: "2026-04-22 11:00",
        ended_at: null,
        duration_ms: 0,
      },
      transcriptSegments: [
        {
          id: "segment-1",
          startMs: 0,
          endMs: 1200,
          text: "恢复后的首段转写",
          isFinal: true,
          speakerId: null,
          revision: 3,
        },
      ],
      summary: {
        version: 4,
        isFinal: false,
        abstract: "恢复后的纪要摘要",
        keyPoints: { title: "关键要点", items: ["恢复 mixed 音频补传"] },
        decisions: { title: "决策", items: ["继续当前会议"] },
        risks: { title: "风险", items: [] },
        actionItems: { title: "行动项", items: ["继续记录待办"] },
        lastUpdatedLabel: "2026-04-22T11:05:00Z",
      },
      actionItems: ["继续记录待办"],
    });

    const state = useSessionViewStore.getState();

    expect(state.activeMeetingId).toBe("meeting-recover");
    expect(state.title).toBe("恢复会议");
    expect(state.transcript).toHaveLength(1);
    expect(state.transcript[0].text).toBe("恢复后的首段转写");
    expect(state.summary.abstract).toBe("恢复后的纪要摘要");
    expect(state.summary.actionItems.items).toEqual(["继续记录待办"]);
  });
});
