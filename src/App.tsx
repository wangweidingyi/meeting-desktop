import { AudioLines, LayoutDashboard, ScrollText } from "lucide-react";
import { BrowserRouter, NavLink, Navigate, Route, Routes } from "react-router-dom";

import { HomePage } from "@/routes/home-page";
import { LiveMeetingPage } from "@/routes/live-meeting-page";
import { MeetingDetailPage } from "@/routes/meeting-detail-page";
import { NotFoundPage } from "@/routes/not-found-page";

const navigationItems = [
  {
    to: "/",
    label: "会议主页",
    icon: LayoutDashboard,
  },
  {
    to: "/meetings/live",
    label: "会中工作台",
    icon: AudioLines,
  },
  {
    to: "/meetings/2026-04-21-product-strategy",
    label: "会后详情",
    icon: ScrollText,
  },
];

function App() {
  return (
    <BrowserRouter>
      <div className="min-h-screen text-slate-900">
        <header className="border-b border-black/5 bg-white/70 backdrop-blur-xl">
          <div className="mx-auto flex max-w-6xl items-center justify-between px-6 py-5">
            <div>
              <p className="text-sm uppercase tracking-[0.3em] text-slate-500">Meeting Assistant</p>
              <h1 className="mt-1 text-lg font-semibold">会议录音转写与纪要工作台</h1>
            </div>
            <nav className="flex items-center gap-2 rounded-full border border-black/5 bg-white/70 p-1">
              {navigationItems.map((item) => {
                const Icon = item.icon;

                return (
                  <NavLink
                    key={item.to}
                    to={item.to}
                    end={item.to === "/"}
                    className={({ isActive }) =>
                      [
                        "flex items-center gap-2 rounded-full px-4 py-2 text-sm transition-colors",
                        isActive ? "bg-slate-900 text-white" : "text-slate-600 hover:bg-slate-100",
                      ].join(" ")
                    }
                  >
                    <Icon className="size-4" />
                    {item.label}
                  </NavLink>
                );
              })}
            </nav>
          </div>
        </header>

        <main className="mx-auto max-w-6xl px-6 py-10">
          <Routes>
            <Route path="/" element={<HomePage />} />
            <Route path="/meetings/live" element={<LiveMeetingPage />} />
            <Route path="/meetings/:meetingId" element={<MeetingDetailPage />} />
            <Route path="/404" element={<NotFoundPage />} />
            <Route path="*" element={<Navigate replace to="/404" />} />
          </Routes>
        </main>
      </div>
    </BrowserRouter>
  );
}

export default App;
