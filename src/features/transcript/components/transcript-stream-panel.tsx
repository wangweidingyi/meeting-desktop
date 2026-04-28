import { Badge } from "@/components/ui/badge";
import type { TranscriptSegmentView } from "@/features/transcript/models";

type TranscriptStreamPanelProps = {
  segments: TranscriptSegmentView[];
};

function formatTranscriptOffset(offsetMs: number) {
  const totalSeconds = Math.max(0, Math.floor(offsetMs / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;

  return `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}

function segmentTimeLabel(segment: TranscriptSegmentView) {
  if (segment.id.endsWith("-transcript")) {
    return `已转写至 ${formatTranscriptOffset(segment.endMs)}`;
  }

  return `${formatTranscriptOffset(segment.startMs)} - ${formatTranscriptOffset(segment.endMs)}`;
}

export function TranscriptStreamPanel({ segments }: TranscriptStreamPanelProps) {
  if (segments.length === 0) {
    return (
      <div className="rounded-2xl border border-dashed border-black/10 bg-slate-50/70 px-4 py-6 text-sm text-slate-500">
        正在等待实时转写结果，新的语音片段会按时间顺序出现在这里。
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {segments.map((segment) => (
        <div
          key={segment.id}
          className="rounded-2xl border border-black/5 bg-slate-50/80 px-4 py-3 text-sm leading-7 text-slate-700"
        >
          <div className="mb-2 flex items-center gap-2 text-xs text-slate-500">
            <span>{segmentTimeLabel(segment)}</span>
            <Badge variant={segment.isFinal ? "default" : "secondary"} className="rounded-full px-2 py-0 text-[11px]">
              {segment.isFinal ? "Final" : `Rev ${segment.revision}`}
            </Badge>
          </div>
          <div>{segment.text}</div>
        </div>
      ))}
    </div>
  );
}
