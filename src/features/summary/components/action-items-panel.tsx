type ActionItemsPanelProps = {
  items: string[];
};

export function ActionItemsPanel({ items }: ActionItemsPanelProps) {
  return (
    <div className="space-y-3">
      {items.length > 0 ? (
        items.map((item) => (
          <div
            key={item}
            className="rounded-2xl border border-black/5 bg-slate-50/80 px-4 py-3 text-sm leading-6 text-slate-700"
          >
            {item}
          </div>
        ))
      ) : (
        <div className="rounded-2xl border border-dashed border-black/10 bg-slate-50/70 px-4 py-6 text-sm text-slate-500">
          暂无行动项。
        </div>
      )}
    </div>
  );
}
