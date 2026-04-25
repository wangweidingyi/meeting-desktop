import { type ReactNode, useEffect } from "react";

import { cn } from "@/lib/utils";

type SheetProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  children: ReactNode;
};

export function Sheet({ open, onOpenChange, children }: SheetProps) {
  useEffect(() => {
    if (!open) {
      return;
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onOpenChange(false);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [open, onOpenChange]);

  if (!open) {
    return null;
  }

  return (
    <div className="fixed inset-0 z-50">
      <button
        aria-label="关闭运行信息面板"
        className="absolute inset-0 bg-slate-950/30 backdrop-blur-[2px]"
        onClick={() => onOpenChange(false)}
        type="button"
      />
      {children}
    </div>
  );
}

type SheetContentProps = {
  children: ReactNode;
  className?: string;
};

export function SheetContent({ children, className }: SheetContentProps) {
  return (
    <div
      aria-modal="true"
      className={cn(
        "absolute inset-y-0 right-0 flex w-full max-w-xl flex-col border-l border-black/10 bg-[#fbfcfe] shadow-2xl shadow-slate-900/20",
        className,
      )}
      role="dialog"
    >
      {children}
    </div>
  );
}

type SheetHeaderProps = {
  children: ReactNode;
  className?: string;
};

export function SheetHeader({ children, className }: SheetHeaderProps) {
  return <div className={cn("space-y-2 border-b border-black/5 px-5 py-5", className)}>{children}</div>;
}

type SheetTitleProps = {
  children: ReactNode;
  className?: string;
};

export function SheetTitle({ children, className }: SheetTitleProps) {
  return <h2 className={cn("text-2xl font-semibold tracking-tight text-slate-950", className)}>{children}</h2>;
}

type SheetDescriptionProps = {
  children: ReactNode;
  className?: string;
};

export function SheetDescription({ children, className }: SheetDescriptionProps) {
  return <p className={cn("text-sm leading-6 text-slate-600", className)}>{children}</p>;
}
