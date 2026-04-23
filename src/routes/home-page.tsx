import type { ReactNode } from "react";
import {
  ArrowRight,
  Clock3,
  FileText,
  History,
  Mic,
  RotateCcw,
  Search,
  TriangleAlert,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { Link, useNavigate } from "react-router-dom";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import type { DesktopMeetingRecord } from "@/features/session/models";
import { useMeetingHistory } from "@/features/meetings/hooks/use-meeting-history";
import { getMeetingDetail, listRecoverableMeetings, resumeRecoverableMeeting } from "@/lib/api/commands";
import { cn } from "@/lib/utils";
import { useSessionViewStore } from "@/lib/state/session-view-store";

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
  const navigate = useNavigate();
  const { meetings, isLoading, error } = useMeetingHistory();
  const [query, setQuery] = useState("");
  const [recoverableMeeting, setRecoverableMeeting] = useState<DesktopMeetingRecord | null>(null);

  useEffect(() => {
    let disposed = false;

    void listRecoverableMeetings()
      .then((items) => {
        if (!disposed) {
          setRecoverableMeeting(items[0] ?? null);
        }
      })
      .catch(() => {
        if (!disposed) {
          setRecoverableMeeting(null);
        }
      });

    return () => {
      disposed = true;
    };
  }, []);

  const handleResumeRecoverableMeeting = async () => {
    if (!recoverableMeeting) {
      return;
    }

    const meeting = await resumeRecoverableMeeting(recoverableMeeting.id);
    const store = useSessionViewStore.getState();

    store.syncFromMeetingRecord(meeting);

    try {
      const detail = await getMeetingDetail(recoverableMeeting.id);
      useSessionViewStore.getState().hydrateRecoveredMeetingDetail(detail);
    } catch {
      // Keep the recovered runtime shell available so the live page can reconnect.
    }

    navigate("/meetings/live");
  };

  const normalizedQuery = query.trim().toLowerCase();
  const filteredMeetings = meetings.filter((meeting) => {
    const haystack = `${meeting.title} ${meeting.transcriptPreview}`.toLowerCase();
    return haystack.includes(normalizedQuery);
  });

  const todayKey = useMemo(() => new Intl.DateTimeFormat("sv-SE").format(new Date()), []);
  const todayMeetings = meetings.filter((meeting) => meeting.startedAt.startsWith(todayKey));
  const latestSummaryMeeting = meetings.find((meeting) => meeting.status === "completed") ?? meetings[0] ?? null;

  return (
    <div className="space-y-6">
      <section className="grid gap-6 xl:grid-cols-[420px_minmax(0,1fr)]">
        <Card className="border border-black/5 bg-[#f7f9fc] py-0 shadow-sm shadow-black/5">
          <CardContent className="space-y-6 p-6">
            <div className="space-y-3">
              <Badge variant="secondary" className="rounded-full bg-white px-3 py-1 text-xs text-slate-600">
                会议工作台
              </Badge>
              <div className="space-y-2">
                <h1 className="text-3xl font-semibold tracking-tight text-slate-950">开始今天的会议工作</h1>
                <p className="text-sm leading-6 text-slate-600">
                  Rust 负责采集、上传、落库和恢复，React 负责把实时转写和纪要稳定地展示出来。
                </p>
              </div>
            </div>

            <div className="grid grid-cols-2 gap-4">
              <QuickActionTile
                title="开始会议"
                description="立即进入会中工作台"
                icon={<Mic className="size-7" />}
                accent="primary"
                footer={
                  <Button asChild className="w-full justify-between">
                    <Link to="/meetings/live">
                      开始录音
                      <ArrowRight className="size-4" />
                    </Link>
                  </Button>
                }
              />

              <QuickActionTile
                title="继续上次会议"
                description={recoverableMeeting ? recoverableMeeting.title : "恢复未完成会议与补传进度"}
                icon={<RotateCcw className="size-7" />}
                accent={recoverableMeeting ? "primary" : "muted"}
                footer={
                  recoverableMeeting ? (
                    <Button className="w-full justify-between" onClick={() => void handleResumeRecoverableMeeting()}>
                      恢复会议
                      <ArrowRight className="size-4" />
                    </Button>
                  ) : (
                    <div className="text-xs text-slate-500">当前没有可恢复的会议</div>
                  )
                }
              />

              <QuickActionTile
                title="历史记录"
                description="查看全部会议与转写沉淀"
                icon={<History className="size-7" />}
                accent="primary"
                footer={
                  <a
                    href="#meeting-history"
                    className="inline-flex items-center gap-2 text-sm font-medium text-slate-700 transition-colors hover:text-slate-950"
                  >
                    查看全部
                    <ArrowRight className="size-4" />
                  </a>
                }
              />

              <QuickActionTile
                title="最近纪要"
                description={
                  latestSummaryMeeting
                    ? `最近完成：${latestSummaryMeeting.title}`
                    : "快速打开最近一次可查看的纪要"
                }
                icon={<FileText className="size-7" />}
                accent={latestSummaryMeeting ? "primary" : "muted"}
                footer={
                  latestSummaryMeeting ? (
                    <Button asChild variant="outline" className="w-full justify-between bg-white">
                      <Link to={`/meetings/${latestSummaryMeeting.id}`}>
                        打开纪要
                        <ArrowRight className="size-4" />
                      </Link>
                    </Button>
                  ) : (
                    <div className="text-xs text-slate-500">会议结束后会在这里展示最近纪要</div>
                  )
                }
              />
            </div>

            <div className="rounded-2xl border border-black/5 bg-white p-4">
              <div className="flex items-start gap-3">
                <div className="rounded-xl bg-amber-50 p-2 text-amber-600">
                  <TriangleAlert className="size-4" />
                </div>
                <div className="min-w-0 flex-1 space-y-1">
                  <p className="text-sm font-medium text-slate-900">异常退出恢复</p>
                  <p className="text-sm leading-6 text-slate-600">
                    {recoverableMeeting
                      ? `已检测到“${recoverableMeeting.title}”支持 mixed 音频补传和状态恢复。`
                      : "正在按 checkpoint 保存上传进度，下次异常退出后可继续恢复。"}
                  </p>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card className="border border-black/5 bg-white py-0 shadow-sm shadow-black/5">
          <CardContent className="flex h-full flex-col gap-6 p-6">
            <div className="flex flex-wrap items-start justify-between gap-4 border-b border-slate-100 pb-5">
              <div className="space-y-2">
                <p className="text-sm font-medium tracking-[0.18em] text-slate-400">TODAY</p>
                <h2 className="text-4xl font-semibold tracking-tight text-slate-950">今天</h2>
                <p className="flex items-center gap-2 text-sm text-slate-500">
                  <Clock3 className="size-4" />
                  本地历史、恢复状态与最近会议概览
                </p>
              </div>
              <Badge variant="outline" className="rounded-full px-3 py-1 text-xs text-slate-600">
                {todayMeetings.length} 场会议
              </Badge>
            </div>

            <div className="flex-1">
              {todayMeetings.length === 0 ? (
                <div className="flex h-full min-h-[280px] flex-col items-center justify-center rounded-[2rem] bg-slate-50/80 px-6 text-center">
                  <div className="mb-5 flex size-20 items-center justify-center rounded-full bg-white text-slate-300 shadow-sm shadow-slate-200/60">
                    <Clock3 className="size-9" />
                  </div>
                  <p className="text-3xl font-semibold tracking-tight text-slate-400">暂无会议</p>
                  <p className="mt-3 max-w-sm text-sm leading-6 text-slate-500">
                    暂时没有会议安排，开始一场新的会议吧。
                  </p>
                </div>
              ) : (
                <div className="space-y-3">
                  {todayMeetings.slice(0, 4).map((meeting) => (
                    <Link key={meeting.id} to={`/meetings/${meeting.id}`} className="block">
                      <div className="rounded-2xl border border-slate-100 bg-slate-50/80 p-4 transition-colors hover:border-slate-200 hover:bg-white">
                        <div className="flex items-start justify-between gap-4">
                          <div className="space-y-2">
                            <p className="text-base font-medium text-slate-950">{meeting.title}</p>
                            <p className="text-sm text-slate-500">
                              {meeting.startedAt}
                              {meeting.endedAt ? ` · ${meeting.endedAt}` : " · 进行中"}
                            </p>
                          </div>
                          <Badge variant={meeting.status === "recording" ? "default" : "outline"}>
                            {statusLabelMap[meeting.status]}
                          </Badge>
                        </div>
                        <p className="mt-3 text-sm leading-6 text-slate-600">{meeting.transcriptPreview}</p>
                      </div>
                    </Link>
                  ))}
                </div>
              )}
            </div>
          </CardContent>
        </Card>
      </section>

      <section
        id="meeting-history"
        className="space-y-4 rounded-[1.75rem] border border-black/5 bg-white/85 p-5 shadow-sm shadow-black/5"
      >
        <div className="flex flex-col gap-4 md:flex-row md:items-end md:justify-between">
          <div className="space-y-2">
            <p className="text-sm uppercase tracking-[0.22em] text-slate-400">Archive</p>
            <h2 className="text-2xl font-semibold tracking-tight text-slate-950">会议历史</h2>
            <p className="text-sm text-slate-500">按标题或转写内容搜索，继续查看完整转写、纪要和行动项。</p>
          </div>

          <label className="flex w-full items-center gap-3 rounded-full border border-black/8 bg-slate-50 px-4 py-3 text-sm text-slate-500 md:max-w-sm">
            <Search className="size-4" />
            <input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="搜索会议标题或转写内容"
              className="w-full border-0 bg-transparent text-slate-900 outline-none placeholder:text-slate-400"
            />
          </label>
        </div>

        <div className="grid gap-4">
          {isLoading ? (
            <Card className="border border-dashed border-black/10 bg-slate-50/80">
              <CardContent className="py-8 text-sm text-slate-500">
                正在从本地 SQLite 加载会议历史...
              </CardContent>
            </Card>
          ) : null}

          {!isLoading && error ? (
            <Card className="border border-rose-100 bg-rose-50/80">
              <CardContent className="py-8 text-sm text-rose-700">
                会议历史加载失败：{error}
              </CardContent>
            </Card>
          ) : null}

          {!isLoading && !error && filteredMeetings.length === 0 ? (
            <Card className="border border-dashed border-black/10 bg-slate-50/80">
              <CardContent className="py-8 text-sm text-slate-500">
                {query ? "没有匹配的会议记录。" : "还没有会议历史，开始第一场会议吧。"}
              </CardContent>
            </Card>
          ) : null}

          {!isLoading && !error
            ? filteredMeetings.map((meeting) => (
                <Link key={meeting.id} to={`/meetings/${meeting.id}`} className="block">
                  <Card className="border border-black/5 bg-white transition-transform hover:-translate-y-0.5 hover:shadow-sm">
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
              ))
            : null}
        </div>
      </section>
    </div>
  );
}

type QuickActionTileProps = {
  title: string;
  description: string;
  icon: ReactNode;
  footer: ReactNode;
  accent?: "primary" | "muted";
};

function QuickActionTile({ title, description, icon, footer, accent = "primary" }: QuickActionTileProps) {
  return (
    <div className="rounded-[1.5rem] border border-black/5 bg-white p-4 shadow-sm shadow-black/5">
      <div className="flex h-full flex-col gap-4">
        <div
          className={cn(
            "flex size-16 items-center justify-center rounded-[1.35rem] text-white shadow-sm",
            accent === "primary"
              ? "bg-[linear-gradient(135deg,#1d7df2_0%,#2d6cf3_100%)]"
              : "bg-[linear-gradient(135deg,#94a3b8_0%,#cbd5e1_100%)]",
          )}
        >
          {icon}
        </div>
        <div className="space-y-1">
          <h3 className="text-xl font-semibold tracking-tight text-slate-950">{title}</h3>
          <p className="min-h-12 text-sm leading-6 text-slate-500">{description}</p>
        </div>
        <div className="mt-auto">{footer}</div>
      </div>
    </div>
  );
}
