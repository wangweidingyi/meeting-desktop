import { Badge } from "@/components/ui/badge";
import type { SummaryViewState } from "@/features/summary/models";

type LiveSummaryPanelProps = {
  summary: SummaryViewState;
};

const summarySections = (summary: SummaryViewState) => [
  { title: "摘要", content: summary.abstract, items: [] as string[] },
  { title: summary.keyPoints.title, content: "", items: summary.keyPoints.items },
  { title: summary.decisions.title, content: "", items: summary.decisions.items },
  { title: summary.risks.title, content: "", items: summary.risks.items },
  { title: summary.actionItems.title, content: "", items: summary.actionItems.items },
];

export function LiveSummaryPanel({ summary }: LiveSummaryPanelProps) {
  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between rounded-2xl border border-black/5 bg-slate-50/80 px-4 py-3">
        <div className="text-xs text-slate-500">最近更新：{summary.lastUpdatedLabel}</div>
        <Badge variant={summary.isFinal ? "default" : "secondary"} className="rounded-full px-2 py-0 text-[11px]">
          {summary.isFinal ? `Final v${summary.version}` : `Live v${summary.version}`}
        </Badge>
      </div>

      {summarySections(summary).map((section) => (
        <div key={section.title} className="space-y-2 rounded-2xl border border-black/5 bg-slate-50/80 p-4">
          <div className="text-sm font-semibold text-slate-900">{section.title}</div>
          {section.content ? <p className="text-sm leading-7 text-slate-600">{section.content}</p> : null}
          {section.items.length > 0 ? (
            <div className="space-y-2">
              {section.items.map((item) => (
                <div key={item} className="text-sm leading-7 text-slate-600">
                  {item}
                </div>
              ))}
            </div>
          ) : !section.content ? (
            <div className="text-sm text-slate-400">暂无内容</div>
          ) : null}
        </div>
      ))}
    </div>
  );
}
