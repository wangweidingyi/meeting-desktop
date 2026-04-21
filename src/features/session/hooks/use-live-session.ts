import { useCallback } from "react";

import {
  createMeeting,
  pauseActiveMeeting,
  resumeActiveMeeting,
  startActiveMeeting,
  stopActiveMeeting,
} from "@/lib/api/commands";
import { useSessionViewStore } from "@/lib/state/session-view-store";

export function useLiveSession() {
  const session = useSessionViewStore();

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

