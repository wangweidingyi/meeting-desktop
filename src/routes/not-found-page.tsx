import { Link } from "react-router-dom";

import { Button } from "@/components/ui/button";

export function NotFoundPage() {
  return (
    <div className="flex min-h-[60vh] flex-col items-center justify-center gap-4 rounded-[2rem] border border-dashed border-slate-300 bg-white/70 px-6 text-center">
      <p className="text-sm uppercase tracking-[0.3em] text-slate-500">404</p>
      <h1 className="text-3xl font-semibold text-slate-900">页面不存在</h1>
      <p className="max-w-md text-sm leading-7 text-slate-600">
        这个地址没有对应的会议主页、会中工作台或会后详情页面，可以返回主工作台继续操作。
      </p>
      <Button asChild>
        <Link to="/">返回首页</Link>
      </Button>
    </div>
  );
}
