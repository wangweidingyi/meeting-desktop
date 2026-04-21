import { create } from "zustand";

import type { MeetingListItem } from "@/features/meetings/models";

type MeetingStore = {
  meetings: MeetingListItem[];
  query: string;
  setQuery: (query: string) => void;
};

const initialMeetings: MeetingListItem[] = [
  {
    id: "2026-04-21-product-strategy",
    title: "产品策略例会",
    startedAt: "2026-04-21 09:30",
    endedAt: "2026-04-21 10:18",
    durationLabel: "48 分钟",
    status: "completed",
    transcriptPreview: "围绕发布节奏、风险项和下周依赖做了集中确认。",
  },
  {
    id: "2026-04-20-customer-review",
    title: "客户复盘会",
    startedAt: "2026-04-20 16:00",
    endedAt: null,
    durationLabel: "进行中",
    status: "recording",
    transcriptPreview: "正在持续接收转写和纪要增量。",
  },
  {
    id: "2026-04-19-engineering-sync",
    title: "研发同步会",
    startedAt: "2026-04-19 14:00",
    endedAt: "2026-04-19 15:06",
    durationLabel: "66 分钟",
    status: "completed",
    transcriptPreview: "明确了音频链路、会后纪要模板和历史导出要求。",
  },
];

export const useMeetingStore = create<MeetingStore>((set) => ({
  meetings: initialMeetings,
  query: "",
  setQuery: (query) => {
    set({ query });
  },
}));
