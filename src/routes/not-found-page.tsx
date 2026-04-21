import { Link } from "react-router-dom";

import { Button } from "@/components/ui/button";

export function NotFoundPage() {
  return (
    <div className="flex min-h-[60vh] flex-col items-center justify-center gap-4 rounded-[2rem] border border-dashed border-slate-300 bg-white/70 px-6 text-center">
      <p className="text-sm uppercase tracking-[0.3em] text-slate-500">404</p>
      <h1 className="text-3xl font-semibold text-slate-900">页面不存在</h1>
      <p className="max-w-md text-sm leading-7 text-slate-600">
        当前路由已经接入完成，这里是兜底页面。你后续新增功能页时，只需要继续补充 route 即可。
      </p>
      <Button asChild>
        <Link to="/">返回首页</Link>
      </Button>
    </div>
  );
}
