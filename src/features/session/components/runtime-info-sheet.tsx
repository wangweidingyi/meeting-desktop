import type { ReactNode } from "react";
import { Cpu, Info, Radio, Signal, Workflow, X } from "lucide-react";
import { useState } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import type { AudioUplinkState, SessionConnectionState } from "@/features/session/models";
import { cn } from "@/lib/utils";
import { useSessionViewStore } from "@/lib/state/session-view-store";

const audioStateLabelMap: Record<AudioUplinkState, string> = {
  idle: "待开始",
  waiting_for_audio: "等待首包音频",
  replaying: "补传中",
  streaming: "实时上行中",
  paused: "已暂停",
  stopped: "已停止",
};

const connectionStateLabelMap: Record<SessionConnectionState, string> = {
  disconnected: "控制链路已断开",
  connecting: "控制链路连接中",
  connected: "控制链路稳定",
  reconnecting: "控制链路重连中",
};

function formatAudioDuration(durationMs: number) {
  if (durationMs <= 0) {
    return "0 秒";
  }

  return `${(durationMs / 1000).toFixed(1)} 秒`;
}

function normalizeTimelineMs(value: number, startedAt: string | null) {
  if (value <= 0) {
    return 0;
  }

  if (startedAt && /^\d+$/.test(startedAt)) {
    const startedAtMs = Number(startedAt);
    if (!Number.isNaN(startedAtMs) && value >= startedAtMs) {
      return value - startedAtMs;
    }
  }

  return value;
}

function formatTimestamp(value: string | null) {
  if (!value) {
    return "尚未发送";
  }

  if (/^\d+$/.test(value)) {
    const timestamp = Number(value);
    if (!Number.isNaN(timestamp)) {
      return new Date(timestamp).toLocaleString("zh-CN", {
        hour12: false,
      });
    }
  }

  return value;
}

function formatBrokerValue(value: string | null) {
  if (value && value.trim().length > 0) {
    return value;
  }

  return "未配置，控制消息不会发送";
}

function ValueCard({
  label,
  value,
  hint,
}: {
  label: string;
  value: string;
  hint?: string;
}) {
  return (
    <div className="rounded-2xl border border-black/5 bg-white px-4 py-4 shadow-sm shadow-black/5">
      <p className="text-xs uppercase tracking-[0.18em] text-slate-400">{label}</p>
      <p className="mt-2 text-lg font-semibold text-slate-950">{value}</p>
      {hint ? <p className="mt-2 text-xs leading-5 text-slate-500">{hint}</p> : null}
    </div>
  );
}

function Section({
  icon,
  title,
  description,
  children,
}: {
  icon: ReactNode;
  title: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <section className="space-y-3 rounded-[1.6rem] border border-black/5 bg-[#f4f7fb] p-4">
      <div className="flex items-start gap-3">
        <div className="rounded-2xl bg-white p-2 text-slate-700 shadow-sm shadow-black/5">{icon}</div>
        <div className="min-w-0">
          <h3 className="text-sm font-semibold text-slate-950">{title}</h3>
          <p className="mt-1 text-sm leading-6 text-slate-600">{description}</p>
        </div>
      </div>
      {children}
    </section>
  );
}

export function RuntimeInfoSheet() {
  const [open, setOpen] = useState(false);
  const session = useSessionViewStore();
  const { runtimeInfo } = session;
  const uploadedDurationMs = normalizeTimelineMs(runtimeInfo.lastUploadedMixedMs, session.startedAtLabel);
  const replayFromMs =
    runtimeInfo.replayFromMs === null
      ? null
      : normalizeTimelineMs(runtimeInfo.replayFromMs, session.startedAtLabel);
  const replayUntilMs =
    runtimeInfo.replayUntilMs === null
      ? null
      : normalizeTimelineMs(runtimeInfo.replayUntilMs, session.startedAtLabel);

  return (
    <>
      <div className="pointer-events-none fixed bottom-5 right-5 z-40">
        <Button
          className="pointer-events-auto h-11 rounded-full px-4 shadow-lg shadow-slate-900/15"
          onClick={() => setOpen(true)}
          type="button"
        >
          <Info className="size-4" />
          运行信息
        </Button>
      </div>

      <Sheet open={open} onOpenChange={setOpen}>
        <SheetContent>
          <SheetHeader className="relative pr-16">
            <Button
              aria-label="关闭运行信息面板"
              className="absolute right-4 top-4"
              onClick={() => setOpen(false)}
              size="icon-sm"
              type="button"
              variant="ghost"
            >
              <X className="size-4" />
            </Button>
            <div className="flex items-center gap-3">
              <Badge variant="secondary" className="rounded-full px-3 py-1 text-xs">
                Runtime
              </Badge>
              <Badge
                className={cn(
                  "rounded-full px-3 py-1 text-xs",
                  runtimeInfo.audioUplinkState === "streaming"
                    ? "bg-emerald-100 text-emerald-700"
                    : runtimeInfo.audioUplinkState === "replaying"
                      ? "bg-amber-100 text-amber-700"
                      : "bg-slate-100 text-slate-700",
                )}
                variant="outline"
              >
                {audioStateLabelMap[runtimeInfo.audioUplinkState]}
              </Badge>
            </div>
            <SheetTitle>运行信息面板</SheetTitle>
            <SheetDescription>
              这里集中展示当前会议的运行状态，方便直接确认控制链路、音频上行和目标地址是否正常。
            </SheetDescription>
          </SheetHeader>

          <div className="flex-1 space-y-4 overflow-y-auto px-5 py-5">
            <Section
              description="确认音频是否真的在发，以及当前已经发到了什么位置。"
              icon={<Radio className="size-4" />}
              title="音频上行"
            >
              <div className="grid gap-3 sm:grid-cols-2">
                <ValueCard label="当前状态" value={audioStateLabelMap[runtimeInfo.audioUplinkState]} />
                <ValueCard
                  hint="按当前会议开始时间换算后的已上行位置，用来确认音频实际已经送到哪里。"
                  label="已上行到"
                  value={formatAudioDuration(uploadedDurationMs)}
                />
                <ValueCard
                  hint="如果一直是空，说明还没有真正送出音频分片。"
                  label="最近分片序号"
                  value={runtimeInfo.lastChunkSequence === null ? "尚未发送" : `#${runtimeInfo.lastChunkSequence}`}
                />
                <ValueCard
                  label="最近发送时间"
                  value={formatTimestamp(runtimeInfo.lastChunkSentAt)}
                />
              </div>
              {replayFromMs !== null && replayUntilMs !== null ? (
                <div className="rounded-2xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm leading-6 text-amber-800">
                  正在处理补传窗口：{formatAudioDuration(replayFromMs)} 到 {formatAudioDuration(replayUntilMs)}
                </div>
              ) : null}
            </Section>

            <Section
              description="控制链路先稳定，实时转写和纪要事件才能持续回流到页面。"
              icon={<Signal className="size-4" />}
              title="控制链路"
            >
              <div className="grid gap-3 sm:grid-cols-2">
                <ValueCard label="连接状态" value={connectionStateLabelMap[session.connectionState]} />
                <ValueCard
                  hint="后续这里可以继续加入 broker、重连次数、最近 ack 等信息。"
                  label="最近异常"
                  value={runtimeInfo.lastTransportError ?? "无"}
                />
              </div>
            </Section>

            <Section
              description="这里直接展示桌面当前连接的后端配置摘要，方便你确认现在到底在用哪个 broker、哪个客户端身份，以及后台当前生效的 STT 配置。"
              icon={<Cpu className="size-4" />}
              title="后端配置"
            >
              <div className="grid gap-3 sm:grid-cols-2">
                <ValueCard label="桌面 Client ID" value={runtimeInfo.controlClientId ?? "未配置"} />
                <ValueCard
                  hint="如果这里未配置，桌面端无法把 hello/start 等控制消息真正发给服务端。"
                  label="MQTT Broker"
                  value={formatBrokerValue(runtimeInfo.mqttBrokerUrl)}
                />
                <ValueCard
                  hint="运行信息优先展示后台当前配置，若后台不可达则回退到桌面启动时读取的配置。"
                  label="STT Provider"
                  value={runtimeInfo.sttProvider ?? "未知"}
                />
                <ValueCard label="STT Model" value={runtimeInfo.sttModel ?? "未知"} />
                <ValueCard
                  label="STT Resource"
                  value={runtimeInfo.sttResourceId ?? "未提供"}
                />
                <ValueCard
                  hint="这里是桌面直接访问后台管理接口时使用的地址。"
                  label="管理后台地址"
                  value={runtimeInfo.adminApiBaseUrl ?? "未配置"}
                />
              </div>
            </Section>

            <Section
              description="会话和目标地址集中放在这里，方便排查到底连的是哪一场会议、发到了哪里。"
              icon={<Workflow className="size-4" />}
              title="会话信息"
            >
              <div className="grid gap-3 sm:grid-cols-2">
                <ValueCard label="会议标题" value={session.title} />
                <ValueCard label="会议 ID" value={session.activeMeetingId ?? "尚未创建"} />
                <ValueCard label="开始时间" value={formatTimestamp(session.startedAtLabel)} />
                <ValueCard
                  hint="当前音频分片会发送到这个后端目标。"
                  label="音频目标"
                  value={runtimeInfo.audioTargetAddr ?? "尚未准备"}
                />
              </div>
            </Section>
          </div>
        </SheetContent>
      </Sheet>
    </>
  );
}
