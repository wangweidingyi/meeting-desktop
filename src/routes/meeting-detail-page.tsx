import { Download, FileText, ListTodo } from "lucide-react";
import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { getMeetingDetail, exportMarkdown } from "@/lib/api/commands";
import type { MeetingDetailView } from "@/features/meetings/models";
import { TranscriptStreamPanel } from "@/features/transcript/components/transcript-stream-panel";
import { LiveSummaryPanel } from "@/features/summary/components/live-summary-panel";
import { ActionItemsPanel } from "@/features/summary/components/action-items-panel";

export function MeetingDetailPage() {
  const { meetingId = "meeting" } = useParams();
  const [detail, setDetail] = useState<MeetingDetailView | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let disposed = false;

    void getMeetingDetail(meetingId)
      .then((response) => {
        if (!disposed) {
          setDetail(response);
          setError(null);
        }
      })
      .catch((reason) => {
        if (!disposed) {
          setError(String(reason));
        }
      });

    return () => {
      disposed = true;
    };
  }, [meetingId]);

  async function handleExportMarkdown() {
    const markdown = await exportMarkdown(meetingId);
    const blob = new Blob([markdown], { type: "text/markdown;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `${meetingId}.md`;
    anchor.click();
    URL.revokeObjectURL(url);
  }

  async function handleCopySummary() {
    if (!detail?.summary.abstract) {
      return;
    }
    await navigator.clipboard.writeText(detail.summary.abstract);
  }

  const meeting = detail?.meeting;

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
              <h1 className="text-3xl font-semibold tracking-tight text-slate-900">
                {meeting?.title ?? "正在加载会议详情"}
              </h1>
              <p className="mt-2 text-sm text-slate-500">
                {meeting
                  ? `${meeting.started_at}${meeting.ended_at ? ` - ${meeting.ended_at}` : " · 未结束"}`
                  : "正在从本地 SQLite 读取会议详情"}
              </p>
            </div>
          </div>

          <div className="flex flex-wrap gap-3">
            <Button variant="outline" onClick={() => void handleCopySummary()} disabled={!detail}>
              <FileText className="size-4" />
              复制纪要
            </Button>
            <Button onClick={() => void handleExportMarkdown()} disabled={!detail}>
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
            <TranscriptStreamPanel segments={detail?.transcriptSegments ?? []} />
          </CardContent>
        </Card>

        <div className="space-y-6">
          <Card className="border border-black/5 bg-white/85">
            <CardHeader>
              <CardTitle>最终会议纪要</CardTitle>
              <CardDescription>结构化输出：摘要、决策、风险、行动项。</CardDescription>
            </CardHeader>
            <CardContent>
              <LiveSummaryPanel
                summary={
                  detail?.summary ?? {
                    version: 0,
                    isFinal: false,
                    abstract: error ? `加载失败：${error}` : "尚未生成最终纪要",
                    keyPoints: { title: "关键要点", items: [] },
                    decisions: { title: "决策", items: [] },
                    risks: { title: "风险", items: [] },
                    actionItems: { title: "行动项", items: [] },
                    lastUpdatedLabel: "尚未生成",
                  }
                }
              />
            </CardContent>
          </Card>

          <Card className="border border-black/5 bg-white/85">
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <ListTodo className="size-4" />
                行动项
              </CardTitle>
            </CardHeader>
            <CardContent>
              <ActionItemsPanel items={detail?.actionItems ?? []} />
            </CardContent>
          </Card>
        </div>
      </section>
    </div>
  );
}
