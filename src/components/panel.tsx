import { cn } from "@/lib/utils";

/** Frosted card used across the app. */
export function Panel({
  title,
  subtitle,
  action,
  className,
  tourId,
  children,
}: {
  title?: React.ReactNode;
  subtitle?: React.ReactNode;
  action?: React.ReactNode;
  className?: string;
  /** Marks the panel as a product tour target. */
  tourId?: string;
  children: React.ReactNode;
}) {
  return (
    <section
      data-tour={tourId}
      className={cn("surface relative overflow-hidden rounded-2xl p-5", className)}
    >
      {(title || action) && (
        <header className="mb-4 flex items-start justify-between gap-4">
          <div className="space-y-0.5">
            {title && <h2 className="font-semibold text-[15px] tracking-tight">{title}</h2>}
            {subtitle && <p className="text-muted-foreground text-xs">{subtitle}</p>}
          </div>
          {action}
        </header>
      )}
      {children}
    </section>
  );
}
