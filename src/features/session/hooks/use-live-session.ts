import { useCallback, useEffect } from "react";

import {
  createMeeting,
  getRuntimeBackendInfo,
  pauseActiveMeeting,
  resumeActiveMeeting,
  startActiveMeeting,
  stopActiveMeeting,
  syncMeetingToBackend,
} from "@/lib/api/commands";
import {
  type DesktopActionItemsDeltaPayload,
  type DesktopRuntimeDiagnosticsPayload,
  type DesktopSessionSnapshotPayload,
  type DesktopSummaryDeltaPayload,
  type DesktopTransportStatePayload,
  listenActionItemsDelta,
  listenRuntimeDiagnostics,
  listenSessionUpdated,
  listenSummaryDelta,
  listenTransportError,
  listenTransportState,
  listenTranscriptDelta,
  type DesktopTranscriptDeltaPayload,
} from "@/lib/events/desktop-events";
import { useSessionViewStore } from "@/lib/state/session-view-store";
import type { SummaryViewState } from "@/features/summary/models";

type AdminSettingsSnapshot = {
  ai?: {
    stt?: {
      provider?: string;
      model?: string;
      options?: {
        resourceId?: string;
      };
    };
  };
};

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

function applyRuntimeDiagnostics(payload: DesktopRuntimeDiagnosticsPayload) {
  useSessionViewStore.getState().applyRuntimeDiagnostics({
    audioTargetAddr: payload.audio_target_addr,
    audioUplinkState: payload.audio_uplink_state,
    lastUploadedMixedMs: payload.last_uploaded_mixed_ms,
    lastChunkSequence: payload.last_chunk_sequence,
    lastChunkSentAt: payload.last_chunk_sent_at,
    replayFromMs: payload.replay_from_ms,
    replayUntilMs: payload.replay_until_ms,
  });
}

async function syncBackendRuntimeInfo() {
  const runtime = await getRuntimeBackendInfo();

  let sttProvider = runtime.startupSttProvider;
  let sttModel = runtime.startupSttModel;
  let sttResourceId = runtime.startupSttResourceId;

  if (runtime.adminApiBaseUrl) {
    try {
      const response = await fetch(`${runtime.adminApiBaseUrl}/api/admin/settings`);
      if (response.ok) {
        const settings = (await response.json()) as AdminSettingsSnapshot;
        sttProvider = settings.ai?.stt?.provider ?? sttProvider;
        sttModel = settings.ai?.stt?.model ?? sttModel;
        sttResourceId = settings.ai?.stt?.options?.resourceId ?? sttResourceId;
      }
    } catch {
      // Keep runtime fallback values when the admin API is temporarily unavailable.
    }
  }

  useSessionViewStore.getState().applyBackendRuntimeInfo({
    audioTargetAddr: runtime.audioTargetAddr,
    mqttBrokerUrl: runtime.mqttBrokerUrl,
    controlClientId: runtime.controlClientId,
    adminApiBaseUrl: runtime.adminApiBaseUrl,
    sttProvider,
    sttModel,
    sttResourceId,
  });
}

function surfaceRuntimeError(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  useSessionViewStore.getState().setConnectionState("disconnected");
  useSessionViewStore.getState().setLastTransportError(message);
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
    let unsubscribeRuntimeDiagnostics = () => {};

    void syncBackendRuntimeInfo();

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

    void listenTransportError((message) => {
      useSessionViewStore.getState().setConnectionState("reconnecting");
      useSessionViewStore.getState().setLastTransportError(message);
    }).then((unsubscribe) => {
      if (disposed) {
        unsubscribe();
        return;
      }
      unsubscribeTransportError = unsubscribe;
    });

    void listenRuntimeDiagnostics((payload) => {
      applyRuntimeDiagnostics(payload);
    }).then((unsubscribe) => {
      if (disposed) {
        unsubscribe();
        return;
      }
      unsubscribeRuntimeDiagnostics = unsubscribe;
    });

    return () => {
      disposed = true;
      unsubscribeSessionUpdated();
      unsubscribeTranscriptDelta();
      unsubscribeSummaryDelta();
      unsubscribeActionItemsDelta();
      unsubscribeTransportState();
      unsubscribeTransportError();
      unsubscribeRuntimeDiagnostics();
    };
  }, []);

  const startMeeting = useCallback(async (title: string) => {
    try {
      const created = await createMeeting(title);
      useSessionViewStore.getState().hydrateMeetingShell(created.id, created.title, created.started_at);
      await syncMeetingToBackend(created);

      const started = await startActiveMeeting();
      useSessionViewStore.getState().syncFromMeetingRecord(started);
      await syncMeetingToBackend(started);
    } catch (error) {
      surfaceRuntimeError(error);
      throw error;
    }
  }, []);

  const pauseMeeting = useCallback(async () => {
    try {
      const meeting = await pauseActiveMeeting();
      useSessionViewStore.getState().syncFromMeetingRecord(meeting);
      await syncMeetingToBackend(meeting);
    } catch (error) {
      surfaceRuntimeError(error);
      throw error;
    }
  }, []);

  const resumeMeeting = useCallback(async () => {
    try {
      const meeting = await resumeActiveMeeting();
      useSessionViewStore.getState().syncFromMeetingRecord(meeting);
      await syncMeetingToBackend(meeting);
    } catch (error) {
      surfaceRuntimeError(error);
      throw error;
    }
  }, []);

  const stopMeeting = useCallback(async () => {
    try {
      const meeting = await stopActiveMeeting();
      useSessionViewStore.getState().syncFromMeetingRecord(meeting);
      await syncMeetingToBackend(meeting);
    } catch (error) {
      surfaceRuntimeError(error);
      throw error;
    }
  }, []);

  return {
    session,
    startMeeting,
    pauseMeeting,
    resumeMeeting,
    stopMeeting,
  };
}
