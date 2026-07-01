import type { Copy } from "./i18n";
import type { BtcMacroSnapshot } from "./types";
import { formatPct, formatRegime, formatUsd } from "./uiFormat";

export function MacroSummaryStrip({
  copy,
  error,
  loading,
  snapshot,
}: {
  copy: Copy;
  error: string | null;
  loading: boolean;
  snapshot: BtcMacroSnapshot | null;
}) {
  if (loading && snapshot === null) {
    return (
      <section className="macro-summary-strip is-loading" data-testid="macro-summary-strip">
        <div>
          <span>{copy.macro.summaryLabel}</span>
          <strong>{copy.macro.summaryLoading}</strong>
        </div>
      </section>
    );
  }

  if (snapshot === null) {
    return (
      <section className="macro-summary-strip is-unavailable" data-testid="macro-summary-strip">
        <div>
          <span>{copy.macro.summaryLabel}</span>
          <strong>{copy.macro.summaryUnavailable}</strong>
          {error ? <em>{error}</em> : null}
        </div>
      </section>
    );
  }

  const metrics = [
    { label: copy.macro.regime, value: formatRegime(snapshot.regime, copy) },
    { label: copy.macro.price, value: formatUsd(snapshot.price) },
    { label: copy.macro.confidence, value: `${snapshot.confidence}/100`, tone: "positive" },
    {
      label: copy.macro.drawdown,
      value: formatPct(snapshot.trend.drawdown_from_window_ath_pct),
      tone: snapshot.trend.drawdown_from_window_ath_pct < 0 ? "negative" : "positive",
    },
    {
      label: copy.macro.cycleProgress,
      value: formatPct(snapshot.cycle.estimated_cycle_progress_pct),
    },
    {
      label: copy.macro.ma200w,
      value: snapshot.trend.ma_200w === null ? "-" : formatUsd(snapshot.trend.ma_200w),
    },
  ];

  return (
    <section className="macro-summary-strip" data-testid="macro-summary-strip">
      <div className="macro-summary-heading">
        <span>{copy.macro.summaryLabel}</span>
        <strong>{snapshot.summary}</strong>
      </div>
      {metrics.map((metric) => (
        <div className="macro-summary-tile" key={metric.label}>
          <span>{metric.label}</span>
          <strong className={metric.tone ?? ""}>{metric.value}</strong>
        </div>
      ))}
    </section>
  );
}
