import { Pause, Play, Square, Wifi, WifiOff } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { useLiveSession } from "@/features/session/hooks/use-live-session";

const transcriptPreview = [
  "主持人：今天先确认桌面端音频链路，双路采集都要稳定。",
  "产品：纪要里要把决策、风险、行动项拆开，不要只是摘要。",
  "研发：首版先走 mixed 单流上传，原始双路 WAV 本地保留。",
];

const summaryColumns = [
  { title: "摘要", content: "当前会议围绕桌面录音、实时转写和恢复链路推进 MVP 设计。" },
  { title: "关键要点", content: "Rust 主控；MQTT 做控制；UDP 负责 mixed 音频；Windows 首发。" },
  { title: "行动项", content: "先落 Phase 1 的目录、状态机、SQLite schema 和页面骨架。" },
];

export function LiveMeetingPage() {
  const { session, pauseMeeting, resumeMeeting, startMeeting, stopMeeting } = useLiveSession();

  return (
    <div className="space-y-6">
      <section className="rounded-[1.75rem] border border-black/5 bg-white/85 p-6 shadow-sm shadow-black/5">
        <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
          <div className="space-y-3">
            <div className="flex items-center gap-3">
              <Badge variant="secondary" className="rounded-full px-3 py-1 text-xs">
                会中工作台
              </Badge>
              <Badge variant={session.connectionState === "connected" ? "default" : "outline"} className="gap-1">
                {session.connectionState === "connected" ? (
                  <Wifi className="size-3.5" />
                ) : (
                  <WifiOff className="size-3.5" />
                )}
                {session.connectionState === "connected" ? "控制链路稳定" : "等待连接"}
              </Badge>
            </div>
            <div>
              <h1 className="text-3xl font-semibold tracking-tight text-slate-900">客户复盘会</h1>
              <p className="mt-2 text-sm text-slate-500">2026-04-21 16:00 开始 · 当前会持续显示转写和纪要增量</p>
            </div>
          </div>

          <div className="grid min-w-64 gap-3 rounded-[1.5rem] border border-black/5 bg-slate-950 px-5 py-4 text-white">
            <div className="text-xs uppercase tracking-[0.22em] text-slate-400">Current Status</div>
            <div className="text-3xl font-semibold">00:18:42</div>
            <div className="flex flex-wrap gap-2 text-xs">
              <Badge variant="outline" className="border-white/20 bg-white/5 text-white">
                {session.status === "recording" ? "录音中" : "待开始"}
              </Badge>
              <Badge variant="outline" className="border-white/20 bg-white/5 text-white">
                {session.flags.isTranscribing ? "转写中" : "转写待命"}
              </Badge>
              <Badge variant="outline" className="border-white/20 bg-white/5 text-white">
                {session.flags.isSummarizing ? "纪要生成中" : "纪要待命"}
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
            {transcriptPreview.map((line) => (
              <div key={line} className="rounded-2xl border border-black/5 bg-slate-50/80 px-4 py-3 text-sm leading-7 text-slate-700">
                {line}
              </div>
            ))}
          </CardContent>
        </Card>

        <Card className="border border-black/5 bg-white/85">
          <CardHeader>
            <CardTitle>实时会议纪要</CardTitle>
            <CardDescription>会中纪要和会后 final 纪要会按版本合并。</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {summaryColumns.map((section) => (
              <div key={section.title} className="space-y-2 rounded-2xl border border-black/5 bg-slate-50/80 p-4">
                <div className="text-sm font-semibold text-slate-900">{section.title}</div>
                <p className="text-sm leading-7 text-slate-600">{section.content}</p>
              </div>
            ))}
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
    </div>
  );
}
