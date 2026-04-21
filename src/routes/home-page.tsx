import { Clock3, Plus, Search, TriangleAlert } from "lucide-react";
import { Link } from "react-router-dom";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { useMeetingStore } from "@/stores/meeting-store";

const statusLabelMap = {
  completed: "已完成",
  recording: "进行中",
  idle: "空闲",
  connecting: "连接中",
  ready: "就绪",
  paused: "已暂停",
  stopping: "停止中",
  error: "异常",
} as const;

export function HomePage() {
  const meetings = useMeetingStore((state) => state.meetings);
  const query = useMeetingStore((state) => state.query);
  const setQuery = useMeetingStore((state) => state.setQuery);

  const filteredMeetings = meetings.filter((meeting) => {
    const haystack = `${meeting.title} ${meeting.transcriptPreview}`.toLowerCase();
    return haystack.includes(query.trim().toLowerCase());
  });

  return (
    <div className="space-y-6">
      <section className="grid gap-6 rounded-[2rem] border border-black/5 bg-white/85 p-8 shadow-sm shadow-black/5 md:grid-cols-[1.35fr_0.85fr]">
        <div className="space-y-5">
          <Badge variant="secondary" className="rounded-full px-3 py-1 text-xs">
            Windows First · Rust Runtime
          </Badge>
          <div className="space-y-3">
            <h1 className="max-w-2xl text-4xl font-semibold tracking-tight text-slate-900 md:text-5xl">
              让会议从“录下来”变成“实时可追踪、会后可沉淀”。
            </h1>
            <p className="max-w-2xl text-base leading-7 text-slate-600">
              桌面端负责双路采集、MQTT 控制、UDP 音频上行、本地持久化和断线恢复；前端只展示状态、转写和纪要。
            </p>
          </div>
          <div className="flex flex-wrap gap-3">
            <Button asChild size="lg">
              <Link to="/meetings/live">
                <Plus className="size-4" />
                新建会议
              </Link>
            </Button>
            <Button asChild variant="outline" size="lg">
              <Link to="/meetings/2026-04-21-product-strategy">查看最近一次纪要</Link>
            </Button>
          </div>
        </div>

        <Card className="border border-amber-100 bg-amber-50/80">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <TriangleAlert className="size-4 text-amber-600" />
              恢复提醒
            </CardTitle>
            <CardDescription>如果应用异常关闭，下次启动会从本地 mixed 音频继续补传。</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4 text-sm text-slate-600">
            <div className="rounded-2xl bg-white/80 p-4 ring-1 ring-black/5">
              检测到最近一次会议支持恢复：
              <div className="mt-2 font-medium text-slate-900">客户复盘会</div>
              <div className="mt-1 flex items-center gap-2 text-xs text-slate-500">
                <Clock3 className="size-3.5" />
                已保存本地音频、转写片段和上传 checkpoint
              </div>
            </div>
            <Button asChild className="w-full">
              <Link to="/meetings/live">继续未完成会议</Link>
            </Button>
          </CardContent>
        </Card>
      </section>

      <section className="flex flex-col gap-4 rounded-[1.5rem] border border-black/5 bg-white/80 p-5 md:flex-row md:items-center md:justify-between">
        <div>
          <p className="text-sm uppercase tracking-[0.22em] text-slate-500">History</p>
          <h2 className="mt-2 text-2xl font-semibold tracking-tight text-slate-900">会议历史</h2>
        </div>
        <label className="flex w-full items-center gap-3 rounded-full border border-black/8 bg-white px-4 py-3 text-sm text-slate-500 md:max-w-sm">
          <Search className="size-4" />
          <input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="搜索会议标题或转写内容"
            className="w-full border-0 bg-transparent text-slate-900 outline-none placeholder:text-slate-400"
          />
        </label>
      </section>

      <section className="grid gap-4">
        {filteredMeetings.map((meeting) => (
          <Link key={meeting.id} to={`/meetings/${meeting.id}`} className="block">
            <Card className="border border-black/5 bg-white/80 transition-transform hover:-translate-y-0.5 hover:shadow-sm">
              <CardHeader>
                <div className="flex items-start justify-between gap-4">
                  <div className="space-y-2">
                    <CardTitle>{meeting.title}</CardTitle>
                    <CardDescription>
                      {meeting.startedAt}
                      {meeting.endedAt ? ` · ${meeting.endedAt}` : " · 未结束"}
                    </CardDescription>
                  </div>
                  <Badge variant={meeting.status === "recording" ? "default" : "outline"}>
                    {statusLabelMap[meeting.status]}
                  </Badge>
                </div>
              </CardHeader>
              <CardContent className="space-y-2">
                <div className="text-sm font-medium text-slate-900">{meeting.durationLabel}</div>
                <p className="text-sm leading-6 text-slate-600">{meeting.transcriptPreview}</p>
              </CardContent>
            </Card>
          </Link>
        ))}
      </section>
    </div>
  );
}
