import { Download, FileText, ListTodo } from "lucide-react";
import { useParams } from "react-router-dom";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

const transcriptSections = [
  "主持人：今天先把 Windows 双路音频采集和 mixed 上行边界确认下来。",
  "研发：Rust 侧会统一控制 MQTT、UDP、SQLite 和恢复状态机。",
  "产品：会后导出至少支持 Markdown，行动项需要独立区块。",
];

const actionItems = [
  "完成 Phase 1：路由、类型系统、状态机骨架、SQLite schema。",
  "补齐 MQTT 控制协议和 UDP 音频包定义。",
  "实现恢复时的 mixed 未上传区间补发。",
];

export function MeetingDetailPage() {
  const { meetingId = "meeting" } = useParams();

  return (
    <div className="space-y-6">
      <section className="rounded-[1.75rem] border border-black/5 bg-white/85 p-6 shadow-sm shadow-black/5">
        <div className="flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
          <div className="space-y-3">
            <div className="flex items-center gap-3">
              <Badge variant="secondary" className="rounded-full px-3 py-1 text-xs">
                会后详情
              </Badge>
              <Badge variant="outline">{meetingId}</Badge>
            </div>
            <div>
              <h1 className="text-3xl font-semibold tracking-tight text-slate-900">产品策略例会</h1>
              <p className="mt-2 text-sm text-slate-500">2026-04-21 09:30 - 10:18 · 已生成最终纪要和行动项</p>
            </div>
          </div>

          <div className="flex flex-wrap gap-3">
            <Button variant="outline">
              <FileText className="size-4" />
              复制纪要
            </Button>
            <Button>
              <Download className="size-4" />
              导出 Markdown
            </Button>
          </div>
        </div>
      </section>

      <section className="grid gap-6 xl:grid-cols-[1.05fr_0.95fr]">
        <Card className="border border-black/5 bg-white/85">
          <CardHeader>
            <CardTitle>完整逐段转写</CardTitle>
            <CardDescription>转写片段会按时间顺序展示，支持后续继续编辑。</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            {transcriptSections.map((line, index) => (
              <div key={line} className="rounded-2xl border border-black/5 bg-slate-50/80 px-4 py-3 text-sm leading-7 text-slate-700">
                <div className="mb-1 text-xs font-medium text-slate-400">00:0{index + 1}:12</div>
                {line}
              </div>
            ))}
          </CardContent>
        </Card>

        <div className="space-y-6">
          <Card className="border border-black/5 bg-white/85">
            <CardHeader>
              <CardTitle>最终会议纪要</CardTitle>
              <CardDescription>结构化输出：摘要、决策、风险、行动项。</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4 text-sm leading-7 text-slate-600">
              <p>会议明确以 Rust 为主控推进桌面端 runtime，React 仅做 UI 展示。</p>
              <p>控制链路采用 MQTT，音频链路采用 UDP，首版上传 mixed 单流，双路原始 WAV 本地保存。</p>
              <p>历史记录、崩溃恢复、Markdown 导出纳入 MVP，而 speaker diarization 和 Opus 编码后置。</p>
            </CardContent>
          </Card>

          <Card className="border border-black/5 bg-white/85">
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <ListTodo className="size-4" />
                行动项
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              {actionItems.map((item) => (
                <div key={item} className="rounded-2xl border border-black/5 bg-slate-50/80 px-4 py-3 text-sm leading-6 text-slate-700">
                  {item}
                </div>
              ))}
            </CardContent>
          </Card>
        </div>
      </section>
    </div>
  );
}
