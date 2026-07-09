import type { Copy } from "./i18n";
import { RadarTable } from "./RadarTable";
import { SymbolDetailPanel } from "./SymbolDetailPanel";
import type { BtcMacroSnapshot, PaperAccountSnapshot, SymbolSnapshot } from "./types";
import { formatPct, formatUsd, type Filter, type ThemeMode } from "./uiFormat";

export function MonitorPage({
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
            ["positions", "持仓"],
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
          LIVE · {filteredSymbols.length} symbols
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
  error,
  filteredSymbols,
  loading,
  paper,
  snapshot,
}: {
  error: string | null;
  filteredSymbols: SymbolSnapshot[];
  loading: boolean;
  paper: PaperAccountSnapshot;
  snapshot: BtcMacroSnapshot | null;
}) {
  const btcSymbol =
    filteredSymbols.find((symbol) => symbol.inst_id.startsWith("BTC-")) ?? filteredSymbols[0] ?? null;
  const hotSymbols = [...filteredSymbols].sort(
    (left, right) => Math.abs(right.change_1h_pct) - Math.abs(left.change_1h_pct),
  );
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
        label="BTC 价格"
        value={snapshot === null ? btcSymbol ? formatUsd(btcSymbol.price) : "-" : formatUsd(snapshot.price)}
        sub={btcSymbol ? `${formatPct(btcSymbol.change_1h_pct)} 1h` : "等待行情"}
        tone="cyan"
      />
      <StatCard
        label="市场状态"
        value={marketState}
        sub={snapshot === null ? "宏观数据同步中" : snapshot.summary}
        tone={marketState === "BULL" ? "green" : marketState === "RISK" ? "amber" : undefined}
      />
      <StatCard
        label="活跃信号"
        value={String(longCount + shortCount)}
        sub={`LONG ${longCount} / SHORT ${shortCount}`}
      />
      <StatCard
        label="热门异动"
        value={hot ? `${shortSymbol(hot.inst_id)} ${formatPct(hot.change_1h_pct)}` : "-"}
        sub={runnerUp ? `${shortSymbol(runnerUp.inst_id)} ${formatPct(runnerUp.change_1h_pct)}` : "等待异动"}
        tone="amber"
      />
      <StatCard
        label="Altseason"
        value={altAllowed ? "允许" : "限制"}
        sub={`持仓 ${paper.positions.length} · 浮盈 ${formatUsd(paper.unrealized_pnl)}`}
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
