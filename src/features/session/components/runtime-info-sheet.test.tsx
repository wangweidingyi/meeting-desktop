import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";

import { RuntimeInfoSheet } from "@/features/session/components/runtime-info-sheet";
import {
  createInitialSessionViewState,
  useSessionViewStore,
} from "@/lib/state/session-view-store";

describe("RuntimeInfoSheet", () => {
  beforeEach(() => {
    useSessionViewStore.setState(createInitialSessionViewState());
  });

  it("opens a diagnostics sheet with session and audio uplink details", () => {
    useSessionViewStore.setState({
      activeMeetingId: "meeting-1",
      title: "客户复盘会",
      status: "recording",
      connectionState: "connected",
      startedAtLabel: "1777031671975",
      runtimeInfo: {
        audioTargetAddr: "127.0.0.1:6000",
        audioUplinkState: "streaming",
        lastUploadedMixedMs: 1777031710400,
        lastChunkSequence: 8,
        lastChunkSentAt: "2026-04-23T15:00:12Z",
        replayFromMs: null,
        replayUntilMs: null,
        lastTransportError: null,
        mqttBrokerUrl: "tcp://127.0.0.1:1883",
        controlClientId: "meeting-desktop",
        adminApiBaseUrl: "http://127.0.0.1:8090",
        sttProvider: "volcengine_streaming",
        sttModel: "bigmodel",
        sttResourceId: "volc.seedasr.sauc.duration",
      },
    });

    render(<RuntimeInfoSheet />);

    fireEvent.click(screen.getByRole("button", { name: "运行信息" }));

    expect(screen.getByText("运行信息面板")).toBeInTheDocument();
    expect(screen.getByText("客户复盘会")).toBeInTheDocument();
    expect(screen.getByText("127.0.0.1:6000")).toBeInTheDocument();
    expect(screen.getByText("tcp://127.0.0.1:1883")).toBeInTheDocument();
    expect(screen.getByText("meeting-desktop")).toBeInTheDocument();
    expect(screen.getByText("volcengine_streaming")).toBeInTheDocument();
    expect(screen.getByText("bigmodel")).toBeInTheDocument();
    expect(screen.getAllByText("实时上行中").length).toBeGreaterThan(0);
    expect(screen.getByText("38.4 秒")).toBeInTheDocument();
    expect(screen.getByText("控制链路稳定")).toBeInTheDocument();
    expect(screen.queryByText("1777031671975")).not.toBeInTheDocument();
    expect(screen.getByText(/2026\/4\/24/)).toBeInTheDocument();
  });

  it("shows a hard warning when mqtt broker is missing", () => {
    useSessionViewStore.setState({
      activeMeetingId: "meeting-2",
      title: "异常排查会",
      status: "recording",
      connectionState: "disconnected",
      startedAtLabel: "2026-04-24 20:00",
      runtimeInfo: {
        audioTargetAddr: "127.0.0.1:6000",
        audioUplinkState: "streaming",
        lastUploadedMixedMs: 2000,
        lastChunkSequence: 3,
        lastChunkSentAt: "2026-04-24T20:00:02Z",
        replayFromMs: null,
        replayUntilMs: null,
        lastTransportError: null,
        mqttBrokerUrl: null,
        controlClientId: "meeting-desktop",
        adminApiBaseUrl: "http://127.0.0.1:8090",
        sttProvider: "volcengine_streaming",
        sttModel: "bigmodel",
        sttResourceId: "volc.seedasr.sauc.duration",
      },
    });

    render(<RuntimeInfoSheet />);

    fireEvent.click(screen.getByRole("button", { name: "运行信息" }));

    expect(screen.getByText("未配置，控制消息不会发送")).toBeInTheDocument();
  });
});
