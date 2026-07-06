import { useMemo, useState } from "react";
import type { Copy } from "./i18n";
import type {
  PaperAccountSnapshot,
  PaperClosedPositionSnapshot,
  PaperStrategyStats,
  PaperTrade,
  TradeTag,
} from "./types";
import {
  formatPct,
  formatPrice,
  formatSignedUsdt,
  formatTimestamp,
  formatUsdt,
  pnlClass,
  summarizePaperReview,
} from "./uiFormat";

type HistoryFilter = "all" | "long" | "short" | "profit" | "loss";
type ReviewSection = "overview" | "strategy" | "history" | "trades";

interface StrategyCurvePoint {
  closedAtMs: number;
  cumulativePnl: number;
}

interface StrategyCurve {
  points: StrategyCurvePoint[];
  finalPnl: number;
  maxDrawdown: number;
  bestCumulative: number;
  worstCumulative: number;
}

export function ReviewPage({ copy, paper }: { copy: Copy; paper: PaperAccountSnapshot }) {
  const [activeSection, setActiveSection] = useState<ReviewSection>("overview");
  const [historyFilter, setHistoryFilter] = useState<HistoryFilter>("all");
  const [historySymbolQuery, setHistorySymbolQuery] = useState("");
  const [historyStartDate, setHistoryStartDate] = useState("");
  const [historyEndDate, setHistoryEndDate] = useState("");
  const [historyVersion, setHistoryVersion] = useState("all");
  const [selectedStrategyVersion, setSelectedStrategyVersion] = useState<string | null>(null);

  const summary = summarizePaperReview(paper);
  const positionHistory = paper.position_history ?? [];
  const strategyStats = useMemo(
    () =>
      paper.strategy_stats && paper.strategy_stats.length > 0
        ? paper.strategy_stats
        : buildStrategyStats(positionHistory),
    [paper.strategy_stats, positionHistory],
  );
  const activeStrategyVersion =
    selectedStrategyVersion ?? strategyStats[0]?.strategy_version ?? null;
  const activeStrategyCurve =
    activeStrategyVersion === null
      ? null
      : buildStrategyCurve(positionHistory, activeStrategyVersion);
  const historyVersionOptions = useMemo(
    () => strategyVersionOptions(positionHistory, strategyStats, paper.trades),
    [paper.trades, positionHistory, strategyStats],
  );
  const filteredHistory = useMemo(
    () =>
      positionHistory.filter(
        (position) =>
          matchesHistoryFilter(position, historyFilter) &&
          matchesHistoryDetailFilters(position, {
            endDate: historyEndDate,
            startDate: historyStartDate,
            symbolQuery: historySymbolQuery,
            version: historyVersion,
          }),
      ),
    [
      historyEndDate,
      historyFilter,
      historyStartDate,
      historySymbolQuery,
      historyVersion,
      positionHistory,
    ],
  );
  const profitFactor =
    paper.profit_factor === undefined
      ? summary.profitFactor === null
        ? "-"
        : summary.profitFactor.toFixed(2)
      : formatNullableRatio(paper.profit_factor);

  const tabs: Array<[ReviewSection, string]> = [
    ["overview", copy.review.performance],
    ["strategy", copy.paper.strategyComparison],
    ["history", copy.paper.positionHistory],
    ["trades", copy.review.tradeRecords],
  ];

  return (
    <section className="review-page page-surface" data-testid="review-page">
      <section className="page-local-tabs" aria-label={copy.views.review}>
        {tabs.map(([value, label]) => (
          <button
            aria-current={activeSection === value ? "page" : undefined}
            className={activeSection === value ? "active" : ""}
            key={value}
            onClick={() => setActiveSection(value)}
            type="button"
          >
            {label}
          </button>
        ))}
      </section>

      {activeSection === "overview" ? (
        <OverviewSection
          activeStrategyVersion={activeStrategyVersion}
          copy={copy}
          onSelectStrategyVersion={setSelectedStrategyVersion}
          paper={paper}
          profitFactor={profitFactor}
          strategyStats={strategyStats}
          summary={summary}
        />
      ) : null}

      {activeSection === "strategy" ? (
        <section className="detail-section review-strategy-card">
          <h2>{copy.paper.strategyComparison}</h2>
          <StrategyStatsTable
            copy={copy}
            onSelectVersion={setSelectedStrategyVersion}
            selectedVersion={activeStrategyVersion}
            stats={strategyStats}
          />
          {activeStrategyVersion === null || activeStrategyCurve === null ? null : (
            <StrategyCurvePanel
              copy={copy}
              curve={activeStrategyCurve}
              version={activeStrategyVersion}
            />
          )}
        </section>
      ) : null}

      {activeSection === "history" ? (
        <HistorySection
          copy={copy}
          filteredHistory={filteredHistory}
          historyEndDate={historyEndDate}
          historyFilter={historyFilter}
          historyStartDate={historyStartDate}
          historySymbolQuery={historySymbolQuery}
          historyVersion={historyVersion}
          historyVersionOptions={historyVersionOptions}
          onEndDateChange={setHistoryEndDate}
          onFilterChange={setHistoryFilter}
          onStartDateChange={setHistoryStartDate}
          onSymbolQueryChange={setHistorySymbolQuery}
          onVersionChange={setHistoryVersion}
          totalHistory={positionHistory.length}
        />
      ) : null}

      {activeSection === "trades" ? (
        <TradeRecordsSection copy={copy} trades={paper.trades} />
      ) : null}
    </section>
  );
}

function OverviewSection({
  activeStrategyVersion,
  copy,
  onSelectStrategyVersion,
  paper,
  profitFactor,
  strategyStats,
  summary,
}: {
  activeStrategyVersion: string | null;
  copy: Copy;
  onSelectStrategyVersion: (version: string) => void;
  paper: PaperAccountSnapshot;
  profitFactor: string;
  strategyStats: PaperStrategyStats[];
  summary: ReturnType<typeof summarizePaperReview>;
}) {
  return (
    <>
      <section className="page-metric-grid review-summary">
        <MetricCard label={copy.paper.initialBalance} value={formatUsdt(paper.initial_balance)} />
        <MetricCard label={copy.paper.equity} value={formatUsdt(paper.equity)} />
        <MetricCard label={copy.paper.available} value={formatUsdt(paper.available_balance)} />
        <MetricCard label={copy.paper.usedMargin} value={formatUsdt(paper.used_margin)} />
        <MetricCard
          label={copy.paper.realized}
          tone={pnlClass(paper.realized_pnl)}
          value={formatSignedUsdt(paper.realized_pnl)}
        />
        <MetricCard
          label={copy.paper.unrealized}
          tone={pnlClass(paper.unrealized_pnl)}
          value={formatSignedUsdt(paper.unrealized_pnl)}
        />
        <MetricCard
          label={copy.review.winRate}
          value={paper.win_rate === undefined ? formatPct(summary.winRate) : formatNullablePct(paper.win_rate)}
        />
        <MetricCard
          label={copy.review.closedTrades}
          value={String(paper.closed_position_count ?? summary.closedCount)}
        />
        <MetricCard label={copy.paper.totalTrades} value={String(paper.total_trades ?? paper.trades.length)} />
        <MetricCard
          label={copy.paper.losingPositions}
          tone={(paper.losing_closed_position_count ?? 0) > 0 ? "negative" : ""}
          value={String(paper.losing_closed_position_count ?? "-")}
        />
        <MetricCard
          label={copy.paper.averageHoldingDuration}
          value={formatNullableDuration(paper.average_holding_duration_ms ?? null)}
        />
        <MetricCard
          label={copy.paper.averagePositionPnl}
          tone={pnlClass(paper.average_closed_position_pnl ?? summary.averageClosedPnl)}
          value={formatNullableSignedUsdt(paper.average_closed_position_pnl ?? summary.averageClosedPnl)}
        />
        <MetricCard
          label={copy.review.averageWin}
          tone={pnlClass(paper.average_winning_pnl ?? summary.averageWin)}
          value={formatNullableSignedUsdt(paper.average_winning_pnl ?? summary.averageWin)}
        />
        <MetricCard
          label={copy.review.averageLoss}
          tone={pnlClass(paper.average_losing_pnl ?? summary.averageLoss)}
          value={formatNullableSignedUsdt(paper.average_losing_pnl ?? summary.averageLoss)}
        />
        <MetricCard
          label={copy.review.maxWin}
          tone={pnlClass(paper.largest_winning_pnl ?? summary.maxWin)}
          value={formatNullableSignedUsdt(paper.largest_winning_pnl ?? summary.maxWin)}
        />
        <MetricCard
          label={copy.review.maxLoss}
          tone={pnlClass(paper.largest_losing_pnl ?? summary.maxLoss)}
          value={formatNullableSignedUsdt(paper.largest_losing_pnl ?? summary.maxLoss)}
        />
        <MetricCard label={copy.review.profitFactor} value={profitFactor} />
        <MetricCard label={copy.paper.totalFees} value={formatUsdt(paper.total_fees ?? 0)} />
      </section>
      <section className="review-grid">
        <section className="detail-section review-chart-card">
          <header className="panel-heading">
            <div>
              <h2>{copy.review.realizedCurve}</h2>
              <p>{copy.paper.realized}: {formatSignedUsdt(paper.realized_pnl)}</p>
            </div>
          </header>
          <RealizedCurve points={summary.realizedPath} />
        </section>
        <section className="detail-section review-strategy-card">
          <h2>{copy.paper.strategyComparison}</h2>
          <StrategyStatsTable
            copy={copy}
            onSelectVersion={onSelectStrategyVersion}
            selectedVersion={activeStrategyVersion}
            stats={strategyStats}
          />
        </section>
      </section>
    </>
  );
}

function StrategyStatsTable({
  copy,
  onSelectVersion,
  selectedVersion,
  stats,
}: {
  copy: Copy;
  onSelectVersion: (version: string) => void;
  selectedVersion: string | null;
  stats: PaperStrategyStats[];
}) {
  if (stats.length === 0) {
    return <p className="muted">{copy.review.noStrategyStats}</p>;
  }

  return (
    <div className="review-strategy-table-wrap" data-testid="paper-strategy-stats">
      <table className="review-strategy-table">
        <thead>
          <tr>
            <th>{copy.paper.strategyVersion}</th>
            <th>{copy.paper.strategyVersionNumber}</th>
            <th>{copy.paper.strategyRuntime}</th>
            <th>{copy.paper.closedPositions}</th>
            <th>{copy.paper.winRate}</th>
            <th>{copy.paper.realized}</th>
            <th>{copy.paper.averagePositionPnl}</th>
            <th>{copy.paper.profitFactor}</th>
            <th>{copy.paper.totalFees}</th>
          </tr>
        </thead>
        <tbody>
          {stats.map((item) => (
            <tr
              className={selectedVersion === item.strategy_version ? "active" : ""}
              key={`${item.strategy_name}-${item.strategy_version}`}
              onClick={() => onSelectVersion(item.strategy_version)}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  onSelectVersion(item.strategy_version);
                }
              }}
              tabIndex={0}
            >
              <td>{item.strategy_name}</td>
              <td>{item.strategy_version}</td>
              <td>{formatNullableDuration(item.running_duration_ms)}</td>
              <td>{item.closed_position_count}</td>
              <td>{formatNullablePct(item.win_rate)}</td>
              <td className={pnlClass(item.realized_pnl)}>{formatSignedUsdt(item.realized_pnl)}</td>
              <td className={pnlClass(item.average_position_pnl ?? 0)}>
                {formatNullableSignedUsdt(item.average_position_pnl)}
              </td>
              <td>{formatNullableRatio(item.profit_factor)}</td>
              <td>{formatUsdt(item.total_fees)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function StrategyCurvePanel({
  copy,
  curve,
  version,
}: {
  copy: Copy;
  curve: StrategyCurve;
  version: string;
}) {
  return (
    <section className="paper-strategy-curve" data-testid="paper-strategy-curve">
      <div className="paper-card-header">
        <div>
          <h3>{copy.paper.strategyCurve}</h3>
          <p>{version}</p>
        </div>
      </div>
      {curve.points.length === 0 ? (
        <p className="muted">{copy.paper.strategyCurveEmpty}</p>
      ) : (
        <>
          <div className="paper-strategy-curve-metrics">
            <Metric
              label={copy.paper.cumulativeRealized}
              value={formatSignedUsdt(curve.finalPnl)}
            />
            <Metric label={copy.paper.maxDrawdown} value={formatSignedUsdt(curve.maxDrawdown)} />
            <Metric
              label={copy.paper.bestCumulative}
              value={formatSignedUsdt(curve.bestCumulative)}
            />
            <Metric
              label={copy.paper.worstCumulative}
              value={formatSignedUsdt(curve.worstCumulative)}
            />
            <Metric label={copy.paper.closedPositions} value={String(curve.points.length)} />
          </div>
          <StrategyCurveSvg copy={copy} curve={curve} version={version} />
        </>
      )}
    </section>
  );
}

function StrategyCurveSvg({
  copy,
  curve,
  version,
}: {
  copy: Copy;
  curve: StrategyCurve;
  version: string;
}) {
  const width = 720;
  const height = 220;
  const paddingX = 36;
  const paddingY = 24;
  const values = [0, ...curve.points.map((point) => point.cumulativePnl)];
  const minValue = Math.min(...values);
  const maxValue = Math.max(...values);
  const span = Math.max(maxValue - minValue, 1);
  const xFor = (index: number) =>
    paddingX +
    (curve.points.length <= 1
      ? (width - paddingX * 2) / 2
      : (index / (curve.points.length - 1)) * (width - paddingX * 2));
  const yFor = (value: number) =>
    height - paddingY - ((value - minValue) / span) * (height - paddingY * 2);
  const linePoints = curve.points
    .map((point, index) => `${xFor(index).toFixed(2)},${yFor(point.cumulativePnl).toFixed(2)}`)
    .join(" ");
  const zeroY = yFor(0);

  return (
    <svg
      aria-label={`${version} ${copy.paper.strategyCurve}`}
      className="paper-strategy-curve-svg"
      role="img"
      viewBox={`0 0 ${width} ${height}`}
    >
      <line className="paper-strategy-curve-grid" x1={paddingX} x2={width - paddingX} y1={paddingY} y2={paddingY} />
      <line
        className="paper-strategy-curve-grid"
        x1={paddingX}
        x2={width - paddingX}
        y1={height - paddingY}
        y2={height - paddingY}
      />
      <line
        className="paper-strategy-curve-zero"
        x1={paddingX}
        x2={width - paddingX}
        y1={zeroY}
        y2={zeroY}
      />
      <polyline className="paper-strategy-curve-line" fill="none" points={linePoints} />
      {curve.points.map((point, index) => (
        <circle
          className={point.cumulativePnl >= 0 ? "paper-strategy-curve-dot positive" : "paper-strategy-curve-dot negative"}
          cx={xFor(index)}
          cy={yFor(point.cumulativePnl)}
          key={`${point.closedAtMs}-${index}`}
          r={3}
        />
      ))}
      <text className="paper-strategy-curve-label" x={paddingX} y={paddingY - 8}>
        {formatSignedUsdt(maxValue)}
      </text>
      <text className="paper-strategy-curve-label" x={paddingX} y={height - 6}>
        {formatSignedUsdt(minValue)}
      </text>
    </svg>
  );
}

function HistorySection({
  copy,
  filteredHistory,
  historyEndDate,
  historyFilter,
  historyStartDate,
  historySymbolQuery,
  historyVersion,
  historyVersionOptions,
  onEndDateChange,
  onFilterChange,
  onStartDateChange,
  onSymbolQueryChange,
  onVersionChange,
  totalHistory,
}: {
  copy: Copy;
  filteredHistory: PaperClosedPositionSnapshot[];
  historyEndDate: string;
  historyFilter: HistoryFilter;
  historyStartDate: string;
  historySymbolQuery: string;
  historyVersion: string;
  historyVersionOptions: string[];
  onEndDateChange: (value: string) => void;
  onFilterChange: (value: HistoryFilter) => void;
  onStartDateChange: (value: string) => void;
  onSymbolQueryChange: (value: string) => void;
  onVersionChange: (value: string) => void;
  totalHistory: number;
}) {
  return (
    <section className="detail-section review-history-panel">
      <header className="panel-heading compact">
        <div>
          <h2>{copy.paper.positionHistory}</h2>
          <p>
            {filteredHistory.length} / {totalHistory}
          </p>
        </div>
      </header>
      <div className="review-history-filter-grid">
        <label>
          <span>{copy.paper.historySymbolSearch}</span>
          <input
            aria-label={copy.paper.historySymbolSearch}
            onChange={(event) => onSymbolQueryChange(event.target.value)}
            placeholder="BTC / ETH / LAB"
            type="search"
            value={historySymbolQuery}
          />
        </label>
        <label>
          <span>{copy.paper.historyStartDate}</span>
          <input
            aria-label={copy.paper.historyStartDate}
            onChange={(event) => onStartDateChange(event.target.value)}
            step="1"
            type="datetime-local"
            value={historyStartDate}
          />
        </label>
        <label>
          <span>{copy.paper.historyEndDate}</span>
          <input
            aria-label={copy.paper.historyEndDate}
            onChange={(event) => onEndDateChange(event.target.value)}
            step="1"
            type="datetime-local"
            value={historyEndDate}
          />
        </label>
        <label>
          <span>{copy.paper.historyVersion}</span>
          <select
            aria-label={copy.paper.historyVersion}
            onChange={(event) => onVersionChange(event.target.value)}
            value={historyVersion}
          >
            <option value="all">{copy.paper.allVersions}</option>
            {historyVersionOptions.map((version) => (
              <option key={version} value={version}>
                {version}
              </option>
            ))}
          </select>
        </label>
      </div>
      <div className="page-local-tabs review-history-filters" role="group" aria-label={copy.paper.positionHistory}>
        {(["all", "long", "short", "profit", "loss"] as HistoryFilter[]).map((value) => (
          <button
            className={historyFilter === value ? "active" : ""}
            key={value}
            onClick={() => onFilterChange(value)}
            type="button"
          >
            {copy.paper.filters[value]}
          </button>
        ))}
      </div>
      {filteredHistory.length === 0 ? (
        <p className="muted panel-empty">{copy.paper.noPositionHistory}</p>
      ) : (
        <ul className="review-history-list">
          {filteredHistory.map((position) => (
            <li key={position.id}>
              <div className="review-history-heading">
                <strong>{position.inst_id}</strong>
                <span>{copy.directions[position.side]}</span>
                <em className={pnlClass(position.realized_pnl)}>
                  {formatSignedUsdt(position.realized_pnl)} / {formatPct(position.pnl_pct)}
                </em>
              </div>
              <dl>
                <Metric label={copy.paper.entry} value={formatPrice(position.entry_price)} />
                <Metric label={copy.paper.exit} value={formatPrice(position.exit_price)} />
                <Metric label={copy.paper.margin} value={formatUsdt(position.margin)} />
                <Metric label={copy.paper.leverage} value={`${position.leverage.toFixed(0)}x`} />
                <Metric label={copy.paper.fee} value={formatUsdt(position.fees)} />
                <Metric label={copy.paper.duration} value={formatDuration(position.duration_ms)} />
                <Metric label={copy.paper.openedAt} value={formatTimestamp(position.opened_at_ms)} />
                <Metric label={copy.paper.closedAt} value={formatTimestamp(position.closed_at_ms)} />
                <Metric label={copy.paper.strategyVersion} value={strategyLabel(position)} />
                <Metric label={copy.paper.openReason} value={position.reason || "-"} />
                <Metric label={copy.paper.closeReason} value={position.close_reason || "-"} />
              </dl>
              <TagChips tags={[...(position.open_tags ?? []), ...(position.close_tags ?? []), ...(position.tags ?? [])]} />
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function TradeRecordsSection({ copy, trades }: { copy: Copy; trades: PaperTrade[] }) {
  return (
    <section className="table-panel review-trades-panel">
      <header className="panel-heading">
        <div>
          <h2>{copy.review.tradeRecords}</h2>
          <p>{trades.length} {copy.paper.history}</p>
        </div>
      </header>
      {trades.length === 0 ? (
        <p className="muted panel-empty">{copy.paper.noTrades}</p>
      ) : (
        <table className="review-trade-table">
          <thead>
            <tr>
              <th>{copy.table.symbol}</th>
              <th>{copy.paper.tradeActions.open}</th>
              <th>{copy.paper.side}</th>
              <th>{copy.paper.strategyVersion}</th>
              <th>{copy.table.price}</th>
              <th>{copy.paper.realized}</th>
              <th>{copy.status.lastScan}</th>
            </tr>
          </thead>
          <tbody>
            {trades.map((trade) => (
              <tr key={trade.id}>
                <td>{trade.inst_id}</td>
                <td>{copy.paper.tradeActions[trade.action]}</td>
                <td>{copy.directions[trade.side]}</td>
                <td>{tradeStrategyLabel(trade)}</td>
                <td>{formatPrice(trade.price)}</td>
                <td className={pnlClass(trade.realized_pnl)}>
                  {formatSignedUsdt(trade.realized_pnl)}
                </td>
                <td>{formatTimestamp(trade.ts_ms)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt>{label}</dt>
      <dd>{value}</dd>
    </div>
  );
}

function MetricCard({
  label,
  tone,
  value,
}: {
  label: string;
  tone?: string;
  value: string;
}) {
  return (
    <div className="metric-card">
      <span>{label}</span>
      <strong className={tone ?? ""}>{value}</strong>
    </div>
  );
}

function TagChips({ tags }: { tags: TradeTag[] }) {
  if (tags.length === 0) {
    return null;
  }
  return (
    <span className="tag-chip-row">
      {tags.map((tag, index) => (
        <span
          className={tag.score_impact < 0 ? "tag-chip tag-chip-warning" : "tag-chip"}
          key={`${tag.kind}-${tag.ts_ms}-${index}`}
          title={tag.reason}
        >
          {tag.label || tag.kind.replace(/_/g, " ")}
        </span>
      ))}
    </span>
  );
}

function RealizedCurve({ points }: { points: Array<{ id: number; value: number }> }) {
  if (points.length === 0) {
    return <div className="review-empty-chart" />;
  }

  const values = points.map((point) => point.value);
  const min = Math.min(0, ...values);
  const max = Math.max(0, ...values);
  const span = max - min || 1;
  const path = points
    .map((point, index) => {
      const x = points.length === 1 ? 100 : (index / (points.length - 1)) * 100;
      const y = 100 - ((point.value - min) / span) * 100;
      return `${index === 0 ? "M" : "L"} ${x.toFixed(2)} ${y.toFixed(2)}`;
    })
    .join(" ");

  return (
    <svg className="review-curve" role="img" aria-label="Realized PnL curve" viewBox="0 0 100 100" preserveAspectRatio="none">
      <path d="M 0 100 L 100 100" className="review-curve-axis" />
      <path d={path} className="review-curve-line" />
    </svg>
  );
}

function matchesHistoryFilter(position: PaperClosedPositionSnapshot, filter: HistoryFilter): boolean {
  if (filter === "long" || filter === "short") {
    return position.side === filter;
  }
  if (filter === "profit") {
    return position.realized_pnl > 0;
  }
  if (filter === "loss") {
    return position.realized_pnl < 0;
  }
  return true;
}

function matchesHistoryDetailFilters(
  position: PaperClosedPositionSnapshot,
  filters: { endDate: string; startDate: string; symbolQuery: string; version: string },
): boolean {
  const symbolQuery = filters.symbolQuery.trim().toLowerCase();
  if (symbolQuery && !position.inst_id.toLowerCase().includes(symbolQuery)) {
    return false;
  }
  if (filters.version !== "all" && strategyVersion(position) !== filters.version) {
    return false;
  }
  const startMs = dateStartMs(filters.startDate);
  if (startMs !== null && position.closed_at_ms < startMs) {
    return false;
  }
  const endMs = dateEndMs(filters.endDate);
  if (endMs !== null && position.closed_at_ms > endMs) {
    return false;
  }
  return true;
}

function buildStrategyStats(positions: PaperClosedPositionSnapshot[]): PaperStrategyStats[] {
  const groups = new Map<string, PaperClosedPositionSnapshot[]>();
  positions.forEach((position) => {
    const key = `${position.strategy_name || position.source || "unknown"}\u0000${strategyVersion(position)}`;
    groups.set(key, [...(groups.get(key) ?? []), position]);
  });

  return Array.from(groups.entries()).map(([key, group]) => {
    const [strategyName, strategyVersionValue] = key.split("\u0000");
    const winners = group.filter((position) => position.realized_pnl > 0);
    const losers = group.filter((position) => position.realized_pnl < 0);
    const grossProfit = sum(winners.map((position) => position.realized_pnl));
    const grossLossAbs = Math.abs(sum(losers.map((position) => position.realized_pnl)));
    return {
      average_holding_duration_ms: averageNumber(group.map((position) => position.duration_ms)),
      average_position_pnl: averageNumber(group.map((position) => position.realized_pnl)),
      closed_position_count: group.length,
      first_trade_ts_ms: Math.min(...group.map((position) => position.opened_at_ms)),
      largest_losing_pnl: minNumber(losers.map((position) => position.realized_pnl)),
      largest_winning_pnl: maxNumber(winners.map((position) => position.realized_pnl)),
      last_trade_ts_ms: Math.max(...group.map((position) => position.closed_at_ms)),
      losing_closed_position_count: losers.length,
      profit_factor: grossProfit > 0 && grossLossAbs > 0 ? grossProfit / grossLossAbs : null,
      realized_pnl: sum(group.map((position) => position.realized_pnl)),
      running_duration_ms:
        Math.max(...group.map((position) => position.closed_at_ms)) -
        Math.min(...group.map((position) => position.opened_at_ms)),
      strategy_name: strategyName,
      strategy_version: strategyVersionValue,
      total_fees: sum(group.map((position) => position.fees)),
      total_trades: group.length * 2,
      win_rate: group.length === 0 ? null : winners.length / group.length,
      winning_closed_position_count: winners.length,
    };
  });
}

function buildStrategyCurve(
  positions: PaperClosedPositionSnapshot[],
  version: string,
): StrategyCurve {
  const matched = positions
    .filter((position) => strategyVersion(position) === version)
    .slice()
    .sort((left, right) => left.closed_at_ms - right.closed_at_ms || left.id - right.id);
  let cumulativePnl = 0;
  let peak = 0;
  let maxDrawdown = 0;
  let bestCumulative = 0;
  let worstCumulative = 0;
  const points = matched.map((position) => {
    cumulativePnl += position.realized_pnl;
    peak = Math.max(peak, cumulativePnl);
    maxDrawdown = Math.min(maxDrawdown, cumulativePnl - peak);
    bestCumulative = Math.max(bestCumulative, cumulativePnl);
    worstCumulative = Math.min(worstCumulative, cumulativePnl);
    return {
      closedAtMs: position.closed_at_ms,
      cumulativePnl,
    };
  });

  return {
    bestCumulative,
    finalPnl: cumulativePnl,
    maxDrawdown,
    points,
    worstCumulative,
  };
}

function strategyVersion(position: Pick<PaperClosedPositionSnapshot, "strategy_version" | "source">): string {
  return position.strategy_version || position.source || "unknown";
}

function strategyLabel(position: PaperClosedPositionSnapshot): string {
  return `${position.strategy_name || position.source || "unknown"} ${strategyVersion(position)}`;
}

function tradeStrategyLabel(trade: {
  source?: string;
  strategy_name?: string;
  strategy_version?: string;
}) {
  const name = trade.strategy_name || trade.source || "-";
  const version = trade.strategy_version || trade.source || "";
  return version ? `${name} ${version}` : name;
}

function strategyVersionOptions(
  positions: PaperClosedPositionSnapshot[],
  stats: PaperStrategyStats[],
  trades: PaperTrade[],
): string[] {
  return Array.from(
    new Set([
      ...positions.map(strategyVersion),
      ...stats.map((item) => item.strategy_version),
      ...trades.map((trade) => trade.strategy_version || trade.source || "unknown"),
    ]),
  )
    .filter((version) => version.trim().length > 0)
    .sort();
}

function dateStartMs(value: string): number | null {
  if (!value) {
    return null;
  }
  const parsed = Date.parse(value.includes("T") ? value : `${value}T00:00:00`);
  return Number.isFinite(parsed) ? parsed : null;
}

function dateEndMs(value: string): number | null {
  if (!value) {
    return null;
  }
  const parsed = Date.parse(value.includes("T") ? value : `${value}T23:59:59.999`);
  return Number.isFinite(parsed) ? parsed : null;
}

function formatNullablePct(value: number | null): string {
  return value === null ? "-" : formatPct(value);
}

function formatNullableSignedUsdt(value: number | null): string {
  return value === null ? "-" : formatSignedUsdt(value);
}

function formatNullableRatio(value: number | null): string {
  return value === null ? "-" : value.toFixed(2);
}

function formatNullableDuration(durationMs: number | null): string {
  return durationMs === null ? "-" : formatDuration(durationMs);
}

function formatDuration(durationMs: number): string {
  const totalMinutes = Math.max(0, Math.round(durationMs / 60000));
  const days = Math.floor(totalMinutes / 1440);
  const hours = Math.floor((totalMinutes % 1440) / 60);
  const minutes = totalMinutes % 60;
  if (days > 0) {
    return `${days}d ${hours}h`;
  }
  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  }
  return `${minutes}m`;
}

function sum(values: number[]): number {
  return values.reduce((total, value) => total + value, 0);
}

function averageNumber(values: number[]): number | null {
  return values.length === 0 ? null : sum(values) / values.length;
}

function maxNumber(values: number[]): number | null {
  return values.length === 0 ? null : Math.max(...values);
}

function minNumber(values: number[]): number | null {
  return values.length === 0 ? null : Math.min(...values);
}
