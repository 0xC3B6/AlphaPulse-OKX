import type { Copy } from "./i18n";
import { RadarTable } from "./RadarTable";
import { SymbolDetailPanel } from "./SymbolDetailPanel";
import type { BtcMacroSnapshot, PaperAccountSnapshot, SymbolSnapshot } from "./types";
import {
  compareSymbolsByAmplitude24h,
  formatPct,
  formatUsd,
  type Filter,
  type ThemeMode,
} from "./uiFormat";

export function MonitorPage({
  btcSymbol,
  copy,
  filter,
  filteredSymbols,
  macroError,
  macroLoading,
  macroSnapshot,
  onFilterChange,
  onOpenTradingView,
  onSelectSymbol,
  onTradeSymbol,
  paper,
  selected,
  themeMode,
}: {
  btcSymbol: SymbolSnapshot | null;
  copy: Copy;
  filter: Filter;
  filteredSymbols: SymbolSnapshot[];
  macroError: string | null;
  macroLoading: boolean;
  macroSnapshot: BtcMacroSnapshot | null;
  onFilterChange: (filter: Filter) => void;
  onOpenTradingView: (symbol: SymbolSnapshot) => void;
  onSelectSymbol: (symbolId: string | null) => void;
  onTradeSymbol: (symbol: SymbolSnapshot) => void;
  paper: PaperAccountSnapshot;
  selected: SymbolSnapshot | null;
  themeMode: ThemeMode;
}) {
  const selectedVisible =
    filteredSymbols.find((symbol) => symbol.inst_id === selected?.inst_id) ?? null;

  return (
    <section className="monitor-terminal" data-testid="monitor-terminal">
      <FigmaStatBar
        btcSymbol={btcSymbol}
        copy={copy}
        error={macroError}
        filteredSymbols={filteredSymbols}
        loading={macroLoading}
        paper={paper}
        snapshot={macroSnapshot}
      />
      <section
        className="toolbar radar-filterbar figma-radar-tabs"
        data-testid="figma-radar-tabs"
        aria-label={copy.aria.radarControls}
      >
        <div className="toolbar-group" role="group" aria-label={copy.aria.opportunityFilters}>
          {[
            ["all", copy.filters.all],
            ["trend", copy.filters.trend],
            ["range", copy.filters.range],
            ["hot", copy.filters.hot],
            ["fixed", copy.filters.fixed],
            ["positions", copy.terminal.positions],
          ].map(([value, label]) => (
            <button
              className={filter === value ? "active" : ""}
              key={value}
              onClick={() => onFilterChange(value as Filter)}
              type="button"
            >
              {label}
            </button>
          ))}
        </div>
        <div className="monitor-live-count" data-testid="monitor-live-count">
          <span aria-hidden="true" />
          {copy.terminal.live} · {filteredSymbols.length} {copy.terminal.symbols}
        </div>
      </section>

      {filteredSymbols.length === 0 ? (
        <section className="empty-state">
          <h2>{copy.empty.title}</h2>
          <p>{copy.empty.body}</p>
        </section>
      ) : (
        <section className="radar-grid radar-workspace monitor-page">
          <div className="radar-list-column">
            <RadarTable
              copy={copy}
              onOpenTradingView={onOpenTradingView}
              onSelectSymbol={onSelectSymbol}
              paper={paper}
              selectedId={selectedVisible?.inst_id ?? null}
              symbols={filteredSymbols}
            />
          </div>
          {selectedVisible ? (
            <aside className="detail-panel figma-detail-column" data-testid="figma-detail-column">
              <SymbolDetailPanel
                copy={copy}
                onClose={() => onSelectSymbol(null)}
                onOpenTradingView={onOpenTradingView}
                onTradeSymbol={onTradeSymbol}
                symbol={selectedVisible}
                themeMode={themeMode}
              />
            </aside>
          ) : null}
        </section>
      )}
    </section>
  );
}

function FigmaStatBar({
  btcSymbol,
  copy,
  error,
  filteredSymbols,
  loading,
  paper,
  snapshot,
}: {
  btcSymbol: SymbolSnapshot | null;
  copy: Copy;
  error: string | null;
  filteredSymbols: SymbolSnapshot[];
  loading: boolean;
  paper: PaperAccountSnapshot;
  snapshot: BtcMacroSnapshot | null;
}) {
  const hotSymbols = [...filteredSymbols].sort(compareSymbolsByAmplitude24h);
  const hot = hotSymbols[0] ?? null;
  const runnerUp = hotSymbols[1] ?? null;
  const longCount = filteredSymbols.filter((symbol) => symbol.trend_score.direction === "long" || symbol.range_score.direction === "long").length;
  const shortCount = filteredSymbols.filter((symbol) => symbol.trend_score.direction === "short" || symbol.range_score.direction === "short").length;
  const altAllowed =
    snapshot?.market_permission.allowed_behaviors.some((behavior) => behavior.toLowerCase().includes("alt")) ??
    false;
  const marketState = snapshot === null ? (loading ? "..." : error ? "ERR" : "WAIT") : snapshot.regime.includes("bear") ? "RISK" : "BULL";

  return (
    <section className="figma-statbar" data-testid="figma-statbar">
      <StatCard
        label={copy.terminal.btcPrice}
        value={btcSymbol ? formatUsd(btcSymbol.price) : snapshot ? formatUsd(snapshot.price) : "-"}
        sub={btcSymbol ? `${formatPct(btcSymbol.change_1h_pct)} 1h` : copy.terminal.waitingForQuote}
        tone="cyan"
      />
      <StatCard
        label={copy.terminal.marketState}
        value={marketState}
        sub={snapshot === null ? copy.terminal.syncingMacro : snapshot.summary}
        tone={marketState === "BULL" ? "green" : marketState === "RISK" ? "amber" : undefined}
      />
      <StatCard
        label={copy.terminal.activeSignals}
        value={String(longCount + shortCount)}
        sub={`LONG ${longCount} / SHORT ${shortCount}`}
      />
      <StatCard
        label={copy.terminal.hotMover}
        value={hot ? `${shortSymbol(hot.inst_id)} ${formatPct(hot.amplitude_24h_pct ?? 0)}` : "-"}
        sub={runnerUp ? `${shortSymbol(runnerUp.inst_id)} ${formatPct(runnerUp.amplitude_24h_pct ?? 0)}` : copy.terminal.waitingForMove}
        tone="amber"
      />
      <StatCard
        label={copy.terminal.altseason}
        value={altAllowed ? copy.terminal.allowed : copy.terminal.restricted}
        sub={`${copy.terminal.positions} ${paper.positions.length} · ${copy.terminal.unrealized} ${formatUsd(paper.unrealized_pnl)}`}
        tone={altAllowed ? "violet" : undefined}
      />
    </section>
  );
}

function StatCard({
  label,
  sub,
  tone,
  value,
}: {
  label: string;
  sub: string;
  tone?: "amber" | "cyan" | "green" | "violet";
  value: string;
}) {
  return (
    <div className="figma-stat-card">
      <span>{label}</span>
      <strong className={tone ? `tone-${tone}` : undefined}>{value}</strong>
      <small>{sub}</small>
    </div>
  );
}

function shortSymbol(instId: string): string {
  return instId.replace(/-USDT-SWAP$/u, "").replace(/-USDT$/u, "");
}
