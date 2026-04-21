import { ArrowRight, CalendarDays, Sparkles, TimerReset } from "lucide-react";
import { Link } from "react-router-dom";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { useMeetingStore } from "@/stores/meeting-store";

export function HomePage() {
  const focusMode = useMeetingStore((state) => state.focusMode);
  const toggleFocusMode = useMeetingStore((state) => state.toggleFocusMode);
  const nextMeeting = useMeetingStore((state) => state.meetings[0]);
  const totalMeetings = useMeetingStore((state) => state.meetings.length);

  return (
    <div className="space-y-6">
      <section className="grid gap-6 rounded-[2rem] border border-black/5 bg-white/80 p-8 shadow-sm shadow-black/5 backdrop-blur md:grid-cols-[1.4fr_0.9fr]">
        <div className="space-y-5">
          <Badge variant="secondary" className="gap-1 rounded-full px-3 py-1 text-xs">
            <Sparkles className="size-3.5" />
            Tauri v2 Desktop Starter
          </Badge>
          <div className="space-y-3">
            <h1 className="max-w-xl text-4xl font-semibold tracking-tight text-slate-900 md:text-5xl">
              Meeting 已经接好 React Router、Zustand 和 shadcn/ui。
            </h1>
            <p className="max-w-xl text-base leading-7 text-slate-600">
              这个首页是一个可直接扩展的桌面应用骨架。你现在已经可以在它上面继续加会议管理、纪要、录音转写或日程同步模块。
            </p>
          </div>
          <div className="flex flex-wrap gap-3">
            <Button asChild size="lg">
              <Link to="/agenda">
                查看今日议程
                <ArrowRight className="size-4" />
              </Link>
            </Button>
            <Button variant="outline" size="lg" onClick={toggleFocusMode}>
              <TimerReset className="size-4" />
              {focusMode ? "关闭专注模式" : "开启专注模式"}
            </Button>
          </div>
        </div>

        <Card className="border border-amber-100 bg-amber-50/80" size="sm">
          <CardHeader>
            <CardTitle>下一场会议</CardTitle>
            <CardDescription>这里的数据由 Zustand store 提供。</CardDescription>
            <CardAction>
              <Badge variant={focusMode ? "default" : "outline"}>
                {focusMode ? "专注中" : "标准模式"}
              </Badge>
            </CardAction>
          </CardHeader>
          <CardContent className="space-y-4">
            <div>
              <p className="text-3xl font-semibold text-slate-900">{nextMeeting?.time}</p>
              <p className="mt-2 text-lg font-medium text-slate-800">{nextMeeting?.title}</p>
              <p className="mt-1 text-sm text-slate-500">
                {nextMeeting?.room} · Host by {nextMeeting?.owner}
              </p>
            </div>
            <div className="rounded-2xl bg-white/80 p-4 text-sm text-slate-600 ring-1 ring-black/5">
              今日共 {totalMeetings} 场会议，路由入口已经配置完成，你可以继续扩展日历、纪要和通知页。
            </div>
          </CardContent>
        </Card>
      </section>

      <section className="grid gap-4 md:grid-cols-3">
        {[
          {
            title: "Vite + React",
            description: "前端基础模板来自官方 create-tauri-app 的 react-ts 模板。",
          },
          {
            title: "React Router",
            description: "已配置首页、议程页和 404 页，可继续拓展桌面多页面导航。",
          },
          {
            title: "shadcn/ui + Zustand",
            description: "按钮、卡片、徽标已就绪，示例状态也已经串上。",
          },
        ].map((item) => (
          <Card key={item.title} className="border border-black/5 bg-white/70" size="sm">
            <CardHeader>
              <CardTitle>{item.title}</CardTitle>
              <CardDescription>{item.description}</CardDescription>
            </CardHeader>
          </Card>
        ))}
      </section>

      <section className="rounded-[1.5rem] border border-dashed border-slate-300 bg-white/60 p-6 text-sm text-slate-600">
        <div className="flex items-center gap-2 font-medium text-slate-900">
          <CalendarDays className="size-4" />
          下一步建议
        </div>
        <p className="mt-3 leading-7">
          你可以继续把这套骨架扩展成会议预约、AI 纪要、设备联动或多人协同桌面应用。当前目录结构已经足够支持继续迭代。
        </p>
      </section>
    </div>
  );
}
