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
  PaperEquityCandle,
  PaperEquityCurves,
  PaperEquityPoint,
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
import { PaginationControls } from "./PaginationControls";

type HistoryFilter = "all" | "long" | "short" | "profit" | "loss";
type ReviewSection = "overview" | "strategy" | "history" | "trades";
type StrategyCurveRange = "1d" | "7d" | "30d" | "90d" | "all";
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
  timestampMs: number;
  openEquity: number;
  highEquity: number;
  lowEquity: number;
  equity: number;
  equityChange: number;
  realizedPnl: number;
  unrealizedPnl: number;
  openPositionsCount: number;
}

interface StrategyCurve {
  points: StrategyCurvePoint[];
  currentEquity: number;
  equityChange: number;
  maxDrawdown: number;
  peakEquity: number;
  troughEquity: number;
}

interface StrategyCurveChartPoint {
  timestampMs: number;
  snapshotCount: number;
  openEquity: number;
  highEquity: number;
  lowEquity: number;
  equity: number;
  equityChange: number;
  negativeEquity: number | null;
  positiveEquity: number | null;
  realizedPnl: number;
  unrealizedPnl: number;
  openPositionsCount: number;
  synthetic?: boolean;
}

interface StrategyAxisScale {
  label: string;
  stepMs: number;
  tickFormat: "date" | "dateTime" | "month" | "time";
}

const STRATEGY_CURVE_RANGE_OPTIONS: Array<[StrategyCurveRange, string]> = [
  ["1d", "1D"],
  ["7d", "7D"],
  ["30d", "30D"],
  ["90d", "90D"],
  ["all", "ALL"],
];

interface StrategySignalAttribution {
  confidence: SignalConfidence;
  maxLoss: number | null;
  netPnl: number;
  positions: PaperClosedPositionSnapshot[];
  profitFactor: number | null;
  recommendationKey: SignalRecommendationKey;
  sampleCount: number;
  signal: string;
  winRate: number | null;
}

const MINUTE_MS = 60_000;
const HOUR_MS = 60 * MINUTE_MS;
const DAY_MS = 24 * HOUR_MS;
const EQUITY_AXIS_PADDING_RATIO = 0.12;
const EQUITY_AXIS_MIN_PADDING_RATIO = 0.0025;
const HISTORY_PAGE_SIZE = 10;
const DEFAULT_HISTORY_SEARCH_FILTERS: HistorySearchFilters = {
  endDate: "",
  filter: "all",
  startDate: "",
  symbolQuery: "",
  version: "all",
};

export function ReviewPage({ copy, paper }: { copy: Copy; paper: PaperAccountSnapshot }) {
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
    () =>
      sortStrategyStats(
        paper.strategy_stats && paper.strategy_stats.length > 0
          ? paper.strategy_stats
          : buildStrategyStats(positionHistory),
      ),
    [paper.strategy_stats, positionHistory],
  );
  const activeStrategyVersion =
    selectedStrategyVersion ?? paper.strategy_version ?? strategyStats[0]?.strategy_version ?? null;
  const activeStrategyCurve =
    activeStrategyVersion === null
      ? null
      : buildStrategyCurve(
          activeStrategyVersion === paper.strategy_version ? (paper.equity_history ?? []) : [],
          paper.initial_balance,
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
              equityCurves={
                activeStrategyVersion === paper.strategy_version ? paper.equity_curves : undefined
              }
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
  strategyStats: PaperStrategyStats[];
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
        <AccountEquityCurve copy={copy} paper={paper} />
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

function AccountEquityCurve({
  copy,
  paper,
}: {
  copy: Copy;
  paper: PaperAccountSnapshot;
}) {
  const [range, setRange] = useState<StrategyCurveRange>("all");
  const curve = useMemo(
    () => buildStrategyCurve(buildAccountEquityHistory(paper), paper.initial_balance),
    [paper],
  );
  const chartCurve = useMemo(
    () => buildCurveForRange(curve, paper.equity_curves, range, paper.initial_balance),
    [curve, paper.equity_curves, paper.initial_balance, range],
  );

  return (
    <section
      className="detail-section review-chart-card review-equity-card"
      data-testid="review-equity-curve"
    >
      <header className="paper-card-header paper-strategy-curve-header">
        <div>
          <h2>{copy.review.equityCurve}</h2>
          <p>{copy.review.equityCurveDescription}</p>
        </div>
        <div className="paper-strategy-range-tabs" role="group" aria-label={copy.review.equityCurve}>
          {STRATEGY_CURVE_RANGE_OPTIONS.map(([value, label]) => (
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
      </header>
      <div className="paper-strategy-curve-metrics review-equity-metrics">
        <Metric label={copy.paper.initialBalance} value={formatUsdt(paper.initial_balance)} />
        <Metric label={copy.paper.currentEquity} value={formatUsdt(chartCurve.currentEquity)} />
        <Metric label={copy.paper.equityChange} value={formatSignedUsdt(chartCurve.equityChange)} />
        <Metric label={copy.paper.maxDrawdown} value={formatSignedUsdt(chartCurve.maxDrawdown)} />
        <Metric label={copy.paper.peakEquity} value={formatUsdt(chartCurve.peakEquity)} />
        <Metric label={copy.paper.troughEquity} value={formatUsdt(chartCurve.troughEquity)} />
      </div>
      <StrategyCurveChart
        ariaLabel={copy.review.equityCurve}
        copy={copy}
        curve={chartCurve}
        initialBalance={paper.initial_balance}
        range={range}
        testId="review-equity-recharts"
        version={paper.strategy_version}
      />
    </section>
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
  stats: PaperStrategyStats[];
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
            const isActiveVersion = activeVersion === item.strategy_version;
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
              <td>{item.closed_position_count}</td>
              <td>{formatNullablePct(item.win_rate)}</td>
              <td className={pnlClass(item.realized_pnl)}>{formatSignedUsdt(item.realized_pnl)}</td>
              <td className={pnlClass(item.realized_pnl)}>
                {formatNullablePct(strategyReturnRate(item.realized_pnl, initialBalance))}
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
  equityCurves,
  initialBalance,
  version,
}: {
  copy: Copy;
  curve: StrategyCurve;
  equityCurves?: PaperEquityCurves;
  initialBalance: number;
  version: string;
}) {
  const [range, setRange] = useState<StrategyCurveRange>("all");
  const chartCurve = useMemo(
    () => buildCurveForRange(curve, equityCurves, range, initialBalance),
    [curve, equityCurves, initialBalance, range],
  );

  return (
    <section className="paper-strategy-curve" data-testid="paper-strategy-curve">
      <div className="paper-card-header paper-strategy-curve-header">
        <div>
          <h3>{version} {copy.paper.strategyCurve}</h3>
        </div>
        <div className="paper-strategy-range-tabs" role="group" aria-label={copy.paper.strategyCurve}>
          {STRATEGY_CURVE_RANGE_OPTIONS.map(([value, label]) => (
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
              label={copy.paper.currentEquity}
              value={formatUsdt(chartCurve.currentEquity)}
            />
            <Metric
              label={copy.paper.equityChange}
              value={formatSignedUsdt(chartCurve.equityChange)}
            />
            <Metric
              label={copy.paper.returnRate}
              value={formatNullablePct(strategyReturnRate(chartCurve.equityChange, initialBalance))}
            />
            <Metric label={copy.paper.maxDrawdown} value={formatSignedUsdt(chartCurve.maxDrawdown)} />
            <Metric
              label={copy.paper.peakEquity}
              value={formatUsdt(chartCurve.peakEquity)}
            />
            <Metric
              label={copy.paper.troughEquity}
              value={formatUsdt(chartCurve.troughEquity)}
            />
            <Metric label={copy.paper.equitySnapshots} value={String(chartCurve.points.length)} />
          </div>
          <StrategyCurveChart
            copy={copy}
            curve={chartCurve}
            initialBalance={initialBalance}
            range={range}
            version={version}
          />
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
  const [selectedRow, setSelectedRow] = useState<StrategySignalAttribution | null>(null);

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
              {rows.map((row) => {
                const detailLabel = copy.paper.viewAttributionDetails.replace(
                  "{signal}",
                  row.signal,
                );
                return (
                  <tr
                    aria-label={detailLabel}
                    className="paper-strategy-doctor-row"
                    key={row.signal}
                    onClick={() => setSelectedRow(row)}
                    onKeyDown={(event) => {
                      if (event.key === "Enter" || event.key === " ") {
                        event.preventDefault();
                        setSelectedRow(row);
                      }
                    }}
                    tabIndex={0}
                  >
                    <td>
                      <button
                        aria-label={detailLabel}
                        className="paper-strategy-doctor-detail-button"
                        onClick={(event) => {
                          event.stopPropagation();
                          setSelectedRow(row);
                        }}
                        type="button"
                      >
                        <span>{row.signal}</span>
                        <small>{copy.paper.attributionDetails}</small>
                      </button>
                    </td>
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
                );
              })}
            </tbody>
          </table>
        </div>
      )}
      {selectedRow ? (
        <StrategyDoctorDetailDialog
          copy={copy}
          onClose={() => setSelectedRow(null)}
          row={selectedRow}
          version={version}
        />
      ) : null}
    </section>
  );
}

function StrategyDoctorDetailDialog({
  copy,
  onClose,
  row,
  version,
}: {
  copy: Copy;
  onClose: () => void;
  row: StrategySignalAttribution;
  version: string;
}) {
  const title = `${row.signal} ${copy.paper.attributionDetails}`;
  const components = signalAttributionComponents(row.positions);

  return (
    <div className="reason-modal-backdrop" onClick={onClose}>
      <section
        aria-label={title}
        aria-modal="true"
        className="reason-modal strategy-doctor-detail-modal"
        onClick={(event) => event.stopPropagation()}
        role="dialog"
      >
        <header>
          <div>
            <h2>{title}</h2>
            <p>{version}</p>
          </div>
          <button aria-label={copy.paper.closeAttributionDetails} onClick={onClose} type="button">
            ×
          </button>
        </header>
        <div className="reason-modal-body">
          <dl className="reason-detail-grid strategy-doctor-detail-summary">
            <div>
              <dt>{copy.paper.sampleCount}</dt>
              <dd>{row.sampleCount}</dd>
            </div>
            <div>
              <dt>{copy.paper.netPnl}</dt>
              <dd className={pnlClass(row.netPnl)}>{formatSignedUsdt(row.netPnl)}</dd>
            </div>
            <div>
              <dt>{copy.paper.winRate}</dt>
              <dd>{formatNullablePct(row.winRate)}</dd>
            </div>
            <div>
              <dt>{copy.paper.profitFactor}</dt>
              <dd>{formatNullableRatio(row.profitFactor)}</dd>
            </div>
            <div>
              <dt>{copy.paper.confidence}</dt>
              <dd>
                <span className={`confidence-pill ${row.confidence}`}>{row.confidence}</span>
              </dd>
            </div>
            <div>
              <dt>{copy.paper.recommendation}</dt>
              <dd>{copy.paper[row.recommendationKey]}</dd>
            </div>
          </dl>

          <section className="strategy-doctor-combination">
            <h3>{copy.paper.signalCombination}</h3>
            <p>{copy.paper.signalCombinationDescription}</p>
            {components.length === 0 ? (
              <p className="muted">{copy.trade.noDecisionTags}</p>
            ) : (
              <div className="strategy-doctor-signal-chips">
                {components.map((component) => (
                  <span key={component}>{component}</span>
                ))}
              </div>
            )}
          </section>

          <section className="strategy-doctor-samples">
            <h3>
              {copy.paper.tradeSampleDetails} <span>{row.positions.length}</span>
            </h3>
            <div className="strategy-doctor-sample-list">
              {row.positions.map((position) => (
                <StrategyDoctorSample copy={copy} key={position.id} position={position} />
              ))}
            </div>
          </section>
        </div>
      </section>
    </div>
  );
}

function StrategyDoctorSample({
  copy,
  position,
}: {
  copy: Copy;
  position: PaperClosedPositionSnapshot;
}) {
  const decisionTags = strategyPositionTags(position);
  const signalTags = uniqueStrings([
    position.primary_signal ?? "",
    ...(position.signal_tags ?? []),
  ]);

  return (
    <article className="strategy-doctor-sample">
      <header>
        <div>
          <strong>{position.inst_id}</strong>
          <span>
            {copy.directions[position.side]} · {formatTimestamp(position.closed_at_ms)}
          </span>
        </div>
        <strong className={pnlClass(position.realized_pnl)}>
          {formatSignedUsdt(position.realized_pnl)}
        </strong>
      </header>
      <dl className="strategy-doctor-sample-metrics">
        <div>
          <dt>{copy.paper.entry}</dt>
          <dd>{formatPrice(position.entry_price)}</dd>
        </div>
        <div>
          <dt>{copy.paper.exit}</dt>
          <dd>{formatPrice(position.exit_price)}</dd>
        </div>
        <div>
          <dt>{copy.paper.openedAt}</dt>
          <dd>{formatTimestamp(position.opened_at_ms)}</dd>
        </div>
        <div>
          <dt>{copy.paper.duration}</dt>
          <dd>{formatDuration(position.duration_ms)}</dd>
        </div>
      </dl>
      <div className="strategy-doctor-sample-reasons">
        <div>
          <span>{copy.paper.openReason}</span>
          <p>{position.reason || "—"}</p>
        </div>
        <div>
          <span>{copy.paper.closeReason}</span>
          <p>{position.close_reason || "—"}</p>
        </div>
      </div>
      {signalTags.length > 0 ? (
        <div className="strategy-doctor-signal-chips compact">
          {signalTags.map((tag) => (
            <span key={tag}>{tag}</span>
          ))}
        </div>
      ) : null}
      {decisionTags.length > 0 ? (
        <section className="reason-tag-section strategy-doctor-tag-section">
          <h3>{copy.trade.decisionTags}</h3>
          <ul>
            {decisionTags.map((tag, index) => (
              <li key={`${tag.kind}-${tag.label}-${tag.ts_ms}-${index}`}>
                <div>
                  <strong>{tag.label}</strong>
                  <span>
                    {copy.trade.scoreImpact}: {tag.score_impact >= 0 ? "+" : ""}
                    {tag.score_impact}
                  </span>
                </div>
                <p>{tag.reason}</p>
              </li>
            ))}
          </ul>
        </section>
      ) : null}
    </article>
  );
}

function StrategyCurveChart({
  ariaLabel,
  copy,
  curve,
  initialBalance,
  range,
  testId = "paper-strategy-recharts",
  version,
}: {
  ariaLabel?: string;
  copy: Copy;
  curve: StrategyCurve;
  initialBalance: number;
  range: StrategyCurveRange;
  testId?: string;
  version: string;
}) {
  const rawStartMs = curve.points[0].timestampMs;
  const rawEndMs = curve.points[curve.points.length - 1].timestampMs;
  const axisScale = strategyAxisScaleForRange(range, rawStartMs, rawEndMs);
  const axisDomain = [rawStartMs - axisScale.stepMs / 2, rawEndMs + axisScale.stepMs / 2];
  const axisTicks = buildStrategyAxisTicks(rawStartMs, rawEndMs, axisScale);
  const data = buildStrategyCurveChartData(curve.points, axisScale.stepMs, initialBalance);
  const equityAxisDomain = buildEquityAxisDomain(curve.points, initialBalance);
  const chart = (
    <AreaChart
      data={data}
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
        dataKey="timestampMs"
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
        allowDataOverflow
        domain={equityAxisDomain}
        tick={{ fill: "#607b96", fontSize: 10, fontWeight: 700 }}
        tickCount={5}
        tickFormatter={(value) => formatCompactUsdt(Number(value))}
        tickLine={false}
        width={76}
      />
      <Tooltip
        content={<StrategyCurveTooltip copy={copy} />}
        cursor={{ stroke: "rgba(18, 217, 156, 0.42)", strokeDasharray: "4 4" }}
        isAnimationActive={false}
      />
      <ReferenceLine
        stroke="rgba(148, 174, 196, 0.45)"
        strokeDasharray="6 6"
        strokeWidth={1.2}
        y={initialBalance}
      />
      <Area
        activeDot={{ fill: "#12d99c", r: 5, stroke: "#071017", strokeWidth: 2 }}
        connectNulls={false}
        dataKey="positiveEquity"
        dot={false}
        fill="url(#strategyPositiveFill)"
        isAnimationActive={false}
        name={copy.paper.equityGrowth}
        stroke="#12d99c"
        strokeWidth={2.6}
        type="linear"
      />
      <Area
        activeDot={{ fill: "#ff4d6d", r: 5, stroke: "#071017", strokeWidth: 2 }}
        connectNulls={false}
        dataKey="negativeEquity"
        dot={false}
        fill="url(#strategyNegativeFill)"
        isAnimationActive={false}
        name={copy.paper.equityDrawdown}
        stroke="#ff4d6d"
        strokeWidth={2.6}
        type="linear"
      />
    </AreaChart>
  );

  return (
    <div
      aria-label={ariaLabel ?? `${version} ${copy.paper.strategyCurve}`}
      className="paper-strategy-chart-wrap"
      data-axis-step-ms={axisScale.stepMs}
      data-point-count={curve.points.length}
      data-rendered-point-count={data.length}
      data-y-axis-max={equityAxisDomain[1]}
      data-y-axis-min={equityAxisDomain[0]}
      data-testid={testId}
      role="img"
    >
      <div className="paper-strategy-chart-legend" aria-hidden="true">
        <span className="paper-strategy-legend-positive">
          <i />
          {copy.paper.equityGrowth}
        </span>
        <span className="paper-strategy-legend-negative">
          <i />
          {copy.paper.equityDrawdown}
        </span>
        <span className="paper-strategy-legend-zero">
          <i />
          {copy.paper.initialBalance}
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
  payload,
}: {
  active?: boolean;
  copy: Copy;
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
      <strong>{point.synthetic ? copy.paper.initialBalance : formatChartDateTime(point.timestampMs)}</strong>
      <span>{formatChartDateTime(point.timestampMs)}</span>
      <dl>
        <Metric label={copy.paper.currentEquity} value={formatUsdt(point.equity)} />
        <Metric label={copy.paper.peakEquity} value={formatUsdt(point.highEquity)} />
        <Metric label={copy.paper.troughEquity} value={formatUsdt(point.lowEquity)} />
        <Metric label={copy.paper.equityChange} value={formatSignedUsdt(point.equityChange)} />
        <Metric label={copy.paper.realized} value={formatSignedUsdt(point.realizedPnl)} />
        <Metric label={copy.paper.unrealized} value={formatSignedUsdt(point.unrealizedPnl)} />
        <Metric
          label={copy.paper.openPositions}
          value={point.synthetic ? "-" : String(point.openPositionsCount)}
        />
      </dl>
    </div>
  );
}

function buildStrategyCurveChartData(
  points: StrategyCurvePoint[],
  stepMs: number,
  initialBalance: number,
): StrategyCurveChartPoint[] {
  const bucketedPoints = bucketStrategyCurvePoints(points, stepMs);
  return bucketedPoints.reduce<StrategyCurveChartPoint[]>((chartPoints, point, index) => {
    const previousPoint = bucketedPoints[index - 1];
    if (previousPoint && crossesBaseline(previousPoint.equity, point.equity, initialBalance)) {
      const distance =
        Math.abs(previousPoint.equity - initialBalance) + Math.abs(point.equity - initialBalance);
      const baselineRatio =
        distance === 0 ? 0 : Math.abs(previousPoint.equity - initialBalance) / distance;
      chartPoints.push({
        timestampMs: Math.round(
          previousPoint.timestampMs +
            (point.timestampMs - previousPoint.timestampMs) * baselineRatio,
        ),
        snapshotCount: 0,
        openEquity: initialBalance,
        highEquity: initialBalance,
        lowEquity: initialBalance,
        equity: initialBalance,
        equityChange: 0,
        negativeEquity: initialBalance,
        positiveEquity: initialBalance,
        realizedPnl: 0,
        unrealizedPnl: 0,
        openPositionsCount: 0,
        synthetic: true,
      });
    }
    chartPoints.push(toStrategyChartPoint(point, initialBalance));
    return chartPoints;
  }, []);
}

function bucketStrategyCurvePoints(
  points: StrategyCurvePoint[],
  stepMs: number,
): StrategyCurveChartPoint[] {
  const buckets = new Map<number, StrategyCurveChartPoint>();
  points.forEach((point) => {
    const bucketMs = Math.floor(point.timestampMs / stepMs) * stepMs;
    const existing = buckets.get(bucketMs);
    buckets.set(bucketMs, {
      timestampMs: bucketMs,
      snapshotCount: (existing?.snapshotCount ?? 0) + 1,
      openEquity: existing?.openEquity ?? point.openEquity,
      highEquity: Math.max(existing?.highEquity ?? point.highEquity, point.highEquity),
      lowEquity: Math.min(existing?.lowEquity ?? point.lowEquity, point.lowEquity),
      equity: point.equity,
      equityChange: point.equityChange,
      negativeEquity: null,
      positiveEquity: null,
      realizedPnl: point.realizedPnl,
      unrealizedPnl: point.unrealizedPnl,
      openPositionsCount: point.openPositionsCount,
    });
  });
  const bucketedPoints = Array.from(buckets.values()).sort(
    (left, right) => left.timestampMs - right.timestampMs,
  );
  const firstPoint = points[0];
  const lastPoint = points[points.length - 1];
  if (
    bucketedPoints.length === 1 &&
    firstPoint !== undefined &&
    lastPoint !== undefined &&
    firstPoint.timestampMs < lastPoint.timestampMs
  ) {
    return [
      strategyCurvePointForChart(firstPoint, 1),
      strategyCurvePointForChart(lastPoint, Math.max(1, points.length - 1)),
    ];
  }
  return bucketedPoints;
}

function strategyCurvePointForChart(
  point: StrategyCurvePoint,
  snapshotCount: number,
): StrategyCurveChartPoint {
  return {
    ...point,
    snapshotCount,
    negativeEquity: null,
    positiveEquity: null,
  };
}

function toStrategyChartPoint(
  point: StrategyCurveChartPoint,
  initialBalance: number,
): StrategyCurveChartPoint {
  const isBaseline = point.equity === initialBalance;
  return {
    ...point,
    negativeEquity: point.equity < initialBalance || isBaseline ? point.equity : null,
    positiveEquity: point.equity > initialBalance || isBaseline ? point.equity : null,
  };
}

function crossesBaseline(left: number, right: number, baseline: number): boolean {
  return (left < baseline && right > baseline) || (left > baseline && right < baseline);
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

function strategyAxisScaleForRange(
  range: StrategyCurveRange,
  startMs: number,
  endMs: number,
): StrategyAxisScale {
  switch (range) {
    case "1d":
      return { label: "10分钟", stepMs: 10 * MINUTE_MS, tickFormat: "time" };
    case "7d":
      return { label: "1小时", stepMs: HOUR_MS, tickFormat: "dateTime" };
    case "30d":
      return { label: "4小时", stepMs: 4 * HOUR_MS, tickFormat: "dateTime" };
    case "90d":
      return { label: "12小时", stepMs: 12 * HOUR_MS, tickFormat: "dateTime" };
    case "all":
      return chooseStrategyAxisScale(startMs, endMs);
  }
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
        <p className="muted panel-empty">{copy.paper.noPositionHistory}</p>
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

function sortStrategyStats(stats: PaperStrategyStats[]): PaperStrategyStats[] {
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
  history: PaperEquityPoint[],
  initialBalance: number,
): StrategyCurve {
  const points: StrategyCurvePoint[] = history
    .slice()
    .sort((left, right) => left.timestamp_ms - right.timestamp_ms)
    .map((point) => ({
      timestampMs: point.timestamp_ms,
      openEquity: point.equity,
      highEquity: point.equity,
      lowEquity: point.equity,
      equity: point.equity,
      equityChange: point.equity - initialBalance,
      realizedPnl: point.realized_pnl,
      unrealizedPnl: point.unrealized_pnl,
      openPositionsCount: point.open_positions_count,
    }));
  return anchorStrategyCurve(points, initialBalance);
}

function buildCurveForRange(
  fallbackCurve: StrategyCurve,
  equityCurves: PaperEquityCurves | undefined,
  range: StrategyCurveRange,
  initialBalance: number,
): StrategyCurve {
  const candles = equityCurves?.[range];
  return candles !== undefined && candles.length > 0
    ? buildStrategyCurveFromCandles(candles, initialBalance)
    : filterStrategyCurve(fallbackCurve, range);
}

function buildStrategyCurveFromCandles(
  candles: PaperEquityCandle[],
  initialBalance: number,
): StrategyCurve {
  const points = candles
    .slice()
    .sort((left, right) => left.bucket_start_ms - right.bucket_start_ms)
    .map<StrategyCurvePoint>((candle) => ({
      timestampMs: candle.bucket_start_ms,
      openEquity: candle.open_equity,
      highEquity: candle.high_equity,
      lowEquity: candle.low_equity,
      equity: candle.close_equity,
      equityChange: candle.close_equity - initialBalance,
      realizedPnl: candle.realized_pnl,
      unrealizedPnl: candle.unrealized_pnl,
      openPositionsCount: candle.open_positions_count,
    }));
  return anchorStrategyCurve(points, initialBalance);
}

function anchorStrategyCurve(
  points: StrategyCurvePoint[],
  initialBalance: number,
): StrategyCurve {
  const firstPoint = points[0];
  if (firstPoint !== undefined && !isInitialEquityPoint(firstPoint, initialBalance)) {
    const nextPoint = points[1];
    const inferredStepMs =
      nextPoint === undefined
        ? 10 * MINUTE_MS
        : Math.max(MINUTE_MS, Math.min(DAY_MS, nextPoint.timestampMs - firstPoint.timestampMs));
    points.unshift({
      timestampMs: firstPoint.timestampMs - inferredStepMs,
      openEquity: initialBalance,
      highEquity: initialBalance,
      lowEquity: initialBalance,
      equity: initialBalance,
      equityChange: 0,
      realizedPnl: 0,
      unrealizedPnl: 0,
      openPositionsCount: 0,
    });
  } else if (firstPoint !== undefined && points.length === 1) {
    points.push({
      ...firstPoint,
      timestampMs: firstPoint.timestampMs + 10 * MINUTE_MS,
    });
  }
  return summarizeEquityCurve(points, initialBalance);
}

function buildEquityAxisDomain(
  points: Array<Pick<StrategyCurvePoint, "equity" | "highEquity" | "lowEquity">>,
  initialBalance: number,
): [number, number] {
  const equities = [
    initialBalance,
    ...points.flatMap((point) => [point.lowEquity, point.equity, point.highEquity]),
  ].filter(Number.isFinite);
  const lowestEquity = Math.min(...equities);
  const highestEquity = Math.max(...equities);
  const spread = highestEquity - lowestEquity;
  const referenceEquity = Math.max(
    Math.abs(initialBalance),
    Math.abs(lowestEquity),
    Math.abs(highestEquity),
    1,
  );
  const padding = Math.max(
    spread * EQUITY_AXIS_PADDING_RATIO,
    referenceEquity * EQUITY_AXIS_MIN_PADDING_RATIO,
  );

  return [Math.max(0, lowestEquity - padding), highestEquity + padding];
}

function buildAccountEquityHistory(paper: PaperAccountSnapshot): PaperEquityPoint[] {
  const history = (paper.equity_history ?? [])
    .slice()
    .sort((left, right) => left.timestamp_ms - right.timestamp_ms);
  const latestHistoryPoint = history[history.length - 1];
  if (latestHistoryPoint !== undefined) {
    if (matchesCurrentAccountSnapshot(latestHistoryPoint, paper)) {
      return history;
    }
    return [
      ...history,
      {
        timestamp_ms: Math.max(
          latestHistoryPoint.timestamp_ms + MINUTE_MS,
          latestAccountTimestamp(paper),
        ),
        equity: paper.equity,
        realized_pnl: paper.realized_pnl,
        unrealized_pnl: paper.unrealized_pnl,
        open_positions_count: paper.positions.length,
      },
    ];
  }
  const latestTimestampMs = latestAccountTimestamp(paper);
  const currentTimestampMs = latestTimestampMs > 0 ? latestTimestampMs : Date.now();
  return [
    {
      timestamp_ms: currentTimestampMs - 10 * MINUTE_MS,
      equity: paper.initial_balance,
      realized_pnl: 0,
      unrealized_pnl: 0,
      open_positions_count: 0,
    },
    {
      timestamp_ms: currentTimestampMs,
      equity: paper.equity,
      realized_pnl: paper.realized_pnl,
      unrealized_pnl: paper.unrealized_pnl,
      open_positions_count: paper.positions.length,
    },
  ];
}

function latestAccountTimestamp(paper: PaperAccountSnapshot): number {
  return Math.max(
    paper.persistence.last_committed_at_ms ?? 0,
    ...paper.trades.map((trade) => trade.ts_ms),
    ...paper.positions.map((position) => position.opened_at_ms),
  );
}

function matchesCurrentAccountSnapshot(
  point: PaperEquityPoint,
  paper: PaperAccountSnapshot,
): boolean {
  const epsilon = 0.000001;
  return (
    Math.abs(point.equity - paper.equity) < epsilon &&
    Math.abs(point.realized_pnl - paper.realized_pnl) < epsilon &&
    Math.abs(point.unrealized_pnl - paper.unrealized_pnl) < epsilon &&
    point.open_positions_count === paper.positions.length
  );
}

function isInitialEquityPoint(point: StrategyCurvePoint, initialBalance: number): boolean {
  const epsilon = 0.000001;
  return (
    Math.abs(point.equity - initialBalance) < epsilon &&
    Math.abs(point.realizedPnl) < epsilon &&
    Math.abs(point.unrealizedPnl) < epsilon
  );
}

function summarizeEquityCurve(
  points: StrategyCurvePoint[],
  initialBalance: number,
): StrategyCurve {
  const startingEquity = points[0]?.openEquity ?? initialBalance;
  let runningPeak = startingEquity;
  let maxDrawdown = 0;
  let peakEquity = startingEquity;
  let troughEquity = startingEquity;
  points.forEach((point) => {
    runningPeak = Math.max(runningPeak, point.highEquity);
    maxDrawdown = Math.min(maxDrawdown, point.lowEquity - runningPeak);
    peakEquity = Math.max(peakEquity, point.highEquity);
    troughEquity = Math.min(troughEquity, point.lowEquity);
  });
  const currentEquity = points[points.length - 1]?.equity ?? initialBalance;

  return {
    currentEquity,
    equityChange: currentEquity - initialBalance,
    maxDrawdown,
    peakEquity,
    points,
    troughEquity,
  };
}

function filterStrategyCurve(curve: StrategyCurve, range: StrategyCurveRange): StrategyCurve {
  if (range === "all" || curve.points.length <= 1) {
    return curve;
  }
  const rangeMs =
    range === "1d"
      ? DAY_MS
      : range === "7d"
        ? 7 * DAY_MS
        : range === "30d"
          ? 30 * DAY_MS
          : 90 * DAY_MS;
  const endMs = curve.points[curve.points.length - 1].timestampMs;
  const points = curve.points.filter((point) => point.timestampMs >= endMs - rangeMs);
  return points.length === 0
    ? curve
    : summarizeEquityCurve(points, curve.currentEquity - curve.equityChange);
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
        positions: [...group].sort((left, right) => right.closed_at_ms - left.closed_at_ms),
        profitFactor,
        recommendationKey: signalRecommendation(confidence),
        sampleCount: group.length,
        signal,
        winRate,
      };
    })
    .sort((left, right) => right.netPnl - left.netPnl || right.sampleCount - left.sampleCount);
}

function signalAttributionComponents(positions: PaperClosedPositionSnapshot[]): string[] {
  return uniqueStrings(
    positions.flatMap((position) => [
      position.primary_signal ?? "",
      ...(position.signal_tags ?? []),
      ...strategyPositionTags(position).map((tag) => tag.label),
    ]),
  );
}

function strategyPositionTags(position: PaperClosedPositionSnapshot): TradeTag[] {
  return [
    ...(position.open_tags ?? []),
    ...(position.tags ?? []),
    ...(position.close_tags ?? []),
  ];
}

function uniqueStrings(values: string[]): string[] {
  return [...new Set(values.map((value) => value.trim()).filter(Boolean))];
}

function primarySignalLabel(position: PaperClosedPositionSnapshot): string {
  const tagLabel = strategyPositionTags(position)
    .map((tag) => tag.label.trim())
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
