import { invoke } from "@tauri-apps/api/core";
import type { DesktopMeetingRecord } from "@/features/session/models";

export async function createMeeting(title: string) {
  return invoke<DesktopMeetingRecord>("create_meeting", { title });
}

export async function listRecoverableMeetings() {
  return invoke<DesktopMeetingRecord[]>("list_recoverable_meetings");
}

export async function startActiveMeeting() {
  return invoke<DesktopMeetingRecord>("start_active_meeting");
}

export async function pauseActiveMeeting() {
  return invoke<DesktopMeetingRecord>("pause_active_meeting");
}

export async function resumeActiveMeeting() {
  return invoke<DesktopMeetingRecord>("resume_active_meeting");
}

export async function stopActiveMeeting() {
  return invoke<DesktopMeetingRecord>("stop_active_meeting");
}

export async function listMeetingHistory() {
  return invoke<DesktopMeetingRecord[]>("list_meeting_history");
}

export async function exportMarkdown(meetingId: string) {
  return invoke<string>("export_markdown", { meetingId });
}
