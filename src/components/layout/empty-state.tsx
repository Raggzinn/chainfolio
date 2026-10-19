import type { LucideIcon } from "lucide-react";

export function EmptyState({
  icon: Icon,
  title,
  description,
  action,
}: {
  icon: LucideIcon;
  title: string;
  description?: string;
  action?: React.ReactNode;
}) {
  return (
    <div className="surface relative flex flex-col items-center justify-center gap-4 overflow-hidden rounded-2xl px-6 py-20 text-center">
      <div className="relative flex size-14 items-center justify-center rounded-2xl bg-primary/12">
        <Icon className="size-6 text-primary" />
      </div>
      <div className="relative max-w-md space-y-1.5">
        <h2 className="font-semibold text-lg tracking-tight">{title}</h2>
        {description && (
          <p className="text-muted-foreground text-sm leading-relaxed">{description}</p>
        )}
      </div>
      {action && <div className="relative">{action}</div>}
    </div>
  );
}
