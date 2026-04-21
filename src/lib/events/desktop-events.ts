import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export const DESKTOP_EVENT_SESSION_UPDATED = "meeting://session-updated";
export const DESKTOP_EVENT_TRANSCRIPT_DELTA = "meeting://transcript-delta";

export function listenSessionUpdated<T>(handler: (payload: T) => void): Promise<UnlistenFn> {
  return listen<T>(DESKTOP_EVENT_SESSION_UPDATED, (event) => handler(event.payload));
}

