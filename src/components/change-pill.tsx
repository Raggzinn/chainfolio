import { ArrowDownRight, ArrowUpRight } from "lucide-react";
import { useTranslation } from "react-i18next";
import { formatPercent } from "@/lib/format";
import { cn } from "@/lib/utils";

export function ChangePill({
  value,
  size = "sm",
  className,
}: {
  value: number;
  size?: "sm" | "md";
  className?: string;
}) {
  const { i18n } = useTranslation();
  const up = value >= 0;
  const Icon = up ? ArrowUpRight : ArrowDownRight;
  return (
    <span
      className={cn(
        "tabular inline-flex items-center gap-0.5 rounded-full font-semibold",
        size === "md" ? "px-2.5 py-1 text-sm" : "px-1.5 py-0.5 text-xs",
        up ? "bg-success/12 text-success" : "bg-danger/12 text-danger",
        className,
      )}
    >
      <Icon className={size === "md" ? "size-4" : "size-3.5"} strokeWidth={2.4} />
      {formatPercent(Math.abs(value), i18n.resolvedLanguage ?? "en", { signed: false })}
    </span>
  );
}
