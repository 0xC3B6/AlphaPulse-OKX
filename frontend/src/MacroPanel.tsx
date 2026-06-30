import { useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import {
  ColorType,
  createChart,
  type CandlestickData,
  type IChartApi,
  type ISeriesApi,
  type UTCTimestamp,
} from "lightweight-charts";
import { fetchBtcMacro } from "./api";
import type { Copy } from "./i18n";
import type {
  Ahr999History,
  Ahr999HistoryPoint,
  AnalogComparisonSet,
  AnalogPathSummary,
  BtcMacroSnapshot,
  ExternalMetricStatus,
  MacroRegime,
} from "./types";

export function MacroPanel({ copy, themeMode }: { copy: Copy; themeMode: string }) {
  const [snapshot, setSnapshot] = useState<BtcMacroSnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [ahrPageSize, setAhrPageSize] = useState(20);
  const [ahrPage, setAhrPage] = useState(0);
  const [analogPeriod, setAnalogPeriod] = useState(90);

  useEffect(() => {
    loadMacro();
  }, []);

  async function loadMacro() {
    setLoading(true);
    setError(null);
    try {
      setSnapshot(await fetchBtcMacro());
    } catch (requestError) {
      setError(requestError instanceof Error ? requestError.message : String(requestError));
    } finally {
      setLoading(false);
    }
  }

  if (loading && snapshot === null) {
    return (
      <section aria-live="polite" className="macro-panel macro-loading" data-testid="macro-loading">
        <div aria-hidden="true" className="macro-loading-pulse">
          <span />
          <span />
          <span />
        </div>
        <div className="macro-loading-body">
          <p>{copy.macro.loading}</p>
          <div aria-hidden="true" className="macro-loading-bars">
            <span />
            <span />
            <span />
          </div>
        </div>
      </section>
    );
  }

  if (error && snapshot === null) {
    return (
      <section className="macro-panel">
        <p className="paper-error">{error}</p>
        <button onClick={loadMacro} type="button">
          {copy.macro.refresh}
        </button>
      </section>
    );
  }

  if (snapshot === null) {
    return null;
  }

  return (
    <section className="macro-panel">
      <header className="macro-header">
        <div>
          <h2>{copy.macro.title}</h2>
          <p>{snapshot.summary}</p>
        </div>
        <button disabled={loading} onClick={loadMacro} type="button">
          {copy.macro.refresh}
        </button>
      </header>

      <section className="macro-regime-band">
        <div>
          <span>{copy.macro.regime}</span>
          <strong>{formatRegime(snapshot.regime, copy)}</strong>
        </div>
        <div>
          <span>{copy.macro.confidence}</span>
          <strong>{snapshot.confidence}/100</strong>
        </div>
        <div>
          <span>{copy.macro.price}</span>
          <strong>{formatUsd(snapshot.price)}</strong>
        </div>
      </section>

      <section className="macro-grid">
        <MetricTile
          label={copy.macro.daysSinceHalving}
          value={`${snapshot.cycle.days_since_halving}`}
        />
        <MetricTile
          label={copy.macro.cycleQuarter}
          value={`Q${snapshot.cycle.cycle_quarter}`}
        />
        <MetricTile
          label={copy.macro.cycleProgress}
          value={formatPct(snapshot.cycle.estimated_cycle_progress_pct)}
        />
        <MetricTile
          label={copy.macro.drawdown}
          value={formatPct(snapshot.trend.drawdown_from_window_ath_pct)}
          tone={snapshot.trend.drawdown_from_window_ath_pct < -0.3 ? "negative" : undefined}
        />
        <MetricTile
          label={copy.macro.ma200w}
          value={snapshot.trend.ma_200w === null ? "-" : formatUsd(snapshot.trend.ma_200w)}
        />
        <MetricTile
          label={copy.macro.priceVsMa200w}
          value={
            snapshot.trend.price_vs_200w_pct === null
              ? "-"
              : formatPct(snapshot.trend.price_vs_200w_pct)
          }
          tone={
            snapshot.trend.price_vs_200w_pct !== null && snapshot.trend.price_vs_200w_pct < 0
              ? "negative"
              : "positive"
          }
        />
        <MetricTile
          label={copy.macro.change90d}
          value={snapshot.momentum.change_90d_pct === null ? "-" : formatPct(snapshot.momentum.change_90d_pct)}
          tone={
            snapshot.momentum.change_90d_pct !== null && snapshot.momentum.change_90d_pct < 0
              ? "negative"
              : "positive"
          }
        />
        <MetricTile
          label={copy.macro.volatility90d}
          value={
            snapshot.momentum.volatility_90d_pct === null
              ? "-"
              : formatPct(snapshot.momentum.volatility_90d_pct)
          }
        />
      </section>

      <section className="macro-progress">
        <div>
          <span>{copy.macro.lastHalving}</span>
          <strong>{formatDate(snapshot.cycle.last_halving_ms)}</strong>
        </div>
        <div className="macro-progress-track">
          <span
            style={{
              width: `${Math.min(
                100,
                Math.max(0, snapshot.cycle.estimated_cycle_progress_pct * 100),
              )}%`,
            }}
          />
        </div>
        <div>
          <span>{copy.macro.nextHalving}</span>
          <strong>{formatDate(snapshot.cycle.next_halving_estimate_ms)}</strong>
        </div>
      </section>

      <div className="macro-columns">
        <section>
          <h3>{copy.macro.events}</h3>
          <ul className="macro-list">
            {snapshot.events.map((event) => (
              <li key={event.id}>
                <strong>{event.title}</strong>
                <span>
                  {formatDate(event.date_ms)} · {event.days_to_event} {copy.macro.daysToEvent}
                </span>
                <em>{formatSnake(event.phase)}</em>
              </li>
            ))}
          </ul>
        </section>

        <section>
          <h3>{copy.macro.valuationMetrics}</h3>
          <ul className="macro-list">
            {snapshot.valuation_metrics.map((metric) => (
              <li key={metric.id}>
                <strong>{metric.name}</strong>
                <span>{formatValuationMetric(metric)}</span>
                <em>{formatMetricMeta(metric)}</em>
              </li>
            ))}
          </ul>
        </section>
      </div>

      <Ahr999HistorySection
        copy={copy}
        history={snapshot.ahr999_history ?? null}
        page={ahrPage}
        pageSize={ahrPageSize}
        setPage={setAhrPage}
        setPageSize={(value) => {
          setAhrPageSize(value);
          setAhrPage(0);
        }}
      />

      <AnalogComparisonSection
        comparisons={snapshot.analog_comparisons ?? []}
        copy={copy}
        selectedPeriod={analogPeriod}
        setSelectedPeriod={setAnalogPeriod}
        themeMode={themeMode}
      />

      <div className="macro-columns">
        <section>
          <h3>{copy.macro.analogs}</h3>
          <ul className="macro-list">
            {snapshot.analogs.map((analog) => (
              <li key={analog.label}>
                <strong>
                  {formatSnake(analog.label)} · {analog.score}/100
                </strong>
                <span>{analog.rationale.join(" / ")}</span>
                {analog.components && analog.components.length > 0 ? (
                  <div className="macro-score-components">
                    {analog.components.map((component) => (
                      <small key={component.label}>
                        {formatSnake(component.label)} {component.points}/{component.max_points}
                      </small>
                    ))}
                  </div>
                ) : null}
              </li>
            ))}
          </ul>
        </section>

        <section>
          <h3>{copy.macro.tradingBias}</h3>
          <ul className="macro-list">
            {snapshot.trading_bias.map((bias) => (
              <li key={bias}>
                <span>{formatSnake(bias)}</span>
              </li>
            ))}
          </ul>
        </section>
      </div>
    </section>
  );
}

function MetricTile({
  label,
  tone,
  value,
}: {
  label: string;
  tone?: "positive" | "negative";
  value: string;
}) {
  return (
    <div className="macro-metric">
      <span>{label}</span>
      <strong className={tone ?? ""}>{value}</strong>
    </div>
  );
}

function Ahr999HistorySection({
  copy,
  history,
  page,
  pageSize,
  setPage,
  setPageSize,
}: {
  copy: Copy;
  history: Ahr999History | null;
  page: number;
  pageSize: number;
  setPage: (page: number) => void;
  setPageSize: (pageSize: number) => void;
}) {
  const points = history?.points ?? [];
  const hasHistory = points.length > 0;
  const bounds = hasHistory
    ? resolveAhrDateBounds(points)
    : {
        startTsMs: Date.UTC(2009, 0, 3),
        endTsMs: Date.UTC(2009, 0, 3),
        startDate: "2009-01-03",
        endDate: "2009-01-03",
      };
  const rangeOptions = [
    { id: "all", label: copy.macro.rangeAll, days: null },
    { id: "180d", label: copy.macro.range6m, days: 180 },
    { id: "365d", label: copy.macro.range1y, days: 365 },
    { id: "1095d", label: copy.macro.range3y, days: 1095 },
    { id: "1825d", label: copy.macro.range5y, days: 1825 },
  ] as const;
  const [rangePreset, setRangePreset] = useState<(typeof rangeOptions)[number]["id"] | "custom">(
    "all",
  );
  const [customStart, setCustomStart] = useState(() => bounds.startDate);
  const [customEnd, setCustomEnd] = useState(() => bounds.endDate);

  useEffect(() => {
    if (!hasHistory) {
      return;
    }
    setRangePreset("all");
    setCustomStart(bounds.startDate);
    setCustomEnd(bounds.endDate);
  }, [bounds.endDate, bounds.startDate, hasHistory]);

  useEffect(() => {
    setPage(0);
  }, [customEnd, customStart, setPage]);

  const filteredPoints = useMemo(
    () =>
      points.filter(
        (point) =>
          normalizeHistoryDate(point.date) >= customStart &&
          normalizeHistoryDate(point.date) <= customEnd,
      ),
    [customEnd, customStart, points],
  );
  const rows = useMemo(() => [...filteredPoints].reverse(), [filteredPoints]);
  const pageCount = Math.max(1, Math.ceil(rows.length / pageSize));
  const safePage = Math.min(page, pageCount - 1);
  const pageRows = rows.slice(safePage * pageSize, safePage * pageSize + pageSize);
  const bandSummaries = summarizeAhrBands(filteredPoints, history?.bands ?? []);

  if (!hasHistory) {
    return (
      <section className="macro-detail-section">
        <h3>{copy.macro.ahr999History}</h3>
        <p className="macro-empty">{copy.macro.ahr999NoHistory}</p>
      </section>
    );
  }

  function selectRangePreset(nextPreset: (typeof rangeOptions)[number]["id"]) {
    const nextRange = resolveAhrRange(bounds, nextPreset, customStart, customEnd, rangeOptions);
    setRangePreset(nextPreset);
    setCustomStart(findNearestHistoryDate(points, nextRange.startTsMs, "start"));
    setCustomEnd(findNearestHistoryDate(points, nextRange.endTsMs, "end"));
  }

  function updateCustomRange(field: "start" | "end", value: string) {
    const nextStart = field === "start" ? value : customStart;
    const nextEnd = field === "end" ? value : customEnd;
    setRangePreset("custom");
    setCustomStart(nextStart);
    setCustomEnd(nextEnd);
  }

  function applyOverviewRange(startDate: string, endDate: string) {
    setRangePreset("custom");
    setCustomStart(startDate);
    setCustomEnd(endDate);
  }

  return (
    <section className="macro-detail-section">
      <div className="macro-section-header">
        <div>
          <h3>{copy.macro.ahr999History}</h3>
          <p>{history?.source}</p>
        </div>
      </div>
      <div className="macro-ahr-range-toolbar">
        <div className="macro-ahr-range-presets" role="group" aria-label={copy.macro.range}>
          {rangeOptions.map((option) => (
            <button
              className={rangePreset === option.id ? "active" : ""}
              key={option.id}
              onClick={() => selectRangePreset(option.id)}
              type="button"
            >
              {option.label}
            </button>
          ))}
        </div>
        <div className="macro-ahr-range-inputs">
          <label>
            <span>{copy.macro.startDate}</span>
            <input
              aria-label={copy.macro.startDate}
              max={customEnd}
              min={bounds.startDate}
              onChange={(event) => updateCustomRange("start", event.target.value)}
              type="date"
              value={customStart}
            />
          </label>
          <label>
            <span>{copy.macro.endDate}</span>
            <input
              aria-label={copy.macro.endDate}
              max={bounds.endDate}
              min={customStart}
              onChange={(event) => updateCustomRange("end", event.target.value)}
              type="date"
              value={customEnd}
            />
          </label>
        </div>
      </div>
      <div className="macro-ahr-chart-card">
        <Ahr999CompositeChart
          allPoints={points}
          copy={copy}
          onRangeChange={applyOverviewRange}
          points={filteredPoints}
          rangeEndTsMs={filteredPoints[filteredPoints.length - 1]?.ts_ms ?? bounds.endTsMs}
          rangeStartTsMs={filteredPoints[0]?.ts_ms ?? bounds.startTsMs}
        />
      </div>
      <div className="macro-ahr-summary-grid">
        {bandSummaries.map((band) => (
          <div className="macro-ahr-summary-card" key={band.label}>
            <strong>{band.label}</strong>
            <span>
              {band.days} {copy.macro.historyDays}
            </span>
            <p>{band.recommendation}</p>
          </div>
        ))}
      </div>
      <div className="macro-table-toolbar">
        <label>
          <span>{copy.macro.historyRowsPerPage}</span>
          <select
            aria-label={copy.macro.historyRowsPerPage}
            onChange={(event) => setPageSize(Number(event.target.value))}
            value={pageSize}
          >
            {[20, 50, 100].map((value) => (
              <option key={value} value={value}>
                {value}
              </option>
            ))}
          </select>
        </label>
        <div className="macro-page-controls">
          <button disabled={safePage === 0} onClick={() => setPage(safePage - 1)} type="button">
            ‹
          </button>
          <span>
            {copy.macro.historyPage} {safePage + 1}/{pageCount}
          </span>
          <button
            disabled={safePage >= pageCount - 1}
            onClick={() => setPage(safePage + 1)}
            type="button"
          >
            ›
          </button>
        </div>
      </div>
      <div className="macro-table-wrap">
        <table className="macro-data-table">
          <thead>
            <tr>
              <th>{copy.macro.timeColumn}</th>
              <th>{copy.macro.ahr999IndexLabel}</th>
              <th>{copy.macro.btcPriceLabel}</th>
              <th>{copy.macro.cost200Label}</th>
            </tr>
          </thead>
          <tbody>
            {pageRows.map((point) => (
              <tr key={point.ts_ms}>
                <td>{point.date}</td>
                <td>{point.value.toFixed(8)}</td>
                <td>{formatUsd(point.btc_price)}</td>
                <td>{formatUsd(point.gma200)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}

function AnalogComparisonSection({
  comparisons,
  copy,
  selectedPeriod,
  setSelectedPeriod,
  themeMode,
}: {
  comparisons: AnalogComparisonSet[];
  copy: Copy;
  selectedPeriod: number;
  setSelectedPeriod: (period: number) => void;
  themeMode: string;
}) {
  const periods = [30, 90, 180, 365];
  const selected =
    comparisons.find((comparison) => comparison.timeframe_days === selectedPeriod) ??
    comparisons.find((comparison) => comparison.timeframe_days === 90) ??
    comparisons[0];

  return (
    <section className="macro-detail-section">
      <div className="macro-section-header">
        <div>
          <h3>{copy.macro.analogComparison}</h3>
          <p>
            {selected?.current
              ? `${copy.macro.currentLookback} ${selected.timeframe_days}D -> ${copy.macro.historicalForward}`
              : copy.macro.noAnalogMatches}
          </p>
        </div>
        <div className="macro-period-tabs" role="group" aria-label={copy.macro.analogComparison}>
          {periods.map((period) => (
            <button
              className={selectedPeriod === period ? "active" : ""}
              key={period}
              onClick={() => setSelectedPeriod(period)}
              type="button"
            >
              {period}D
            </button>
          ))}
        </div>
      </div>
      {selected?.current ? (
        <>
          <div className="macro-kline-grid">
            <KlineComparisonCard
              ariaLabel={`${copy.macro.currentLookback} ${selected.timeframe_days}D K线`}
              title={`${copy.macro.currentLookback} ${selected.timeframe_days}D`}
              themeMode={themeMode}
              window={selected.current}
            />
            {selected.matches.map((match) => (
              <KlineComparisonCard
                ariaLabel={`${formatForwardTitle(copy, match.label)} K线`}
                key={match.id}
                themeMode={themeMode}
                title={`${formatForwardTitle(copy, match.label)} · ${match.score}/100`}
                window={match.forward ?? match.lookback}
              >
                <div className="macro-kline-meta">
                  <span>
                    final {formatPct(match.final_return_pct)} · drawdown{" "}
                    {formatPct(match.max_drawdown_pct)} · runup {formatPct(match.max_runup_pct)}
                  </span>
                  <div className="macro-score-components">
                    {match.components.map((component) => (
                      <small key={component.label}>
                        {formatSnake(component.label)} {component.points}/{component.max_points}
                      </small>
                    ))}
                  </div>
                </div>
              </KlineComparisonCard>
            ))}
          </div>
          {selected.matches.length > 0 ? (
            <ul className="macro-list macro-analog-list">
              {selected.matches.map((match) => (
                <li key={match.id}>
                  <strong>
                    {formatForwardTitle(copy, match.label)} · {match.score}/100
                  </strong>
                  <span>
                    final {formatPct(match.final_return_pct)} · drawdown{" "}
                    {formatPct(match.max_drawdown_pct)} · runup {formatPct(match.max_runup_pct)}
                  </span>
                  <div className="macro-score-components">
                    {match.components.map((component) => (
                      <small key={component.label}>
                        {formatSnake(component.label)} {component.points}/{component.max_points}
                      </small>
                    ))}
                  </div>
                </li>
              ))}
            </ul>
          ) : (
            <p className="macro-empty">{copy.macro.noAnalogMatches}</p>
          )}
        </>
      ) : (
        <p className="macro-empty">{copy.macro.noAnalogMatches}</p>
      )}
    </section>
  );
}

function KlineComparisonCard({
  ariaLabel,
  children,
  themeMode,
  title,
  window,
}: {
  ariaLabel: string;
  children?: ReactNode;
  themeMode: string;
  title: string;
  window: AnalogPathSummary;
}) {
  const priceSummary = summarizeKlinePrices(window);

  return (
    <div className="macro-kline-card">
      <h4>{title}</h4>
      <InteractiveKlineChart ariaLabel={ariaLabel} themeMode={themeMode} window={window} />
      <div className="macro-kline-meta">
        {priceSummary ? <span>{priceSummary}</span> : null}
        <span>
          final {formatPct(window.final_return_pct)} · drawdown {formatPct(window.max_drawdown_pct)}{" "}
          · runup {formatPct(window.max_runup_pct)}
        </span>
      </div>
      {children}
    </div>
  );
}

function InteractiveKlineChart({
  ariaLabel,
  themeMode,
  window,
}: {
  ariaLabel: string;
  themeMode: string;
  window: AnalogPathSummary;
}) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const seriesRef = useRef<ISeriesApi<"Candlestick"> | null>(null);

  useEffect(() => {
    const container = containerRef.current;
    if (!container || import.meta.env.MODE === "test") {
      return;
    }

    const chart = createChart(container, {
      autoSize: true,
      height: 220,
      layout: chartLayoutOptions(),
      grid: chartGridOptions(),
      handleScale: true,
      handleScroll: true,
      rightPriceScale: {
        borderColor: cssVar("--border"),
        visible: true,
      },
      timeScale: {
        borderColor: cssVar("--border"),
        timeVisible: true,
      },
    });
    const series = chart.addCandlestickSeries({
      upColor: "#16a06a",
      downColor: "#d94c43",
      borderUpColor: "#16a06a",
      borderDownColor: "#d94c43",
      wickUpColor: "#16a06a",
      wickDownColor: "#d94c43",
      priceFormat: {
        type: "price",
        precision: 2,
        minMove: 0.01,
      },
    });

    chartRef.current = chart;
    seriesRef.current = series;

    return () => {
      chart.remove();
      chartRef.current = null;
      seriesRef.current = null;
    };
  }, []);

  useEffect(() => {
    const chart = chartRef.current;
    if (!chart) {
      return;
    }

    chart.applyOptions({
      layout: chartLayoutOptions(),
      grid: chartGridOptions(),
      rightPriceScale: {
        borderColor: cssVar("--border"),
        visible: true,
      },
      timeScale: {
        borderColor: cssVar("--border"),
      },
    });
  }, [themeMode]);

  useEffect(() => {
    const chart = chartRef.current;
    const series = seriesRef.current;
    if (!chart || !series) {
      return;
    }

    series.setData(
      window.candles.map((candle): CandlestickData => ({
        time: toChartTime(candle.ts_ms),
        open: candle.open,
        high: candle.high,
        low: candle.low,
        close: candle.close,
      })),
    );
    chart.timeScale().fitContent();
  }, [window]);

  return (
    <div aria-label={ariaLabel} className="macro-kline-chart" role="img">
      <div className="macro-kline-canvas" ref={containerRef} />
      {import.meta.env.MODE === "test" ? (
        <div className="macro-kline-test-fallback">
          {window.candles.slice(0, 3).map((candle) => (
            <span key={candle.ts_ms}>{formatUsd(candle.close)}</span>
          ))}
        </div>
      ) : null}
    </div>
  );
}

function formatForwardTitle(copy: Copy, label: string): string {
  return `${copy.macro.historicalForward} ${label.replace(/^after\s+/i, "")}`;
}

function summarizeKlinePrices(window: AnalogPathSummary): string | null {
  const first = window.candles[0];
  const last = window.candles[window.candles.length - 1];
  if (!first || !last) {
    return null;
  }
  const low = Math.min(...window.candles.map((candle) => candle.low));
  const high = Math.max(...window.candles.map((candle) => candle.high));
  return `price ${formatUsd(first.open)} -> ${formatUsd(last.close)} · range ${formatUsd(low)}-${formatUsd(high)}`;
}

function chartLayoutOptions() {
  return {
    background: { type: ColorType.Solid, color: cssVar("--surface") },
    textColor: cssVar("--text-muted"),
  };
}

function chartGridOptions() {
  return {
    horzLines: { color: cssVar("--border-subtle") },
    vertLines: { color: cssVar("--border-subtle") },
  };
}

function cssVar(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

function toChartTime(tsMs: number): UTCTimestamp {
  return Math.floor(tsMs / 1000) as UTCTimestamp;
}

function Ahr999CompositeChart({
  allPoints,
  copy,
  onRangeChange,
  points,
  rangeEndTsMs,
  rangeStartTsMs,
}: {
  allPoints: Ahr999HistoryPoint[];
  copy: Copy;
  onRangeChange: (startDate: string, endDate: string) => void;
  points: Ahr999HistoryPoint[];
  rangeEndTsMs: number;
  rangeStartTsMs: number;
}) {
  if (points.length === 0 || allPoints.length === 0) {
    return null;
  }
  const containerRef = useRef<HTMLDivElement | null>(null);
  const dragStateRef = useRef<{
    mode: "start" | "end" | "window";
    anchorIndex: number;
    startIndex: number;
    endIndex: number;
  } | null>(null);
  const [dragging, setDragging] = useState(false);
  const width = 1240;
  const height = 540;
  const leftAxisWidth = 88;
  const rightAxisWidth = 94;
  const chartTop = 56;
  const chartBottom = 338;
  const overviewTop = 398;
  const overviewBottom = 456;
  const plotLeft = leftAxisWidth;
  const plotRight = width - rightAxisWidth;
  const plotWidth = plotRight - plotLeft;
  const plotBottom = chartBottom;
  const minTsMs = points[0].ts_ms;
  const maxTsMs = points[points.length - 1].ts_ms;
  const allMinTsMs = allPoints[0].ts_ms;
  const allMaxTsMs = allPoints[allPoints.length - 1].ts_ms;
  const ahrDomain = buildLogDomain([
    ...points.map((point) => point.value),
    0.45,
    1.2,
  ]);
  const priceDomain = buildLogDomain(
    points.flatMap((point) => [point.btc_price, point.gma200]),
  );
  const overviewPriceDomain = buildLogDomain(allPoints.map((point) => point.btc_price));
  const leftTicks = buildLogTicks(ahrDomain.min, ahrDomain.max);
  const rightTicks = buildLogTicks(priceDomain.min, priceDomain.max);
  const timeTicks = buildTimeTicks(minTsMs, maxTsMs, 6);
  const gridYs = mergeNearbyTicks([
    ...leftTicks.map((value) => yCoordLog(value, ahrDomain.min, ahrDomain.max, chartTop, plotBottom)),
    ...rightTicks.map((value) => yCoordLog(value, priceDomain.min, priceDomain.max, chartTop, plotBottom)),
  ]);
  const rangeStartIndex = findPointIndexByTs(allPoints, rangeStartTsMs);
  const rangeEndIndex = findPointIndexByTs(allPoints, rangeEndTsMs);

  function xCoord(tsMs: number) {
    if (maxTsMs === minTsMs) {
      return plotLeft + plotWidth / 2;
    }
    return plotLeft + ((tsMs - minTsMs) / (maxTsMs - minTsMs)) * plotWidth;
  }

  function overviewXCoord(tsMs: number) {
    if (allMaxTsMs === allMinTsMs) {
      return plotLeft + plotWidth / 2;
    }
    return plotLeft + ((tsMs - allMinTsMs) / (allMaxTsMs - allMinTsMs)) * plotWidth;
  }

  function updateOverviewRange(startIndex: number, endIndex: number) {
    const safeStartIndex = clamp(startIndex, 0, allPoints.length - 1);
    const safeEndIndex = clamp(endIndex, safeStartIndex, allPoints.length - 1);
    onRangeChange(
      normalizeHistoryDate(allPoints[safeStartIndex].date),
      normalizeHistoryDate(allPoints[safeEndIndex].date),
    );
  }

  function beginOverviewDrag(
    mode: "start" | "end" | "window",
    event: React.PointerEvent<SVGElement>,
  ) {
    event.preventDefault();
    const anchorIndex =
      mode === "start"
        ? rangeStartIndex
        : mode === "end"
          ? rangeEndIndex
          : findNearestOverviewIndex(
              containerRef.current,
              event.clientX,
              plotLeft,
              plotWidth,
              allPoints.length,
            );
    dragStateRef.current = {
      mode,
      anchorIndex,
      startIndex: rangeStartIndex,
      endIndex: rangeEndIndex,
    };
    setDragging(true);
  }

  useEffect(() => {
    if (!dragging) {
      return;
    }

    function handlePointerMove(event: PointerEvent) {
      const dragState = dragStateRef.current;
      if (!dragState) {
        return;
      }
      const nextIndex = findNearestOverviewIndex(
        containerRef.current,
        event.clientX,
        plotLeft,
        plotWidth,
        allPoints.length,
      );

      if (dragState.mode === "start") {
        updateOverviewRange(Math.min(nextIndex, dragState.endIndex - 1), dragState.endIndex);
        return;
      }

      if (dragState.mode === "end") {
        updateOverviewRange(dragState.startIndex, Math.max(nextIndex, dragState.startIndex + 1));
        return;
      }

      const widthInPoints = dragState.endIndex - dragState.startIndex;
      const indexDelta = nextIndex - dragState.anchorIndex;
      const nextStartIndex = clamp(
        dragState.startIndex + indexDelta,
        0,
        Math.max(0, allPoints.length - 1 - widthInPoints),
      );
      updateOverviewRange(nextStartIndex, nextStartIndex + widthInPoints);
    }

    function handlePointerUp() {
      dragStateRef.current = null;
      setDragging(false);
    }

    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", handlePointerUp);
    return () => {
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerUp);
    };
  }, [allPoints, dragging, onRangeChange, plotLeft, plotWidth, rangeEndIndex, rangeStartIndex]);

  return (
    <div className={dragging ? "macro-ahr-composite is-dragging" : "macro-ahr-composite"} ref={containerRef}>
      <div className="macro-ahr-legend">
        {[
          { label: copy.macro.cost200Label, color: "#d4d8e2" },
          { label: copy.macro.btcPriceLabel, color: "#e5b84d" },
          { label: copy.macro.ahr999IndexLabel, color: "#5b8cff" },
          { label: copy.macro.buyBottomLabel, color: "#ff5576" },
          { label: copy.macro.fixedInvestLabel, color: "#68c36d" },
        ].map((item) => (
          <span key={item.label}>
            <i style={{ backgroundColor: item.color }} />
            {item.label}
          </span>
        ))}
      </div>
      <svg className="macro-ahr-svg" role="img" viewBox={`0 0 ${width} ${height}`}>
        <rect
          className="macro-ahr-zone-fill"
          height={plotBottom - yCoordLog(0.45, ahrDomain.min, ahrDomain.max, chartTop, plotBottom)}
          width={plotWidth}
          x={plotLeft}
          y={yCoordLog(0.45, ahrDomain.min, ahrDomain.max, chartTop, plotBottom)}
        />
        {gridYs.map((y, index) => (
          <line
            className={index === 0 || index === gridYs.length - 1 ? "macro-chart-gridline--axis" : "macro-chart-gridline"}
            key={`grid-${index}`}
            x1={plotLeft}
            x2={plotRight}
            y1={y}
            y2={y}
          />
        ))}
        {timeTicks.map((tick, index) => (
          <g key={`x-grid-${index}`}>
            <line
              className={
                index === 0 || index === timeTicks.length - 1
                  ? "macro-chart-gridline--axis"
                  : "macro-chart-gridline"
              }
              x1={xCoord(tick)}
              x2={xCoord(tick)}
              y1={chartTop}
              y2={plotBottom}
            />
            <text className="macro-chart-x-label" x={xCoord(tick)} y={plotBottom + 26}>
              {formatChartDate(tick)}
            </text>
          </g>
        ))}
        <line
          className="macro-reference-line macro-reference-line--buy"
          x1={plotLeft}
          x2={plotRight}
          y1={yCoordLog(0.45, ahrDomain.min, ahrDomain.max, chartTop, plotBottom)}
          y2={yCoordLog(0.45, ahrDomain.min, ahrDomain.max, chartTop, plotBottom)}
        />
        <line
          className="macro-reference-line macro-reference-line--fixed"
          x1={plotLeft}
          x2={plotRight}
          y1={yCoordLog(1.2, ahrDomain.min, ahrDomain.max, chartTop, plotBottom)}
          y2={yCoordLog(1.2, ahrDomain.min, ahrDomain.max, chartTop, plotBottom)}
        />
        <text
          className="macro-reference-label"
          x={plotRight - 8}
          y={yCoordLog(0.45, ahrDomain.min, ahrDomain.max, chartTop, plotBottom) - 6}
        >
          0.45
        </text>
        <text
          className="macro-reference-label"
          x={plotRight - 8}
          y={yCoordLog(1.2, ahrDomain.min, ahrDomain.max, chartTop, plotBottom) - 6}
        >
          1.2
        </text>
        <path
          className="macro-ahr-line macro-ahr-line--cost"
          d={linePath(
            points.map((point) => ({
              x: xCoord(point.ts_ms),
              y: yCoordLog(point.gma200, priceDomain.min, priceDomain.max, chartTop, plotBottom),
            })),
          )}
        />
        <path
          className="macro-ahr-line macro-ahr-line--btc"
          d={linePath(
            points.map((point) => ({
              x: xCoord(point.ts_ms),
              y: yCoordLog(point.btc_price, priceDomain.min, priceDomain.max, chartTop, plotBottom),
            })),
          )}
        />
        <path
          className="macro-ahr-line macro-ahr-line--ahr"
          d={linePath(
            points.map((point) => ({
              x: xCoord(point.ts_ms),
              y: yCoordLog(point.value, ahrDomain.min, ahrDomain.max, chartTop, plotBottom),
            })),
          )}
        />
        <circle
          className="macro-ahr-point macro-ahr-point--btc"
          cx={xCoord(points[points.length - 1].ts_ms)}
          cy={yCoordLog(points[points.length - 1].btc_price, priceDomain.min, priceDomain.max, chartTop, plotBottom)}
          r={4}
        />
        <circle
          className="macro-ahr-point macro-ahr-point--cost"
          cx={xCoord(points[points.length - 1].ts_ms)}
          cy={yCoordLog(points[points.length - 1].gma200, priceDomain.min, priceDomain.max, chartTop, plotBottom)}
          r={3.6}
        />
        <circle
          className="macro-ahr-point macro-ahr-point--ahr"
          cx={xCoord(points[points.length - 1].ts_ms)}
          cy={yCoordLog(points[points.length - 1].value, ahrDomain.min, ahrDomain.max, chartTop, plotBottom)}
          r={4}
        />
        {leftTicks.map((tick, index) => (
          <text
            key={`left-${index}`}
            className="macro-chart-value-label"
            x={plotLeft - 10}
            y={yCoordLog(tick, ahrDomain.min, ahrDomain.max, chartTop, plotBottom) + 4}
          >
            {formatAhrAxisValue(tick)}
          </text>
        ))}
        {rightTicks.map((tick, index) => (
          <text
            key={`right-${index}`}
            className="macro-chart-value-label macro-chart-value-label--right"
            x={plotRight + 10}
            y={yCoordLog(tick, priceDomain.min, priceDomain.max, chartTop, plotBottom) + 4}
          >
            {formatCompactUsdAxis(tick)}
          </text>
        ))}
        <rect
          className="macro-ahr-overview-track"
          height={overviewBottom - overviewTop}
          rx={10}
          width={plotWidth}
          x={plotLeft}
          y={overviewTop}
        />
        <path
          className="macro-ahr-overview-area"
          d={areaPath(
            allPoints.map((point) => ({
              x: overviewXCoord(point.ts_ms),
              y: yCoordLog(
                point.btc_price,
                overviewPriceDomain.min,
                overviewPriceDomain.max,
                overviewTop + 4,
                overviewBottom - 4,
              ),
            })),
            overviewBottom,
          )}
        />
        <rect
          className="macro-ahr-overview-selection"
          height={overviewBottom - overviewTop}
          onPointerDown={(event) => beginOverviewDrag("window", event)}
          rx={10}
          width={Math.max(18, overviewXCoord(rangeEndTsMs) - overviewXCoord(rangeStartTsMs))}
          x={overviewXCoord(rangeStartTsMs)}
          y={overviewTop}
        />
        <circle
          className="macro-ahr-overview-handle"
          cx={overviewXCoord(rangeStartTsMs)}
          cy={(overviewTop + overviewBottom) / 2}
          onPointerDown={(event) => beginOverviewDrag("start", event)}
          r={12}
        />
        <circle
          className="macro-ahr-overview-handle"
          cx={overviewXCoord(rangeEndTsMs)}
          cy={(overviewTop + overviewBottom) / 2}
          onPointerDown={(event) => beginOverviewDrag("end", event)}
          r={12}
        />
        <text className="macro-ahr-overview-label" x={plotLeft} y={overviewBottom + 22}>
          {formatChartDate(allMinTsMs)}
        </text>
        <text className="macro-ahr-overview-label macro-ahr-overview-label--right" x={plotRight} y={overviewBottom + 22}>
          {formatChartDate(allMaxTsMs)}
        </text>
      </svg>
    </div>
  );
}

function resolveAhrDateBounds(points: Ahr999HistoryPoint[]) {
  const firstPoint = points[0];
  const lastPoint = points[points.length - 1];
  return {
    startTsMs: firstPoint.ts_ms,
    endTsMs: lastPoint.ts_ms,
    startDate: normalizeHistoryDate(firstPoint.date),
    endDate: normalizeHistoryDate(lastPoint.date),
  };
}

function resolveAhrRange(
  bounds: { startTsMs: number; endTsMs: number },
  preset: "all" | "180d" | "365d" | "1095d" | "1825d" | "custom",
  _customStart: string,
  _customEnd: string,
  options: ReadonlyArray<{ id: "all" | "180d" | "365d" | "1095d" | "1825d"; days: number | null }>,
) {
  if (preset === "custom") {
    return bounds;
  }

  const matched = options.find((option) => option.id === preset);
  if (!matched || matched.days === null) {
    return bounds;
  }

  return {
    startTsMs: Math.max(bounds.startTsMs, bounds.endTsMs - (matched.days - 1) * 86_400_000),
    endTsMs: bounds.endTsMs,
  };
}

function summarizeAhrBands(points: Ahr999HistoryPoint[], bands: Ahr999History["bands"]) {
  const byId = Object.fromEntries(bands.map((band) => [band.id, band]));
  const deepValueDays = points.filter((point) => point.value < 0.45).length;
  const accumulationDays = points.filter((point) => point.value >= 0.45 && point.value < 1.2).length;
  const aboveDays = points.filter((point) => point.value >= 1.2).length;
  const overheatedDays = points.filter((point) => point.value >= 5).length;

  return [
    {
      label: "AHR999 > 1.2",
      days: aboveDays,
      recommendation:
        overheatedDays > 0
          ? `${byId.neutral?.recommendation ?? ""} · overheated ${overheatedDays} days`
          : byId.neutral?.recommendation ?? "",
    },
    {
      label: "AHR999 0.45 - 1.2",
      days: accumulationDays,
      recommendation: byId.accumulation?.recommendation ?? "",
    },
    {
      label: "AHR999 < 0.45",
      days: deepValueDays,
      recommendation: byId.deep_value?.recommendation ?? "",
    },
  ];
}

function normalizeHistoryDate(value: string): string {
  return value.replace(/\//g, "-");
}

function findNearestHistoryDate(
  points: Ahr999HistoryPoint[],
  tsMs: number,
  edge: "start" | "end",
) {
  if (edge === "start") {
    return normalizeHistoryDate(
      points.find((point) => point.ts_ms >= tsMs)?.date ?? points[0].date,
    );
  }
  return normalizeHistoryDate(
    [...points].reverse().find((point) => point.ts_ms <= tsMs)?.date ??
      points[points.length - 1].date,
  );
}

function findPointIndexByTs(points: Ahr999HistoryPoint[], tsMs: number) {
  const foundIndex = points.findIndex((point) => point.ts_ms === tsMs);
  if (foundIndex >= 0) {
    return foundIndex;
  }

  let nearestIndex = 0;
  let nearestDistance = Number.POSITIVE_INFINITY;
  points.forEach((point, index) => {
    const distance = Math.abs(point.ts_ms - tsMs);
    if (distance < nearestDistance) {
      nearestDistance = distance;
      nearestIndex = index;
    }
  });
  return nearestIndex;
}

function findNearestOverviewIndex(
  container: HTMLDivElement | null,
  clientX: number,
  plotLeft: number,
  plotWidth: number,
  totalPoints: number,
) {
  if (!container || totalPoints <= 1) {
    return 0;
  }
  const rect = container.getBoundingClientRect();
  const relativeX = ((clientX - rect.left) / rect.width) * 1240;
  const normalized = clamp((relativeX - plotLeft) / plotWidth, 0, 1);
  return clamp(Math.round(normalized * (totalPoints - 1)), 0, totalPoints - 1);
}

function clamp(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, value));
}

function buildLogDomain(values: number[]) {
  const positiveValues = values.filter((value) => value > 0 && Number.isFinite(value));
  const minValue = Math.min(...positiveValues);
  const maxValue = Math.max(...positiveValues);
  const minExponent = Math.floor(Math.log10(minValue));
  const maxExponent = Math.ceil(Math.log10(maxValue));
  return {
    min: 10 ** (minValue === maxValue ? minExponent - 1 : minExponent),
    max: 10 ** (minValue === maxValue ? maxExponent + 1 : maxExponent),
  };
}

function buildLogTicks(minValue: number, maxValue: number) {
  const ticks: number[] = [];
  const minExponent = Math.floor(Math.log10(minValue));
  const maxExponent = Math.ceil(Math.log10(maxValue));

  for (let exponent = minExponent; exponent <= maxExponent; exponent += 1) {
    for (const multiplier of [1, 2, 5]) {
      const tick = multiplier * 10 ** exponent;
      if (tick >= minValue && tick <= maxValue) {
        ticks.push(tick);
      }
    }
  }

  return ticks.length <= 7 ? ticks : ticks.filter((_, index) => index % Math.ceil(ticks.length / 7) === 0);
}

function yCoordLog(
  value: number,
  minValue: number,
  maxValue: number,
  top: number,
  bottom: number,
) {
  const minLog = Math.log10(minValue);
  const maxLog = Math.log10(maxValue);
  const valueLog = Math.log10(Math.max(value, minValue));
  const ratio = (valueLog - minLog) / Math.max(0.0001, maxLog - minLog);
  return bottom - ratio * (bottom - top);
}

function mergeNearbyTicks(values: number[]) {
  return [...values]
    .sort((a, b) => a - b)
    .filter((value, index, list) => index === 0 || Math.abs(value - list[index - 1]) > 8);
}

function buildTimeTicks(minTsMs: number, maxTsMs: number, count: number) {
  if (maxTsMs === minTsMs) {
    return [minTsMs];
  }
  return Array.from({ length: count }, (_, index) => minTsMs + ((maxTsMs - minTsMs) * index) / (count - 1));
}

function linePath(points: Array<{ x: number; y: number }>) {
  return points
    .map((point, index) => `${index === 0 ? "M" : "L"} ${point.x.toFixed(2)} ${point.y.toFixed(2)}`)
    .join(" ");
}

function areaPath(points: Array<{ x: number; y: number }>, baselineY: number) {
  if (points.length === 0) {
    return "";
  }
  const first = points[0];
  const last = points[points.length - 1];
  return `${linePath(points)} L ${last.x.toFixed(2)} ${baselineY.toFixed(2)} L ${first.x.toFixed(2)} ${baselineY.toFixed(2)} Z`;
}

function formatChartDate(tsMs: number): string {
  const date = new Date(tsMs);
  return `${date.getUTCFullYear()}/${String(date.getUTCMonth() + 1).padStart(2, "0")}/${String(
    date.getUTCDate(),
  ).padStart(2, "0")}`;
}

function formatAhrAxisValue(value: number): string {
  if (value >= 1000) {
    return `${(value / 1000).toFixed(2)}K`;
  }
  if (value >= 1) {
    return value.toFixed(2);
  }
  return value.toFixed(4);
}

function formatCompactUsdAxis(value: number): string {
  if (value >= 1_000_000) {
    return `$${(value / 1_000_000).toFixed(2)}M`;
  }
  if (value >= 1_000) {
    return `$${(value / 1_000).toFixed(2)}K`;
  }
  if (value >= 1) {
    return `$${value.toFixed(2)}`;
  }
  return `$${value.toFixed(4)}`;
}

function formatRegime(regime: MacroRegime, copy: Copy): string {
  return copy.macro.regimes[regime] ?? formatSnake(regime);
}

function formatSnake(value: string): string {
  return value.replace(/_/g, " ");
}

function formatValuationMetric(metric: ExternalMetricStatus): string {
  if (metric.value == null) {
    return formatSnake(metric.status);
  }
  return `${metric.value.toFixed(2)} · ${formatSnake(metric.status)}`;
}

function formatMetricMeta(metric: ExternalMetricStatus): string {
  return [metric.source, metric.date, metric.zone ? formatSnake(metric.zone) : null, metric.note]
    .filter(Boolean)
    .join(" · ");
}

function formatUsd(value: number): string {
  return `$${value.toLocaleString(undefined, {
    maximumFractionDigits: 2,
    minimumFractionDigits: 2,
  })}`;
}

function formatPct(value: number): string {
  return `${(value * 100).toFixed(2)}%`;
}

function formatDate(value: number): string {
  return new Date(value).toLocaleDateString();
}
