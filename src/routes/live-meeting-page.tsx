import { Pause, Play, Square, Wifi, WifiOff } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { useLiveSession } from "@/features/session/hooks/use-live-session";
import { RuntimeInfoSheet } from "@/features/session/components/runtime-info-sheet";
import { LiveSummaryPanel } from "@/features/summary/components/live-summary-panel";
import { TranscriptStreamPanel } from "@/features/transcript/components/transcript-stream-panel";

export function LiveMeetingPage() {
  const { session, pauseMeeting, resumeMeeting, startMeeting, stopMeeting } = useLiveSession();
  const transcriptFlowLabel = session.flags.isTranscribing ? "转写实时回流中" : "转写待命中";
  const summaryFlowLabel = session.summary.isFinal
    ? "纪要已完成整理"
    : session.flags.isSummarizing
      ? "纪要异步整理中"
      : "纪要待命中";
  const sessionFlowHint = session.summary.isFinal
    ? "最终纪要已经和最终转写对齐。"
    : "转写会优先实时显示，纪要会基于最新转写异步补齐。";
  const connectionTone =
    session.connectionState === "connected"
      ? { label: "控制链路稳定", icon: Wifi, variant: "default" as const }
      : session.connectionState === "connecting"
        ? { label: "控制链路连接中", icon: WifiOff, variant: "outline" as const }
        : session.connectionState === "reconnecting"
          ? { label: "控制链路重连中", icon: WifiOff, variant: "outline" as const }
          : { label: "控制链路已断开", icon: WifiOff, variant: "outline" as const };
  const ConnectionIcon = connectionTone.icon;
  const title = session.activeMeetingId ? session.title : "客户复盘会";
  const startedAtLabel = session.startedAtLabel
    ? `${session.startedAtLabel} 开始`
    : "等待会议创建";
  const elapsedLabel = session.elapsedLabel || "00:00:00";

  return (
    <div className="space-y-6">
      <section className="rounded-[1.75rem] border border-black/5 bg-white/85 p-6 shadow-sm shadow-black/5">
        <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
          <div className="space-y-3">
            <div className="flex items-center gap-3">
              <Badge variant="secondary" className="rounded-full px-3 py-1 text-xs">
                会中工作台
              </Badge>
              <Badge variant={connectionTone.variant} className="gap-1">
                <ConnectionIcon className="size-3.5" />
                {connectionTone.label}
              </Badge>
            </div>
            <div>
              <h1 className="text-3xl font-semibold tracking-tight text-slate-900">{title}</h1>
              <p className="mt-2 text-sm text-slate-500">
                {startedAtLabel} · 当前会持续显示转写和纪要增量
              </p>
              <p className="mt-2 text-sm leading-6 text-slate-600">{sessionFlowHint}</p>
            </div>
          </div>

          <div className="grid min-w-64 gap-3 rounded-[1.5rem] border border-black/5 bg-slate-950 px-5 py-4 text-white">
            <div className="text-xs uppercase tracking-[0.22em] text-slate-400">Current Status</div>
            <div className="text-3xl font-semibold">{elapsedLabel}</div>
            <div className="flex flex-wrap gap-2 text-xs">
              <Badge variant="outline" className="border-white/20 bg-white/5 text-white">
                {session.status === "recording" ? "录音中" : "待开始"}
              </Badge>
              <Badge variant="outline" className="border-white/20 bg-white/5 text-white">
                {transcriptFlowLabel}
              </Badge>
              <Badge variant="outline" className="border-white/20 bg-white/5 text-white">
                {summaryFlowLabel}
              </Badge>
            </div>
          </div>
        </div>
      </section>

      <section className="grid gap-6 xl:grid-cols-[1.15fr_0.85fr]">
        <Card className="border border-black/5 bg-white/85">
          <CardHeader>
            <CardTitle>实时转写</CardTitle>
            <CardDescription>增量转写只刷新局部内容，避免整块闪动。</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <TranscriptStreamPanel segments={session.transcript} />
          </CardContent>
        </Card>

        <Card className="border border-black/5 bg-white/85">
          <CardHeader>
            <CardTitle>实时会议纪要</CardTitle>
            <CardDescription>转写先实时显示，纪要会基于最新转写异步刷新。</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <LiveSummaryPanel summary={session.summary} />
          </CardContent>
        </Card>
      </section>

      <section className="flex flex-wrap items-center gap-3 rounded-[1.5rem] border border-black/5 bg-white/85 p-4">
        <Button onClick={() => void startMeeting("客户复盘会")}>
          <Play className="size-4" />
          开始
        </Button>
        <Button variant="outline" onClick={() => void pauseMeeting()}>
          <Pause className="size-4" />
          暂停
        </Button>
        <Button variant="outline" onClick={() => void resumeMeeting()}>
          <Play className="size-4" />
          继续
        </Button>
        <Button variant="destructive" onClick={() => void stopMeeting()}>
          <Square className="size-4" />
          停止
        </Button>
      </section>

      <RuntimeInfoSheet />
    </div>
  );
}
