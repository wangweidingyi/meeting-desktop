import { beforeEach, describe, expect, it, vi } from "vitest";

import { syncMeetingToBackend } from "@/lib/api/commands";

const coreMocks = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: coreMocks.invokeMock,
}));

describe("syncMeetingToBackend", () => {
  beforeEach(() => {
    coreMocks.invokeMock.mockReset();
    vi.restoreAllMocks();
  });

  beforeEach(() => {
    window.localStorage.clear();
  });

  it("syncs meetings through the authenticated app route", async () => {
    coreMocks.invokeMock.mockResolvedValue({
      controlClientId: "meeting-desktop",
      mqttBrokerUrl: "tcp://127.0.0.1:1883",
      audioTargetAddr: "127.0.0.1:6000",
      adminApiBaseUrl: "http://127.0.0.1:8090",
      startupSttProvider: "volcengine_streaming",
      startupSttModel: "bigmodel",
      startupSttResourceId: "volc.seedasr.sauc.duration",
      currentUserId: "user-1",
      currentUserName: "张三",
    });
    window.localStorage.setItem(
      "meeting.desktop.auth",
      JSON.stringify({
        token: "desktop-session-token",
        user: {
          id: "user-1",
          username: "zhangsan",
          displayName: "张三",
          role: "member",
          status: "active",
        },
      }),
    );

    const fetchMock = vi.fn().mockResolvedValue(
      new Response(null, {
        status: 200,
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    await syncMeetingToBackend({
      id: "meeting-1",
      title: "客户复盘会",
      status: "recording",
      started_at: "2026-04-22T10:00:00Z",
      ended_at: null,
      duration_ms: 0,
    });

    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:8090/api/app/meetings/meeting-1",
      expect.objectContaining({
        method: "PUT",
        headers: {
          Authorization: "Bearer desktop-session-token",
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          id: "meeting-1",
          title: "客户复盘会",
          status: "recording",
          started_at: "2026-04-22T10:00:00Z",
          ended_at: null,
          duration_ms: 0,
          client_id: "meeting-desktop",
        }),
      }),
    );
  });

  it("skips backend sync when desktop login session is absent", async () => {
    coreMocks.invokeMock.mockResolvedValue({
      controlClientId: "meeting-desktop",
      mqttBrokerUrl: null,
      audioTargetAddr: "127.0.0.1:6000",
      adminApiBaseUrl: "http://127.0.0.1:8090",
      startupSttProvider: null,
      startupSttModel: null,
      startupSttResourceId: null,
      currentUserId: null,
      currentUserName: null,
    });

    const fetchMock = vi.fn().mockResolvedValue(
      new Response(null, {
        status: 200,
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    await syncMeetingToBackend({
      id: "meeting-2",
      title: "默认身份会议",
      status: "idle",
      started_at: "2026-04-22T11:00:00Z",
      ended_at: null,
      duration_ms: 0,
    });

    expect(fetchMock).not.toHaveBeenCalled();
  });
});
