import { useState } from "react";
import { AudioLines, LayoutDashboard, ScrollText } from "lucide-react";
import { BrowserRouter, NavLink, Navigate, Route, Routes } from "react-router-dom";

import {
  clearDesktopAuthSession,
  getDesktopAuthSession,
  loginDesktop,
  logoutDesktop,
  type DesktopAuthSession,
} from "@/lib/auth";
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
  const [session, setSession] = useState(() => getDesktopAuthSession());

  if (!session) {
    return <DesktopLoginPage onSuccess={setSession} />;
  }

  return (
    <BrowserRouter>
      <div className="min-h-screen text-slate-900">
        <header className="border-b border-black/5 bg-white/70 backdrop-blur-xl">
          <div className="mx-auto flex max-w-6xl items-center justify-between px-6 py-5">
            <div>
              <p className="text-sm uppercase tracking-[0.3em] text-slate-500">Meeting Assistant</p>
              <h1 className="mt-1 text-lg font-semibold">会议录音转写与纪要工作台</h1>
            </div>
            <div className="flex items-center gap-4">
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

              <div className="hidden rounded-3xl border border-black/5 bg-white/80 px-4 py-3 text-right shadow-sm md:block">
                <div className="text-sm font-medium text-slate-900">
                  {session.user.displayName || session.user.username}
                </div>
                <div className="text-xs uppercase tracking-[0.18em] text-slate-500">
                  {session.user.role} · @{session.user.username}
                </div>
              </div>

              <button
                type="button"
                className="inline-flex h-10 items-center rounded-full border border-black/10 bg-white/80 px-4 text-sm font-medium text-slate-700 transition hover:border-slate-900 hover:text-slate-900"
                onClick={() => {
                  void logoutDesktop().finally(() => {
                    clearDesktopAuthSession();
                    setSession(null);
                  });
                }}
              >
                退出登录
              </button>
            </div>
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

type DesktopLoginPageProps = {
  onSuccess: (session: DesktopAuthSession) => void;
};

function DesktopLoginPage({ onSuccess }: DesktopLoginPageProps) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [errorMessage, setErrorMessage] = useState("");

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setIsSubmitting(true);
    setErrorMessage("");

    try {
      const session = await loginDesktop(username, password);
      onSuccess(session);
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : "登录失败");
    } finally {
      setIsSubmitting(false);
    }
  }

  return (
    <div className="min-h-screen bg-[radial-gradient(circle_at_top,rgba(125,211,252,0.18),transparent_28%),linear-gradient(180deg,#f8fafc_0%,#eef2ff_100%)] px-6 py-10 text-slate-900">
      <div className="mx-auto grid min-h-[calc(100vh-5rem)] max-w-6xl gap-8 lg:grid-cols-[1.1fr,0.9fr] lg:items-center">
        <section className="space-y-6 rounded-[36px] border border-white/60 bg-white/70 p-10 shadow-[0_30px_100px_rgba(15,23,42,0.12)] backdrop-blur">
          <p className="text-sm uppercase tracking-[0.32em] text-slate-500">Meeting Assistant</p>
          <div className="space-y-3">
            <h1 className="text-5xl font-semibold tracking-tight text-slate-950">账号登录</h1>
            <p className="max-w-2xl text-sm leading-7 text-slate-600">
              使用管理后台创建的账号进入桌面端。登录后会议记录会自动绑定到当前用户，并同步到服务端供后台查看。
            </p>
          </div>
          <div className="grid gap-4 sm:grid-cols-3">
            {[
              "会议记录归属到真实用户",
              "桌面端与管理端共享账号体系",
              "后续支持多种流式识别模型扩展",
            ].map((item) => (
              <div
                key={item}
                className="rounded-3xl border border-slate-200 bg-white/80 px-5 py-4 text-sm leading-6 text-slate-600"
              >
                {item}
              </div>
            ))}
          </div>
        </section>

        <section className="rounded-[32px] border border-slate-200 bg-white/92 p-8 shadow-[0_24px_80px_rgba(15,23,42,0.08)]">
          <div className="space-y-2">
            <p className="text-xs font-medium tracking-[0.24em] text-slate-500 uppercase">Sign In</p>
            <h2 className="text-2xl font-semibold tracking-tight text-slate-950">进入会议工作台</h2>
          </div>

          <form className="mt-8 space-y-5" onSubmit={handleSubmit}>
            <label className="block space-y-2">
              <span className="text-sm font-medium text-slate-700">用户名</span>
              <input
                className="h-12 w-full rounded-2xl border border-slate-200 bg-slate-50 px-4 text-sm text-slate-900 outline-none transition focus:border-slate-900 focus:bg-white"
                value={username}
                onChange={(event) => setUsername(event.target.value)}
                autoComplete="username"
                placeholder="请输入用户名"
              />
            </label>

            <label className="block space-y-2">
              <span className="text-sm font-medium text-slate-700">密码</span>
              <input
                className="h-12 w-full rounded-2xl border border-slate-200 bg-slate-50 px-4 text-sm text-slate-900 outline-none transition focus:border-slate-900 focus:bg-white"
                type="password"
                value={password}
                onChange={(event) => setPassword(event.target.value)}
                autoComplete="current-password"
                placeholder="请输入密码"
              />
            </label>

            {errorMessage ? (
              <div className="rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700">
                {errorMessage}
              </div>
            ) : null}

            <button
              type="submit"
              disabled={isSubmitting}
              className="inline-flex h-12 w-full items-center justify-center rounded-full bg-slate-900 px-5 text-sm font-medium text-white transition hover:bg-slate-700 disabled:cursor-not-allowed disabled:bg-slate-400"
            >
              {isSubmitting ? "登录中..." : "登录桌面端"}
            </button>
          </form>
        </section>
      </div>
    </div>
  );
}
