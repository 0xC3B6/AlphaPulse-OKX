import { useMemo, useState } from "react";
import {
  Area,
  AreaChart,
  CartesianGrid,
  ReferenceLine,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import type { Copy } from "./i18n";
import type {
  PaperAccountSnapshot,
  PaperClosedPositionSnapshot,
  PaperStrategyStats,
  PaperTrade,
  StrategyCenterSnapshot,
  StrategyEquitySnapshot,
  TradeTagLike,
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
import { PaginationControls } from "./PaginationControls";

type HistoryFilter = "all" | "long" | "short" | "profit" | "loss";
type ReviewSection = "overview" | "strategy" | "history" | "trades";
type StrategyCurveRange = "7d" | "30d" | "90d" | "all";
type StrategyCurveKind = "realized" | "equity";
type SignalConfidence = "high" | "med" | "low";
type SignalRecommendationKey = "continueExecution" | "optimizeEntry" | "pauseOrReduce";

interface HistorySearchFilters {
  endDate: string;
  filter: HistoryFilter;
  startDate: string;
  symbolQuery: string;
  version: string;
}

interface StrategyCurvePoint {
  closedAtMs: number;
  cumulativePnl: number;
  instId: string;
  realizedPnl: number;
  tradeIndex: number;
}

interface StrategyCurve {
  kind: StrategyCurveKind;
  points: StrategyCurvePoint[];
  finalPnl: number;
  latestEquity?: number;
  maxDrawdown: number;
  bestCumulative: number;
  worstCumulative: number;
}

interface StrategyCurveChartPoint {
  closedAtMs: number;
  closedCount: number;
  cumulativePnl: number;
  negativePnl: number | null;
  periodPnl: number;
  positivePnl: number | null;
  synthetic?: boolean;
}

interface StrategyAxisScale {
  label: string;
  stepMs: number;
  tickFormat: "date" | "dateTime" | "month" | "time";
}

interface StrategySignalAttribution {
  confidence: SignalConfidence;
  maxLoss: number | null;
  netPnl: number;
  profitFactor: number | null;
  recommendationKey: SignalRecommendationKey;
  sampleCount: number;
  signal: string;
  winRate: number | null;
}

type ReviewStrategyStats = PaperStrategyStats & {
  current_equity?: number;
  open_position_count?: number;
  return_pct?: number | null;
  run_status?: string;
  unrealized_pnl?: number;
};

const MINUTE_MS = 60_000;
const HOUR_MS = 60 * MINUTE_MS;
const DAY_MS = 24 * HOUR_MS;
const HISTORY_PAGE_SIZE = 10;
const DEFAULT_HISTORY_SEARCH_FILTERS: HistorySearchFilters = {
  endDate: "",
  filter: "all",
  startDate: "",
  symbolQuery: "",
  version: "all",
};

export function ReviewPage({
  copy,
  paper,
  strategyCenter,
}: {
  copy: Copy;
  paper: PaperAccountSnapshot;
  strategyCenter?: StrategyCenterSnapshot;
}) {
  const [activeSection, setActiveSection] = useState<ReviewSection>("overview");
  const [historyDraftFilters, setHistoryDraftFilters] = useState<HistorySearchFilters>(
    DEFAULT_HISTORY_SEARCH_FILTERS,
  );
  const [historyAppliedFilters, setHistoryAppliedFilters] = useState<HistorySearchFilters>(
    DEFAULT_HISTORY_SEARCH_FILTERS,
  );
  const [historyPage, setHistoryPage] = useState(0);
  const [selectedStrategyVersion, setSelectedStrategyVersion] = useState<string | null>(null);

  const summary = summarizePaperReview(paper);
  const positionHistory = paper.position_history ?? [];
  const strategyStats = useMemo(
    () => {
      const closedStats =
        paper.strategy_stats && paper.strategy_stats.length > 0
          ? paper.strategy_stats
          : buildStrategyStats(positionHistory);
      return sortStrategyStats(mergeLiveStrategyStats(closedStats, buildLiveStrategyStats(strategyCenter)));
    },
    [paper.strategy_stats, positionHistory, strategyCenter],
  );
  const activeStrategyVersion =
    selectedStrategyVersion ?? strategyStats[0]?.strategy_version ?? null;
  const activeStrategyCurve =
    activeStrategyVersion === null
      ? null
      : buildStrategyCurve(
          positionHistory,
          activeStrategyVersion,
          strategyCenter?.versions.find(
            (version) => version.version.version_code === activeStrategyVersion,
          ),
        );
  const activeSignalAttribution = useMemo(
    () =>
      activeStrategyVersion === null
        ? []
        : buildSignalAttribution(positionHistory, activeStrategyVersion),
    [activeStrategyVersion, positionHistory],
  );
  const historyVersionOptions = useMemo(
    () => strategyVersionOptions(positionHistory, strategyStats, paper.trades),
    [paper.trades, positionHistory, strategyStats],
  );
  const filteredHistory = useMemo(
    () =>
      positionHistory.filter(
        (position) =>
          matchesHistoryFilter(position, historyAppliedFilters.filter) &&
          matchesHistoryDetailFilters(position, historyAppliedFilters),
      ),
    [historyAppliedFilters, positionHistory],
  );
  const historyPageCount = Math.max(1, Math.ceil(filteredHistory.length / HISTORY_PAGE_SIZE));
  const safeHistoryPage = Math.min(historyPage, historyPageCount - 1);
  const visibleHistory = filteredHistory.slice(
    safeHistoryPage * HISTORY_PAGE_SIZE,
    (safeHistoryPage + 1) * HISTORY_PAGE_SIZE,
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
            initialBalance={paper.initial_balance}
            onSelectVersion={setSelectedStrategyVersion}
            selectedVersion={activeStrategyVersion}
            stats={strategyStats}
          />
          {activeStrategyVersion === null || activeStrategyCurve === null ? null : (
            <StrategyCurvePanel
              copy={copy}
              curve={activeStrategyCurve}
              initialBalance={paper.initial_balance}
              version={activeStrategyVersion}
            />
          )}
          {activeStrategyVersion === null ? null : (
            <StrategyDoctorSection
              copy={copy}
              rows={activeSignalAttribution}
              version={activeStrategyVersion}
            />
          )}
        </section>
      ) : null}

      {activeSection === "history" ? (
        <HistorySection
          copy={copy}
          draftFilters={historyDraftFilters}
          filteredHistory={filteredHistory}
          historyVersionOptions={historyVersionOptions}
          onApplyFilters={() => {
            setHistoryAppliedFilters(historyDraftFilters);
            setHistoryPage(0);
          }}
          onDraftFiltersChange={setHistoryDraftFilters}
          onPageChange={setHistoryPage}
          onResetFilters={() => {
            setHistoryDraftFilters(DEFAULT_HISTORY_SEARCH_FILTERS);
            setHistoryAppliedFilters(DEFAULT_HISTORY_SEARCH_FILTERS);
            setHistoryPage(0);
          }}
          page={safeHistoryPage}
          pageCount={historyPageCount}
          openPositionCount={paper.positions.length}
          totalHistory={positionHistory.length}
          visibleHistory={visibleHistory}
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
  strategyStats: ReviewStrategyStats[];
  summary: ReturnType<typeof summarizePaperReview>;
}) {
  const returnRate = accountReturnRate(paper);

  return (
    <>
      <section className="page-metric-grid review-summary">
        <MetricCard label={copy.paper.initialBalance} value={formatUsdt(paper.initial_balance)} />
        <MetricCard label={copy.paper.equity} value={formatUsdt(paper.equity)} />
        <MetricCard label={copy.paper.available} value={formatUsdt(paper.available_balance)} />
        <MetricCard label={copy.paper.usedMargin} value={formatUsdt(paper.used_margin)} />
        <MetricCard
          label={copy.paper.returnRate}
          tone={pnlClass(returnRate ?? 0)}
          value={formatNullablePct(returnRate)}
        />
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
            initialBalance={paper.initial_balance}
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
  initialBalance,
  onSelectVersion,
  selectedVersion,
  stats,
}: {
  copy: Copy;
  initialBalance: number;
  onSelectVersion: (version: string) => void;
  selectedVersion: string | null;
  stats: ReviewStrategyStats[];
}) {
  if (stats.length === 0) {
    return <p className="muted">{copy.review.noStrategyStats}</p>;
  }

  const activeVersion = stats[0]?.strategy_version ?? null;

  return (
    <div className="review-strategy-table-wrap" data-testid="paper-strategy-stats">
      <table className="review-strategy-table">
        <thead>
          <tr>
            <th>{copy.paper.strategyVersionShort}</th>
            <th>{copy.paper.strategyRuntime}</th>
            <th>{copy.paper.equity}</th>
            <th>{copy.paper.strategyTrades}</th>
            <th>{copy.paper.winRate}</th>
            <th>{copy.paper.realized}</th>
            <th>{copy.paper.returnRate}</th>
            <th>{copy.paper.averagePnlShort}</th>
            <th>{copy.paper.averageHoldingShort}</th>
            <th>{copy.paper.profitFactor}</th>
            <th>{copy.paper.totalFees}</th>
            <th>{copy.paper.status}</th>
          </tr>
        </thead>
        <tbody>
          {stats.map((item) => {
            const isActiveVersion =
              item.run_status === undefined
                ? activeVersion === item.strategy_version
                : item.run_status === "running";
            const returnRate = item.return_pct ?? strategyReturnRate(item.realized_pnl, initialBalance);
            return (
            <tr
              aria-label={`${item.strategy_name} ${item.strategy_version}`}
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
              <td>
                <span className="sr-only">{item.strategy_name} </span>
                <span className="strategy-version-pill">{item.strategy_version}</span>
              </td>
              <td>{formatNullableDuration(item.running_duration_ms)}</td>
              <td>{item.current_equity === undefined ? "-" : formatUsdt(item.current_equity)}</td>
              <td>
                {item.closed_position_count}
                {item.open_position_count === undefined ? null : (
                  <small className="muted"> / {item.open_position_count} open</small>
                )}
              </td>
              <td>{formatNullablePct(item.win_rate)}</td>
              <td className={pnlClass(item.realized_pnl)}>
                {formatSignedUsdt(item.realized_pnl)}
                {item.unrealized_pnl === undefined ? null : (
                  <small className={pnlClass(item.unrealized_pnl)}> {formatSignedUsdt(item.unrealized_pnl)} UPNL</small>
                )}
              </td>
              <td className={pnlClass(returnRate ?? 0)}>
                {formatNullablePct(returnRate)}
              </td>
              <td className={pnlClass(item.average_position_pnl ?? 0)}>
                {formatNullableSignedUsdt(item.average_position_pnl)}
              </td>
              <td>{formatNullableDuration(item.average_holding_duration_ms)}</td>
              <td>{formatNullableRatio(item.profit_factor)}</td>
              <td className="negative">{formatSignedUsdt(-item.total_fees)}</td>
              <td>
                <span className={isActiveVersion ? "strategy-status-pill active" : "strategy-status-pill"}>
                  {isActiveVersion ? copy.paper.active : copy.paper.closed}
                </span>
              </td>
            </tr>
          );
          })}
        </tbody>
      </table>
    </div>
  );
}

function StrategyCurvePanel({
  copy,
  curve,
  initialBalance,
  version,
}: {
  copy: Copy;
  curve: StrategyCurve;
  initialBalance: number;
  version: string;
}) {
  const [range, setRange] = useState<StrategyCurveRange>("all");
  const chartCurve = useMemo(() => filterStrategyCurve(curve, range), [curve, range]);
  const curveTitle = curve.kind === "equity" ? copy.paper.equityCurve : copy.paper.strategyCurve;
  const rangeOptions: Array<[StrategyCurveRange, string]> = [
    ["7d", "7D"],
    ["30d", "30D"],
    ["90d", "90D"],
    ["all", "ALL"],
  ];

  return (
    <section className="paper-strategy-curve" data-testid="paper-strategy-curve">
      <div className="paper-card-header paper-strategy-curve-header">
        <div>
          <h3>{version} {curveTitle}</h3>
        </div>
        <div className="paper-strategy-range-tabs" role="group" aria-label={curveTitle}>
          {rangeOptions.map(([value, label]) => (
            <button
              aria-pressed={range === value}
              className={range === value ? "active" : ""}
              key={value}
              onClick={() => setRange(value)}
              type="button"
            >
              {label}
            </button>
          ))}
        </div>
      </div>
      {curve.points.length === 0 ? (
        <p className="muted">{copy.paper.strategyCurveEmpty}</p>
      ) : (
        <>
          <div className="paper-strategy-curve-metrics">
            <Metric
              label={curve.kind === "equity" ? copy.paper.equityChange : copy.paper.cumulativeRealized}
              value={formatSignedUsdt(curve.finalPnl)}
            />
            <Metric
              label={copy.paper.returnRate}
              value={formatNullablePct(strategyReturnRate(curve.finalPnl, initialBalance))}
            />
            <Metric label={copy.paper.maxDrawdown} value={formatSignedUsdt(curve.maxDrawdown)} />
            {curve.kind === "equity" ? (
              <Metric
                label={copy.paper.latestEquity}
                value={formatUsdt(curve.latestEquity ?? initialBalance + curve.finalPnl)}
              />
            ) : null}
            <Metric
              label={copy.paper.bestCumulative}
              value={formatSignedUsdt(curve.bestCumulative)}
            />
            <Metric
              label={copy.paper.worstCumulative}
              value={formatSignedUsdt(curve.worstCumulative)}
            />
            <Metric
              label={curve.kind === "equity" ? copy.paper.equityPoints : copy.paper.closedPositions}
              value={String(curve.points.length)}
            />
          </div>
          <StrategyCurveChart copy={copy} curve={chartCurve} version={version} />
        </>
      )}
    </section>
  );
}

function StrategyDoctorSection({
  copy,
  rows,
  version,
}: {
  copy: Copy;
  rows: StrategySignalAttribution[];
  version: string;
}) {
  return (
    <section className="paper-strategy-doctor" data-testid="paper-strategy-doctor">
      <header className="paper-strategy-doctor-header">
        <h3>
          {copy.paper.signalAttribution} · {copy.paper.strategyDoctor}
          <span>{version}</span>
        </h3>
      </header>
      {rows.length === 0 ? (
        <p className="muted panel-empty">{copy.paper.noSignalAttribution}</p>
      ) : (
        <div className="paper-strategy-doctor-table-wrap">
          <table className="paper-strategy-doctor-table">
            <thead>
              <tr>
                <th>{copy.paper.primarySignal}</th>
                <th>{copy.paper.sampleCount}</th>
                <th>{copy.paper.netPnl}</th>
                <th>{copy.paper.winRate}</th>
                <th>{copy.paper.profitFactor}</th>
                <th>{copy.paper.maxLoss}</th>
                <th>{copy.paper.confidence}</th>
                <th>{copy.paper.recommendation}</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((row) => (
                <tr key={row.signal}>
                  <td>{row.signal}</td>
                  <td>{row.sampleCount}</td>
                  <td className={pnlClass(row.netPnl)}>{formatSignedUsdt(row.netPnl)}</td>
                  <td className={pnlClass((row.winRate ?? 0) - 0.5)}>
                    {formatNullablePct(row.winRate)}
                  </td>
                  <td>{formatNullableRatio(row.profitFactor)}</td>
                  <td className={pnlClass(row.maxLoss ?? 0)}>
                    {formatNullableSignedUsdt(row.maxLoss)}
                  </td>
                  <td>
                    <span className={`confidence-pill ${row.confidence}`}>
                      {row.confidence}
                    </span>
                  </td>
                  <td>{copy.paper[row.recommendationKey]}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}

function StrategyCurveChart({
  copy,
  curve,
  version,
}: {
  copy: Copy;
  curve: StrategyCurve;
  version: string;
}) {
  const rawStartMs = curve.points[0].closedAtMs;
  const rawEndMs = curve.points[curve.points.length - 1].closedAtMs;
  const axisScale = chooseStrategyAxisScale(rawStartMs, rawEndMs);
  const axisDomain = [rawStartMs - axisScale.stepMs / 2, rawEndMs + axisScale.stepMs / 2];
  const axisTicks = buildStrategyAxisTicks(rawStartMs, rawEndMs, axisScale);
  const data = buildStrategyCurveChartData(curve.points, axisScale.stepMs);
  const shouldAnimate = import.meta.env.MODE !== "test";
  const chart = (
    <AreaChart
      data={data}
      key={`${version}-${rawStartMs}-${rawEndMs}-${axisScale.stepMs}`}
      margin={{ bottom: 8, left: 0, right: 8, top: 16 }}
      {...(import.meta.env.MODE === "test" ? { height: 300, width: 920 } : {})}
    >
      <defs>
        <linearGradient id="strategyPositiveFill" x1="0" x2="0" y1="0" y2="1">
          <stop offset="5%" stopColor="#12d99c" stopOpacity={0.28} />
          <stop offset="95%" stopColor="#12d99c" stopOpacity={0} />
        </linearGradient>
        <linearGradient id="strategyNegativeFill" x1="0" x2="0" y1="0" y2="1">
          <stop offset="5%" stopColor="#ff4d6d" stopOpacity={0.3} />
          <stop offset="95%" stopColor="#ff4d6d" stopOpacity={0} />
        </linearGradient>
      </defs>
      <CartesianGrid stroke="rgba(74, 98, 120, 0.2)" strokeDasharray="3 3" vertical={false} />
      <XAxis
        allowDataOverflow
        dataKey="closedAtMs"
        domain={axisDomain}
        interval={0}
        scale="time"
        tick={{ fill: "#607b96", fontSize: 10, fontWeight: 700 }}
        tickFormatter={(value) => formatStrategyAxisTick(Number(value), axisScale.tickFormat)}
        tickLine={false}
        ticks={axisTicks}
        type="number"
      />
      <YAxis
        tick={{ fill: "#607b96", fontSize: 10, fontWeight: 700 }}
        tickFormatter={(value) => formatCompactUsdt(Number(value))}
        tickLine={false}
        width={76}
      />
      <Tooltip
        content={<StrategyCurveTooltip copy={copy} curveKind={curve.kind} />}
        cursor={{ stroke: "rgba(18, 217, 156, 0.42)", strokeDasharray: "4 4" }}
        isAnimationActive={false}
      />
      <ReferenceLine
        stroke="rgba(148, 174, 196, 0.45)"
        strokeDasharray="6 6"
        strokeWidth={1.2}
        y={0}
      />
      <Area
        activeDot={{ fill: "#12d99c", r: 5, stroke: "#071017", strokeWidth: 2 }}
        connectNulls={false}
        dataKey="positivePnl"
        dot={false}
        fill="url(#strategyPositiveFill)"
        isAnimationActive={shouldAnimate}
        animationDuration={1200}
        animationEasing="ease-out"
        name="盈利"
        stroke="#12d99c"
        strokeWidth={2.6}
        type="linear"
      />
      <Area
        activeDot={{ fill: "#ff4d6d", r: 5, stroke: "#071017", strokeWidth: 2 }}
        connectNulls={false}
        dataKey="negativePnl"
        dot={false}
        fill="url(#strategyNegativeFill)"
        isAnimationActive={shouldAnimate}
        animationBegin={180}
        animationDuration={1200}
        animationEasing="ease-out"
        name="亏损"
        stroke="#ff4d6d"
        strokeWidth={2.6}
        type="linear"
      />
    </AreaChart>
  );

  return (
    <div
      aria-label={`${version} ${curve.kind === "equity" ? copy.paper.equityCurve : copy.paper.strategyCurve}`}
      className="paper-strategy-chart-wrap"
      data-testid="paper-strategy-recharts"
      role="img"
    >
      <div className="paper-strategy-chart-legend" aria-hidden="true">
        <span className="paper-strategy-legend-positive">
          <i />
          盈利
        </span>
        <span className="paper-strategy-legend-negative">
          <i />
          亏损
        </span>
        <span className="paper-strategy-legend-zero">
          <i />
          0轴
        </span>
      </div>
      <div className="paper-strategy-recharts-curve">
        {import.meta.env.MODE === "test" ? chart : (
          <ResponsiveContainer height="100%" width="100%">
            {chart}
          </ResponsiveContainer>
        )}
      </div>
      <div className="paper-strategy-axis-summary" data-testid="paper-strategy-axis-summary">
        <span>时间轴 · {axisScale.label}</span>
        <span>{formatChartDateTime(rawStartMs)} - {formatChartDateTime(rawEndMs)}</span>
      </div>
    </div>
  );
}

function StrategyCurveTooltip({
  active,
  copy,
  curveKind,
  payload,
}: {
  active?: boolean;
  copy: Copy;
  curveKind: StrategyCurveKind;
  payload?: Array<{ payload?: StrategyCurveChartPoint }>;
}) {
  if (!active || !payload || payload.length === 0) {
    return null;
  }
  const point = payload.find((item) => item.payload)?.payload;
  if (point === undefined) {
    return null;
  }

  return (
    <div className="paper-strategy-tooltip">
      <strong>{point.synthetic ? "零轴" : formatChartDateTime(point.closedAtMs)}</strong>
      <span>{formatChartDateTime(point.closedAtMs)}</span>
      <dl>
        <Metric
          label={curveKind === "equity" ? copy.paper.equityChange : "累计收益"}
          value={formatSignedUsdt(point.cumulativePnl)}
        />
        <Metric
          label={curveKind === "equity" ? copy.paper.equityPeriodChange : "区间盈亏"}
          value={point.synthetic ? "-" : formatSignedUsdt(point.periodPnl)}
        />
        <Metric
          label={curveKind === "equity" ? copy.paper.equityPoints : "平仓数"}
          value={point.synthetic ? "-" : String(point.closedCount)}
        />
      </dl>
    </div>
  );
}

function buildStrategyCurveChartData(
  points: StrategyCurvePoint[],
  stepMs: number,
): StrategyCurveChartPoint[] {
  const bucketedPoints = bucketStrategyCurvePoints(points, stepMs);
  return bucketedPoints.reduce<StrategyCurveChartPoint[]>((chartPoints, point, index) => {
    const previousPoint = bucketedPoints[index - 1];
    if (previousPoint && crossesZero(previousPoint.cumulativePnl, point.cumulativePnl)) {
      const distance =
        Math.abs(previousPoint.cumulativePnl) + Math.abs(point.cumulativePnl);
      const zeroRatio = distance === 0 ? 0 : Math.abs(previousPoint.cumulativePnl) / distance;
      chartPoints.push({
        closedAtMs: Math.round(
          previousPoint.closedAtMs + (point.closedAtMs - previousPoint.closedAtMs) * zeroRatio,
        ),
        closedCount: 0,
        cumulativePnl: 0,
        negativePnl: 0,
        periodPnl: 0,
        positivePnl: 0,
        synthetic: true,
      });
    }
    chartPoints.push(toStrategyChartPoint(point));
    return chartPoints;
  }, []);
}

function bucketStrategyCurvePoints(
  points: StrategyCurvePoint[],
  stepMs: number,
): StrategyCurveChartPoint[] {
  const buckets = new Map<number, StrategyCurveChartPoint>();
  points.forEach((point) => {
    const bucketMs = Math.floor(point.closedAtMs / stepMs) * stepMs;
    const existing = buckets.get(bucketMs);
    buckets.set(bucketMs, {
      closedAtMs: bucketMs,
      closedCount: (existing?.closedCount ?? 0) + 1,
      cumulativePnl: point.cumulativePnl,
      negativePnl: null,
      periodPnl: (existing?.periodPnl ?? 0) + point.realizedPnl,
      positivePnl: null,
    });
  });
  return Array.from(buckets.values()).sort((left, right) => left.closedAtMs - right.closedAtMs);
}

function toStrategyChartPoint(point: StrategyCurveChartPoint): StrategyCurveChartPoint {
  const isZero = point.cumulativePnl === 0;
  return {
    ...point,
    negativePnl: point.cumulativePnl < 0 || isZero ? point.cumulativePnl : null,
    positivePnl: point.cumulativePnl > 0 || isZero ? point.cumulativePnl : null,
  };
}

function crossesZero(left: number, right: number): boolean {
  return (left < 0 && right > 0) || (left > 0 && right < 0);
}

function chooseStrategyAxisScale(startMs: number, endMs: number): StrategyAxisScale {
  const span = Math.max(0, endMs - startMs);
  if (span <= DAY_MS) {
    return { label: "10分钟", stepMs: 10 * MINUTE_MS, tickFormat: "time" };
  }
  if (span <= 3 * DAY_MS) {
    return { label: "30分钟", stepMs: 30 * MINUTE_MS, tickFormat: "dateTime" };
  }
  if (span <= 7 * DAY_MS) {
    return { label: "1小时", stepMs: HOUR_MS, tickFormat: "dateTime" };
  }
  if (span <= 31 * DAY_MS) {
    return { label: "4小时", stepMs: 4 * HOUR_MS, tickFormat: "dateTime" };
  }
  if (span <= 366 * DAY_MS) {
    return { label: "1D", stepMs: DAY_MS, tickFormat: "date" };
  }
  if (span <= 3 * 366 * DAY_MS) {
    return { label: "1W", stepMs: 7 * DAY_MS, tickFormat: "date" };
  }
  return { label: "1M", stepMs: 30 * DAY_MS, tickFormat: "month" };
}

function buildStrategyAxisTicks(
  startMs: number,
  endMs: number,
  axisScale: StrategyAxisScale,
): number[] {
  if (startMs === endMs) {
    return [startMs];
  }

  const ticks = [startMs];
  const firstIntervalTick = Math.ceil(startMs / axisScale.stepMs) * axisScale.stepMs;
  for (
    let tick = firstIntervalTick;
    tick < endMs && ticks.length < 120;
    tick += axisScale.stepMs
  ) {
    if (tick > startMs) {
      ticks.push(tick);
    }
  }
  ticks.push(endMs);

  return limitTicks(uniqueSortedNumbers(ticks), 8);
}

function limitTicks(ticks: number[], maxTicks: number): number[] {
  if (ticks.length <= maxTicks) {
    return ticks;
  }
  return uniqueSortedNumbers(
    Array.from({ length: maxTicks }, (_, index) => {
      const sourceIndex = Math.round((index * (ticks.length - 1)) / (maxTicks - 1));
      return ticks[sourceIndex];
    }),
  );
}

function uniqueSortedNumbers(values: number[]): number[] {
  return Array.from(new Set(values)).sort((left, right) => left - right);
}

function formatStrategyAxisTick(value: number, tickFormat: StrategyAxisScale["tickFormat"]): string {
  if (tickFormat === "month") {
    return new Intl.DateTimeFormat("zh-CN", {
      month: "2-digit",
      year: "2-digit",
    }).format(new Date(value));
  }
  if (tickFormat === "date") {
    return new Intl.DateTimeFormat("zh-CN", {
      day: "2-digit",
      month: "2-digit",
    }).format(new Date(value));
  }
  if (tickFormat === "dateTime") {
    return new Intl.DateTimeFormat("zh-CN", {
      day: "2-digit",
      hour: "2-digit",
      hour12: false,
      minute: "2-digit",
      month: "2-digit",
    }).format(new Date(value));
  }
  return new Intl.DateTimeFormat("zh-CN", {
    hour: "2-digit",
    hour12: false,
    minute: "2-digit",
  }).format(new Date(value));
}

function formatChartDateTime(value: number): string {
  return new Intl.DateTimeFormat("zh-CN", {
    day: "2-digit",
    hour: "2-digit",
    hour12: false,
    minute: "2-digit",
    month: "2-digit",
    year: "numeric",
  }).format(new Date(value));
}

function formatHistoryDateTime(value: number): string {
  return new Intl.DateTimeFormat("zh-CN", {
    day: "2-digit",
    hour: "2-digit",
    hour12: false,
    minute: "2-digit",
    month: "2-digit",
    year: "numeric",
  }).format(new Date(value));
}

function formatCompactUsdt(value: number): string {
  const prefix = value > 0 ? "+" : "";
  const absValue = Math.abs(value);
  if (absValue >= 1_000_000) {
    return `${prefix}${(value / 1_000_000).toFixed(1)}M`;
  }
  if (absValue >= 1_000) {
    return `${prefix}${(value / 1_000).toFixed(1)}K`;
  }
  return `${prefix}${value.toFixed(0)}`;
}

function HistorySection({
  copy,
  draftFilters,
  filteredHistory,
  historyVersionOptions,
  onApplyFilters,
  onDraftFiltersChange,
  onPageChange,
  onResetFilters,
  openPositionCount,
  page,
  pageCount,
  totalHistory,
  visibleHistory,
}: {
  copy: Copy;
  draftFilters: HistorySearchFilters;
  filteredHistory: PaperClosedPositionSnapshot[];
  historyVersionOptions: string[];
  onApplyFilters: () => void;
  onDraftFiltersChange: (filters: HistorySearchFilters) => void;
  onPageChange: (page: number) => void;
  onResetFilters: () => void;
  openPositionCount: number;
  page: number;
  pageCount: number;
  totalHistory: number;
  visibleHistory: PaperClosedPositionSnapshot[];
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
      <form
        className="review-history-search-form"
        onSubmit={(event) => {
          event.preventDefault();
          onApplyFilters();
        }}
      >
        <div className="review-history-filter-grid">
          <label>
            <span>{copy.paper.historySymbolSearch}</span>
            <input
              aria-label={copy.paper.historySymbolSearch}
              onChange={(event) =>
                onDraftFiltersChange({ ...draftFilters, symbolQuery: event.target.value })
              }
              placeholder="BTC / ETH / LAB"
              type="search"
              value={draftFilters.symbolQuery}
            />
          </label>
          <label>
            <span>{copy.paper.historyStartDate}</span>
            <input
              aria-label={copy.paper.historyStartDate}
              onChange={(event) =>
                onDraftFiltersChange({ ...draftFilters, startDate: event.target.value })
              }
              step="1"
              type="datetime-local"
              value={draftFilters.startDate}
            />
          </label>
          <label>
            <span>{copy.paper.historyEndDate}</span>
            <input
              aria-label={copy.paper.historyEndDate}
              onChange={(event) =>
                onDraftFiltersChange({ ...draftFilters, endDate: event.target.value })
              }
              step="1"
              type="datetime-local"
              value={draftFilters.endDate}
            />
          </label>
          <label>
            <span>{copy.paper.historyVersion}</span>
            <select
              aria-label={copy.paper.historyVersion}
              onChange={(event) =>
                onDraftFiltersChange({ ...draftFilters, version: event.target.value })
              }
              value={draftFilters.version}
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
        <div className="review-history-search-row">
          <div className="page-local-tabs review-history-filters" role="group" aria-label={copy.paper.positionHistory}>
            {(["all", "long", "short", "profit", "loss"] as HistoryFilter[]).map((value) => (
              <button
                className={draftFilters.filter === value ? "active" : ""}
                key={value}
                onClick={() => onDraftFiltersChange({ ...draftFilters, filter: value })}
                type="button"
              >
                {copy.paper.filters[value]}
              </button>
            ))}
          </div>
          <div className="review-history-search-actions">
            <button className="review-history-search-button" type="submit">
              {copy.paper.historySearch}
            </button>
            <button className="review-history-reset-button" onClick={onResetFilters} type="button">
              {copy.paper.historyReset}
            </button>
          </div>
        </div>
      </form>
      {filteredHistory.length === 0 ? (
        <p className="muted panel-empty">
          {copy.paper.noPositionHistory}
          {totalHistory === 0 && openPositionCount > 0 ? (
            <>
              <br />
              <span>{copy.paper.openPositions} {openPositionCount}</span>
            </>
          ) : null}
        </p>
      ) : (
        <>
          <ul className="review-history-list">
            {visibleHistory.map((position) => (
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
                  <Metric label={copy.paper.openedAt} value={formatHistoryDateTime(position.opened_at_ms)} />
                  <Metric label={copy.paper.closedAt} value={formatHistoryDateTime(position.closed_at_ms)} />
                  <Metric label={copy.paper.strategyVersion} value={strategyLabel(position)} />
                  <Metric label={copy.paper.openReason} value={position.reason || "-"} />
                  <Metric label={copy.paper.closeReason} value={position.close_reason || "-"} />
                </dl>
                <TagChips tags={[...(position.open_tags ?? []), ...(position.close_tags ?? []), ...(position.tags ?? [])]} />
              </li>
            ))}
          </ul>
          {pageCount > 1 ? (
            <PaginationControls
              className="review-history-pagination"
              copy={copy}
              onPageChange={onPageChange}
              page={page}
              pageCount={pageCount}
              testId="review-history-pagination"
              total={filteredHistory.length}
            />
          ) : null}
        </>
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

function TagChips({ tags }: { tags: TradeTagLike[] }) {
  if (tags.length === 0) {
    return null;
  }
  return (
    <span className="tag-chip-row">
      {tags.map((tag, index) => {
        const label = tradeTagLabel(tag);
        const isWarning = typeof tag !== "string" && tag.score_impact < 0;
        return (
          <span
            className={isWarning ? "tag-chip tag-chip-warning" : "tag-chip"}
            key={typeof tag === "string" ? `${tag}-${index}` : `${tag.kind}-${tag.ts_ms}-${index}`}
            title={typeof tag === "string" ? label : tag.reason}
          >
            {label}
          </span>
        );
      })}
    </span>
  );
}

function RealizedCurve({ points }: { points: Array<{ id: number; value: number }> }) {
  if (points.length === 0) {
    return <div className="review-empty-chart" />;
  }

  const data = points.map((point, index) => ({
    id: point.id,
    label: `#${index + 1}`,
    value: point.value,
  }));
  const chart = (
    <AreaChart
      data={data}
      margin={{ bottom: 8, left: -10, right: 10, top: 12 }}
      {...(import.meta.env.MODE === "test" ? { height: 220, width: 720 } : {})}
    >
      <defs>
        <linearGradient id="reviewRealizedFill" x1="0" x2="0" y1="0" y2="1">
          <stop offset="5%" stopColor="#10b981" stopOpacity={0.28} />
          <stop offset="95%" stopColor="#10b981" stopOpacity={0} />
        </linearGradient>
      </defs>
      <CartesianGrid stroke="rgba(74, 98, 120, 0.22)" strokeDasharray="3 3" />
      <XAxis
        dataKey="label"
        tick={{ fill: "#4a6278", fontSize: 10 }}
        tickLine={false}
      />
      <YAxis
        tick={{ fill: "#4a6278", fontSize: 10 }}
        tickFormatter={(value) => `$${Number(value).toFixed(0)}`}
        tickLine={false}
        width={58}
      />
      <Tooltip
        contentStyle={{
          background: "#0b1220",
          border: "1px solid rgba(74, 98, 120, 0.45)",
          borderRadius: 8,
          color: "#c8d8e8",
          fontFamily: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
          fontSize: 12,
        }}
        formatter={(value) => [`$${Number(value).toFixed(2)}`, "PnL"]}
        labelFormatter={(label) => `Trade ${label}`}
      />
      <ReferenceLine stroke="rgba(148, 174, 196, 0.24)" y={0} />
      <Area
        dataKey="value"
        dot={{ fill: "#10b981", r: 3, stroke: "#0b1220", strokeWidth: 2 }}
        fill="url(#reviewRealizedFill)"
        isAnimationActive={false}
        name="Realized PnL"
        stroke="#10b981"
        strokeWidth={2}
        type="monotone"
      />
    </AreaChart>
  );

  return (
    <div
      aria-label="Realized PnL curve"
      className="review-curve review-recharts-curve"
      data-testid="review-realized-recharts"
      role="img"
    >
      {import.meta.env.MODE === "test" ? chart : (
        <ResponsiveContainer height="100%" width="100%">
          {chart}
        </ResponsiveContainer>
      )}
    </div>
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
  if (startMs !== null && position.opened_at_ms < startMs) {
    return false;
  }
  const endMs = dateEndMs(filters.endDate);
  if (endMs !== null && position.closed_at_ms > endMs) {
    return false;
  }
  return true;
}

function buildLiveStrategyStats(strategyCenter?: StrategyCenterSnapshot): ReviewStrategyStats[] {
  return (
    strategyCenter?.versions.map(({ overview, run, version }) => {
      const winningCount =
        overview.win_rate === null ? 0 : Math.round(overview.win_rate * overview.closed_trades);
      return {
        average_holding_duration_ms: null,
        average_position_pnl:
          overview.closed_trades > 0 ? overview.realized_pnl / overview.closed_trades : null,
        closed_position_count: overview.closed_trades,
        current_equity: overview.current_equity,
        first_trade_ts_ms: run.start_time_ms,
        largest_losing_pnl: null,
        largest_winning_pnl: null,
        last_trade_ts_ms: overview.last_updated_ms,
        losing_closed_position_count: Math.max(0, overview.closed_trades - winningCount),
        open_position_count: overview.open_positions,
        profit_factor: overview.profit_factor,
        realized_pnl: overview.realized_pnl,
        return_pct: overview.return_pct,
        run_status: run.status,
        running_duration_ms: overview.run_time_ms,
        strategy_name: overview.name || version.name,
        strategy_version: overview.version_code,
        total_fees: overview.total_fee,
        total_trades: overview.closed_trades + overview.open_positions,
        unrealized_pnl: overview.unrealized_pnl,
        win_rate: overview.win_rate,
        winning_closed_position_count: winningCount,
      };
    }) ?? []
  );
}

function mergeLiveStrategyStats(
  closedStats: ReviewStrategyStats[],
  liveStats: ReviewStrategyStats[],
): ReviewStrategyStats[] {
  if (closedStats.length === 0) {
    return liveStats;
  }

  const liveByVersion = new Map(liveStats.map((item) => [item.strategy_version, item]));
  const closedVersions = new Set(closedStats.map((item) => item.strategy_version));
  const mergedClosedStats = closedStats.map((item) => {
    const live = liveByVersion.get(item.strategy_version);
    return live === undefined
      ? item
      : {
          ...item,
          current_equity: live.current_equity,
          open_position_count: live.open_position_count,
          return_pct: live.return_pct,
          run_status: live.run_status,
          unrealized_pnl: live.unrealized_pnl,
        };
  });
  const liveOnlyStats = liveStats.filter((item) => !closedVersions.has(item.strategy_version));
  return [...mergedClosedStats, ...liveOnlyStats];
}

function buildStrategyStats(positions: PaperClosedPositionSnapshot[]): ReviewStrategyStats[] {
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

function sortStrategyStats(stats: ReviewStrategyStats[]): ReviewStrategyStats[] {
  return [...stats].sort((left, right) => {
    const versionOrder = compareStrategyVersions(right.strategy_version, left.strategy_version);
    if (versionOrder !== 0) {
      return versionOrder;
    }
    return (right.last_trade_ts_ms ?? 0) - (left.last_trade_ts_ms ?? 0);
  });
}

function compareStrategyVersions(left: string, right: string): number {
  const leftParts = parseStrategyVersion(left);
  const rightParts = parseStrategyVersion(right);
  const length = Math.max(leftParts.length, rightParts.length);
  for (let index = 0; index < length; index += 1) {
    const diff = (leftParts[index] ?? 0) - (rightParts[index] ?? 0);
    if (diff !== 0) {
      return diff;
    }
  }
  return left.localeCompare(right);
}

function parseStrategyVersion(version: string): number[] {
  const matches = version.match(/\d+/g);
  return matches === null ? [] : matches.map((part) => Number(part));
}

function buildStrategyCurve(
  positions: PaperClosedPositionSnapshot[],
  version: string,
  liveVersion?: StrategyCenterSnapshot["versions"][number],
): StrategyCurve {
  const matched = positions
    .filter((position) => strategyVersion(position) === version)
    .slice()
    .sort((left, right) => left.closed_at_ms - right.closed_at_ms || left.id - right.id);
  if (matched.length === 0 && liveVersion?.equity && liveVersion.equity.length > 0) {
    return buildStrategyEquityCurve(
      liveVersion.equity,
      liveVersion.run.initial_equity || liveVersion.paper.initial_balance,
    );
  }

  let cumulativePnl = 0;
  let peak = 0;
  let maxDrawdown = 0;
  let bestCumulative = 0;
  let worstCumulative = 0;
  const points = matched.map((position, index) => {
    cumulativePnl += position.realized_pnl;
    peak = Math.max(peak, cumulativePnl);
    maxDrawdown = Math.min(maxDrawdown, cumulativePnl - peak);
    bestCumulative = Math.max(bestCumulative, cumulativePnl);
    worstCumulative = Math.min(worstCumulative, cumulativePnl);
    return {
      closedAtMs: position.closed_at_ms,
      cumulativePnl,
      instId: position.inst_id,
      realizedPnl: position.realized_pnl,
      tradeIndex: index + 1,
    };
  });

  return {
    bestCumulative,
    finalPnl: cumulativePnl,
    kind: "realized",
    maxDrawdown,
    points,
    worstCumulative,
  };
}

function buildStrategyEquityCurve(
  snapshots: StrategyEquitySnapshot[],
  initialEquity: number,
): StrategyCurve {
  const sorted = snapshots
    .slice()
    .sort((left, right) => left.timestamp_ms - right.timestamp_ms);
  let peak = 0;
  let maxDrawdown = 0;
  let bestCumulative = 0;
  let worstCumulative = 0;
  const points = sorted.map((snapshot, index) => {
    const cumulativePnl = snapshot.equity - initialEquity;
    const previousEquity = index === 0 ? initialEquity : sorted[index - 1].equity;
    const periodPnl = snapshot.equity - previousEquity;
    peak = Math.max(peak, cumulativePnl);
    maxDrawdown = Math.min(maxDrawdown, cumulativePnl - peak);
    bestCumulative = Math.max(bestCumulative, cumulativePnl);
    worstCumulative = Math.min(worstCumulative, cumulativePnl);
    return {
      closedAtMs: snapshot.timestamp_ms,
      cumulativePnl,
      instId: snapshot.version_code,
      realizedPnl: periodPnl,
      tradeIndex: index + 1,
    };
  });

  return {
    bestCumulative,
    finalPnl: points.length === 0 ? 0 : points[points.length - 1].cumulativePnl,
    kind: "equity",
    latestEquity: sorted.length === 0 ? undefined : sorted[sorted.length - 1].equity,
    maxDrawdown,
    points,
    worstCumulative,
  };
}

function filterStrategyCurve(curve: StrategyCurve, range: StrategyCurveRange): StrategyCurve {
  if (range === "all" || curve.points.length <= 1) {
    return curve;
  }
  const rangeMs =
    range === "7d" ? 7 * DAY_MS : range === "30d" ? 30 * DAY_MS : 90 * DAY_MS;
  const endMs = curve.points[curve.points.length - 1].closedAtMs;
  const points = curve.points.filter((point) => point.closedAtMs >= endMs - rangeMs);
  return points.length === 0 ? curve : { ...curve, points };
}

function buildSignalAttribution(
  positions: PaperClosedPositionSnapshot[],
  version: string,
): StrategySignalAttribution[] {
  const groups = new Map<string, PaperClosedPositionSnapshot[]>();
  positions
    .filter((position) => strategyVersion(position) === version)
    .forEach((position) => {
      const signal = primarySignalLabel(position);
      groups.set(signal, [...(groups.get(signal) ?? []), position]);
    });

  return Array.from(groups.entries())
    .map(([signal, group]) => {
      const winners = group.filter((position) => position.realized_pnl > 0);
      const losers = group.filter((position) => position.realized_pnl < 0);
      const grossProfit = sum(winners.map((position) => position.realized_pnl));
      const grossLossAbs = Math.abs(sum(losers.map((position) => position.realized_pnl)));
      const netPnl = sum(group.map((position) => position.realized_pnl));
      const winRate = group.length === 0 ? null : winners.length / group.length;
      const profitFactor =
        grossProfit > 0 && grossLossAbs > 0 ? grossProfit / grossLossAbs : null;
      const maxLoss = minNumber(losers.map((position) => position.realized_pnl));
      const confidence = signalConfidence(group.length, netPnl, winRate, profitFactor);
      return {
        confidence,
        maxLoss,
        netPnl,
        profitFactor,
        recommendationKey: signalRecommendation(confidence),
        sampleCount: group.length,
        signal,
        winRate,
      };
    })
    .sort((left, right) => right.netPnl - left.netPnl || right.sampleCount - left.sampleCount);
}

function primarySignalLabel(position: PaperClosedPositionSnapshot): string {
  const tagLabel = [
    ...(position.open_tags ?? []),
    ...(position.tags ?? []),
    ...(position.close_tags ?? []),
  ]
    .map((tag) => tradeTagLabel(tag).trim())
    .find((label) => label.length > 0);
  if (tagLabel !== undefined) {
    return tagLabel;
  }

  const reason = `${position.reason} ${position.close_reason}`.toLowerCase();
  if (reason.includes("fvg") && reason.includes("trend")) {
    return "FVG + Trend";
  }
  if (reason.includes("hot") && reason.includes("trend")) {
    return "Hot Mover + Trend";
  }
  if (reason.includes("sweep")) {
    return "Sweep Failure";
  }
  if (reason.includes("multiday") || reason.includes("reversal")) {
    return "Multiday Reversal";
  }
  if (reason.includes("overextension") || reason.includes("extension")) {
    return position.side === "short" ? "Overextension Short" : "Overextension Long";
  }
  if (reason.includes("pattern") && reason.includes("range")) {
    return "Pattern + Range";
  }
  if (reason.includes("time") && reason.includes("range")) {
    return "Time Risk + Range";
  }
  if (reason.includes("trend")) {
    return position.side === "short" ? "Trend Short" : "Trend Long";
  }
  return "Scalping Signal";
}

function tradeTagLabel(tag: TradeTagLike): string {
  return typeof tag === "string" ? tag : tag.label;
}

function signalConfidence(
  sampleCount: number,
  netPnl: number,
  winRate: number | null,
  profitFactor: number | null,
): SignalConfidence {
  if (
    sampleCount >= 3 &&
    netPnl > 0 &&
    (winRate ?? 0) >= 0.62 &&
    (profitFactor ?? 0) >= 2
  ) {
    return "high";
  }
  if (netPnl > 0 && (winRate ?? 0) >= 0.5) {
    return "med";
  }
  return "low";
}

function signalRecommendation(confidence: SignalConfidence): SignalRecommendationKey {
  if (confidence === "high") {
    return "continueExecution";
  }
  if (confidence === "med") {
    return "optimizeEntry";
  }
  return "pauseOrReduce";
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
  stats: ReviewStrategyStats[],
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

function accountReturnRate(paper: PaperAccountSnapshot): number | null {
  return strategyReturnRate(paper.equity - paper.initial_balance, paper.initial_balance);
}

function strategyReturnRate(value: number, initialBalance: number): number | null {
  return initialBalance === 0 ? null : value / initialBalance;
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
