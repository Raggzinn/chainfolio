import { CalendarRange } from "lucide-react";
import { useState } from "react";
import type { DateRange } from "react-day-picker";
import { enUS, ptBR } from "react-day-picker/locale";
import { useTranslation } from "react-i18next";
import { Calendar } from "@/components/ui/calendar";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { DEMO_HISTORY_DAYS } from "@/features/demo/data";
import { cn } from "@/lib/utils";

const DAY_MS = 86_400_000;

/** Calendar button for picking a specific start and end date. */
export function DateRangePicker({
  value,
  active,
  onChange,
}: {
  value?: DateRange;
  active: boolean;
  onChange: (range: DateRange) => void;
}) {
  const { t, i18n } = useTranslation();
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState<DateRange | undefined>(value);
  const locale = i18n.resolvedLanguage === "pt-BR" ? ptBR : enUS;
  const today = new Date();
  const earliest = new Date(today.getTime() - (DEMO_HISTORY_DAYS - 1) * DAY_MS);

  const short = new Intl.DateTimeFormat(i18n.resolvedLanguage, { day: "numeric", month: "short" });
  const label =
    active && value?.from && value.to
      ? `${short.format(value.from)} – ${short.format(value.to)}`
      : t("filters.custom");

  const apply = () => {
    if (draft?.from) {
      onChange({ from: draft.from, to: draft.to ?? draft.from });
      setOpen(false);
    }
  };

  return (
    <Popover
      open={open}
      onOpenChange={(next) => {
        setOpen(next);
        if (next) setDraft(value);
      }}
    >
      <PopoverTrigger asChild>
        <button
          type="button"
          aria-pressed={active}
          className={cn(
            "flex h-9 items-center gap-2 rounded-xl border bg-card px-3 font-medium text-xs transition-colors hover:bg-accent",
            active && "border-primary/40 text-primary",
          )}
        >
          <CalendarRange className="size-4" />
          {label}
        </button>
      </PopoverTrigger>
      <PopoverContent align="end" className="w-auto p-0">
        <Calendar
          mode="range"
          numberOfMonths={2}
          locale={locale}
          selected={draft}
          onSelect={setDraft}
          defaultMonth={draft?.from ?? new Date(today.getFullYear(), today.getMonth() - 1, 1)}
          disabled={{ before: earliest, after: today }}
          startMonth={earliest}
          endMonth={today}
          captionLayout="dropdown"
        />
        <div className="flex items-center justify-between gap-3 border-t px-3 py-2.5">
          <span className="text-muted-foreground text-xs">
            {draft?.from
              ? `${short.format(draft.from)}${draft.to ? ` – ${short.format(draft.to)}` : ""}`
              : t("filters.pickDates")}
          </span>
          <button
            type="button"
            disabled={!draft?.from}
            onClick={apply}
            className="rounded-lg bg-primary px-3 py-1.5 font-medium text-primary-foreground text-xs transition-colors hover:bg-primary/90 disabled:opacity-40"
          >
            {t("filters.apply")}
          </button>
        </div>
      </PopoverContent>
    </Popover>
  );
}
