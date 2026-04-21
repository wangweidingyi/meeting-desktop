import { CalendarRange, Plus } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { useMeetingStore } from "@/stores/meeting-store";

export function AgendaPage() {
  const meetings = useMeetingStore((state) => state.meetings);
  const addQuickMeeting = useMeetingStore((state) => state.addQuickMeeting);

  return (
    <div className="space-y-6">
      <section className="flex flex-col gap-4 rounded-[1.75rem] border border-black/5 bg-white/80 p-6 shadow-sm shadow-black/5 md:flex-row md:items-center md:justify-between">
        <div>
          <p className="text-sm uppercase tracking-[0.2em] text-slate-500">Agenda</p>
          <h2 className="mt-2 text-3xl font-semibold tracking-tight text-slate-900">今日会议列表</h2>
        </div>
        <Button onClick={addQuickMeeting}>
          <Plus className="size-4" />
          添加示例会议
        </Button>
      </section>

      <section className="grid gap-4">
        {meetings.map((meeting) => (
          <Card key={meeting.id} className="border border-black/5 bg-white/70">
            <CardHeader>
              <div className="flex items-start justify-between gap-4">
                <div>
                  <CardTitle>{meeting.title}</CardTitle>
                  <CardDescription className="mt-1">
                    {meeting.room} · Host by {meeting.owner}
                  </CardDescription>
                </div>
                <Badge variant="outline">{meeting.time}</Badge>
              </div>
            </CardHeader>
            <CardContent className="flex items-center gap-2 text-sm text-slate-600">
              <CalendarRange className="size-4" />
              使用 Zustand 管理议程列表，后续可以无缝接入本地数据库或远端 API。
            </CardContent>
          </Card>
        ))}
      </section>
    </div>
  );
}
