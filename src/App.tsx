import { CalendarDays, LayoutDashboard } from "lucide-react";
import { BrowserRouter, NavLink, Navigate, Route, Routes } from "react-router-dom";

import { AgendaPage } from "@/routes/agenda-page";
import { HomePage } from "@/routes/home-page";
import { NotFoundPage } from "@/routes/not-found-page";

const navigationItems = [
  {
    to: "/",
    label: "概览",
    icon: LayoutDashboard,
  },
  {
    to: "/agenda",
    label: "议程",
    icon: CalendarDays,
  },
];

function App() {
  return (
    <BrowserRouter>
      <div className="min-h-screen text-slate-900">
        <header className="border-b border-black/5 bg-white/70 backdrop-blur-xl">
          <div className="mx-auto flex max-w-6xl items-center justify-between px-6 py-5">
            <div>
              <p className="text-sm uppercase tracking-[0.3em] text-slate-500">Meeting</p>
              <h1 className="mt-1 text-lg font-semibold">Tauri v2 App Shell</h1>
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
            <Route path="/agenda" element={<AgendaPage />} />
            <Route path="/404" element={<NotFoundPage />} />
            <Route path="*" element={<Navigate replace to="/404" />} />
          </Routes>
        </main>
      </div>
    </BrowserRouter>
  );
}

export default App;
