import { useCallback, useEffect } from "react";

import {
  createMeeting,
  pauseActiveMeeting,
  resumeActiveMeeting,
  startActiveMeeting,
  stopActiveMeeting,
} from "@/lib/api/commands";
import {
  type DesktopActionItemsDeltaPayload,
  type DesktopSessionSnapshotPayload,
  type DesktopSummaryDeltaPayload,
  type DesktopTransportStatePayload,
  listenActionItemsDelta,
  listenSessionUpdated,
  listenSummaryDelta,
  listenTransportError,
  listenTransportState,
  listenTranscriptDelta,
  type DesktopTranscriptDeltaPayload,
} from "@/lib/events/desktop-events";
import { useSessionViewStore } from "@/lib/state/session-view-store";
import type { SummaryViewState } from "@/features/summary/models";

function mapTranscriptDelta(payload: DesktopTranscriptDeltaPayload) {
  return {
    id: payload.segment_id,
    startMs: payload.start_ms,
    endMs: payload.end_ms,
    text: payload.text,
    isFinal: payload.is_final,
    speakerId: payload.speaker_id,
    revision: payload.revision,
  };
}

function syncSessionSnapshot(payload: DesktopSessionSnapshotPayload) {
  if (!payload.meeting) {
    return;
  }

  useSessionViewStore.getState().syncFromMeetingRecord(payload.meeting);
}

function mapSummaryDelta(payload: DesktopSummaryDeltaPayload): SummaryViewState {
  return {
    version: payload.version,
    isFinal: payload.is_final,
    abstract: payload.abstract_text,
    keyPoints: { title: "关键要点", items: payload.key_points },
    decisions: { title: "决策", items: payload.decisions },
    risks: { title: "风险", items: payload.risks },
    actionItems: { title: "行动项", items: payload.action_items },
    lastUpdatedLabel: payload.updated_at || "刚刚更新",
  };
}

function applyActionItemsDelta(payload: DesktopActionItemsDeltaPayload) {
  useSessionViewStore
    .getState()
    .applyActionItems(payload.version, payload.items, payload.is_final, payload.updated_at || "刚刚更新");
}

function applyTransportState(payload: DesktopTransportStatePayload) {
  useSessionViewStore.getState().setConnectionState(payload.state);
}

export function useLiveSession() {
  const session = useSessionViewStore();

  useEffect(() => {
    let disposed = false;
    let unsubscribeSessionUpdated = () => {};
    let unsubscribeTranscriptDelta = () => {};
    let unsubscribeSummaryDelta = () => {};
    let unsubscribeActionItemsDelta = () => {};
    let unsubscribeTransportState = () => {};
    let unsubscribeTransportError = () => {};

    void listenSessionUpdated(syncSessionSnapshot).then((unsubscribe) => {
      if (disposed) {
        unsubscribe();
        return;
      }
      unsubscribeSessionUpdated = unsubscribe;
    });

    void listenTranscriptDelta((payload) => {
      useSessionViewStore.getState().applyTranscriptSegment(mapTranscriptDelta(payload));
    }).then((unsubscribe) => {
      if (disposed) {
        unsubscribe();
        return;
      }
      unsubscribeTranscriptDelta = unsubscribe;
    });

    void listenSummaryDelta((payload) => {
      useSessionViewStore.getState().applySummarySnapshot(mapSummaryDelta(payload));
    }).then((unsubscribe) => {
      if (disposed) {
        unsubscribe();
        return;
      }
      unsubscribeSummaryDelta = unsubscribe;
    });

    void listenActionItemsDelta((payload) => {
      applyActionItemsDelta(payload);
    }).then((unsubscribe) => {
      if (disposed) {
        unsubscribe();
        return;
      }
      unsubscribeActionItemsDelta = unsubscribe;
    });

    void listenTransportState((payload) => {
      applyTransportState(payload);
    }).then((unsubscribe) => {
      if (disposed) {
        unsubscribe();
        return;
      }
      unsubscribeTransportState = unsubscribe;
    });

    void listenTransportError(() => {
      useSessionViewStore.getState().setConnectionState("reconnecting");
    }).then((unsubscribe) => {
      if (disposed) {
        unsubscribe();
        return;
      }
      unsubscribeTransportError = unsubscribe;
    });

    return () => {
      disposed = true;
      unsubscribeSessionUpdated();
      unsubscribeTranscriptDelta();
      unsubscribeSummaryDelta();
      unsubscribeActionItemsDelta();
      unsubscribeTransportState();
      unsubscribeTransportError();
    };
  }, []);

  const startMeeting = useCallback(async (title: string) => {
    const created = await createMeeting(title);
    useSessionViewStore.getState().hydrateMeetingShell(created.id, created.title, created.started_at);

    const started = await startActiveMeeting();
    useSessionViewStore.getState().syncFromMeetingRecord(started);
  }, []);

  const pauseMeeting = useCallback(async () => {
    const meeting = await pauseActiveMeeting();
    useSessionViewStore.getState().syncFromMeetingRecord(meeting);
  }, []);

  const resumeMeeting = useCallback(async () => {
    const meeting = await resumeActiveMeeting();
    useSessionViewStore.getState().syncFromMeetingRecord(meeting);
  }, []);

  const stopMeeting = useCallback(async () => {
    const meeting = await stopActiveMeeting();
    useSessionViewStore.getState().syncFromMeetingRecord(meeting);
  }, []);

  return {
    session,
    startMeeting,
    pauseMeeting,
    resumeMeeting,
    stopMeeting,
  };
}
