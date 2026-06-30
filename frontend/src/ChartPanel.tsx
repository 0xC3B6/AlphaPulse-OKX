import { useEffect, useMemo, useRef, useState } from "react";
import {
  ColorType,
  createChart,
  type CandlestickData,
  type IChartApi,
  type ISeriesApi,
  type UTCTimestamp,
} from "lightweight-charts";
import { fetchSymbolChart } from "./api";
import type { Copy } from "./i18n";
import type { ChartSnapshot, FvgZone, SymbolSnapshot } from "./types";

type ChartTimeframe = ChartSnapshot["timeframe"];

interface FvgBox {
  key: string;
  direction: FvgZone["direction"];
  filled: boolean;
  left: number;
  top: number;
  width: number;
  height: number;
  label: string;
}

export function ChartPanel({
  copy,
  symbol,
  themeMode,
}: {
  copy: Copy;
  symbol: SymbolSnapshot;
  themeMode: string;
}) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const seriesRef = useRef<ISeriesApi<"Candlestick"> | null>(null);
  const chartDataRef = useRef<ChartSnapshot | null>(null);
  const showFvgRef = useRef(true);
  const [chartData, setChartData] = useState<ChartSnapshot | null>(null);
  const [boxes, setBoxes] = useState<FvgBox[]>([]);
  const [timeframe, setTimeframe] = useState<ChartTimeframe>("m15");
  const [showFvg, setShowFvg] = useState(true);
  const [showFilled, setShowFilled] = useState(true);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    chartDataRef.current = chartData;
    updateFvgBoxes();
  }, [chartData]);

  useEffect(() => {
    showFvgRef.current = showFvg;
    updateFvgBoxes();
  }, [showFvg]);

  useEffect(() => {
    let active = true;
    setLoading(true);
    setError(null);

    fetchSymbolChart(symbol.inst_id, timeframe, showFilled)
      .then((data) => {
        if (!active) {
          return;
        }
        setChartData(data);
      })
      .catch((requestError) => {
        if (active) {
          setError(requestError instanceof Error ? requestError.message : String(requestError));
        }
      })
      .finally(() => {
        if (active) {
          setLoading(false);
        }
      });

    return () => {
      active = false;
    };
  }, [symbol.inst_id, timeframe, showFilled]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container || import.meta.env.MODE === "test") {
      return;
    }

    const styles = getComputedStyle(document.documentElement);
    const chart = createChart(container, {
      autoSize: true,
      height: 280,
      layout: {
        background: { type: ColorType.Solid, color: styles.getPropertyValue("--surface").trim() },
        textColor: styles.getPropertyValue("--text-muted").trim(),
      },
      grid: {
        horzLines: { color: styles.getPropertyValue("--border-subtle").trim() },
        vertLines: { color: styles.getPropertyValue("--border-subtle").trim() },
      },
      rightPriceScale: {
        borderColor: styles.getPropertyValue("--border").trim(),
      },
      timeScale: {
        borderColor: styles.getPropertyValue("--border").trim(),
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
    });

    chartRef.current = chart;
    seriesRef.current = series;
    chart.timeScale().subscribeVisibleTimeRangeChange(updateFvgBoxes);

    return () => {
      chart.timeScale().unsubscribeVisibleTimeRangeChange(updateFvgBoxes);
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

    const styles = getComputedStyle(document.documentElement);
    chart.applyOptions({
      layout: {
        background: { type: ColorType.Solid, color: styles.getPropertyValue("--surface").trim() },
        textColor: styles.getPropertyValue("--text-muted").trim(),
      },
      grid: {
        horzLines: { color: styles.getPropertyValue("--border-subtle").trim() },
        vertLines: { color: styles.getPropertyValue("--border-subtle").trim() },
      },
      rightPriceScale: {
        borderColor: styles.getPropertyValue("--border").trim(),
      },
      timeScale: {
        borderColor: styles.getPropertyValue("--border").trim(),
      },
    });
    updateFvgBoxes();
  }, [themeMode]);

  useEffect(() => {
    const chart = chartRef.current;
    const series = seriesRef.current;
    if (!chart || !series || !chartData) {
      return;
    }

    series.setData(
      chartData.candles.map((candle): CandlestickData => ({
        time: toChartTime(candle.ts_ms),
        open: candle.open,
        high: candle.high,
        low: candle.low,
        close: candle.close,
      })),
    );
    chart.timeScale().fitContent();
    requestAnimationFrame(updateFvgBoxes);
  }, [chartData]);

  const latestPrice = useMemo(
    () =>
      chartData && chartData.candles.length > 0
        ? chartData.candles[chartData.candles.length - 1].close
        : symbol.price,
    [chartData, symbol.price],
  );

  return (
    <section className="chart-panel" aria-label={copy.chart.title}>
      <div className="chart-header">
        <div>
          <h3>{copy.chart.title}</h3>
          <p>
            {timeframe} · {formatChartPrice(latestPrice)}
          </p>
        </div>
        <div className="chart-timeframes" role="group" aria-label={copy.chart.timeframe}>
          {(["m5", "m15", "h1"] as const).map((value) => (
            <button
              className={timeframe === value ? "active" : ""}
              key={value}
              onClick={() => setTimeframe(value)}
              type="button"
            >
              {value}
            </button>
          ))}
        </div>
      </div>
      <div className="chart-controls">
        <label className="chart-toggle">
          <input
            checked={showFvg}
            onChange={(event) => setShowFvg(event.target.checked)}
            type="checkbox"
          />
          <span>{copy.chart.showFvg}</span>
        </label>
        <label className="chart-toggle">
          <input
            checked={showFilled}
            onChange={(event) => setShowFilled(event.target.checked)}
            type="checkbox"
          />
          <span>{copy.chart.showFilledFvg}</span>
        </label>
      </div>
      <div className="chart-stage">
        <div className="chart-canvas" ref={containerRef} />
        <div className="fvg-overlay" aria-hidden="true">
          {boxes.map((box) => (
            <div
              className={[
                "fvg-box",
                box.direction === "long" ? "fvg-box-long" : "fvg-box-short",
                box.filled ? "fvg-box-filled" : "",
              ]
                .filter(Boolean)
                .join(" ")}
              key={box.key}
              style={{
                height: `${box.height}px`,
                left: `${box.left}px`,
                top: `${box.top}px`,
                width: `${box.width}px`,
              }}
            >
              <span>{box.label}</span>
            </div>
          ))}
        </div>
        {loading ? <div className="chart-message">{copy.chart.loading}</div> : null}
        {error ? <div className="chart-message chart-error">{error}</div> : null}
      </div>
    </section>
  );

  function updateFvgBoxes() {
    const chart = chartRef.current;
    const series = seriesRef.current;
    const data = chartDataRef.current;
    if (!chart || !series || !data || !showFvgRef.current) {
      setBoxes([]);
      return;
    }

    const nextBoxes = data.fvgs
      .map((zone, index) => {
        const x1 = chart.timeScale().timeToCoordinate(toChartTime(zone.start_ts_ms));
        const x2 = chart.timeScale().timeToCoordinate(toChartTime(zone.end_ts_ms));
        const y1 = series.priceToCoordinate(zone.upper);
        const y2 = series.priceToCoordinate(zone.lower);
        if (x1 === null || x2 === null || y1 === null || y2 === null) {
          return null;
        }
        const left = Math.min(x1, x2);
        const top = Math.min(y1, y2);
        return {
          key: `${zone.timeframe}-${zone.direction}-${zone.start_ts_ms}-${index}`,
          direction: zone.direction,
          filled: zone.filled,
          left,
          top,
          width: Math.max(Math.abs(x2 - x1), 3),
          height: Math.max(Math.abs(y2 - y1), 3),
          label: `${copy.directions[zone.direction]} ${formatChartPrice(zone.lower)}-${formatChartPrice(zone.upper)}`,
        };
      })
      .filter((box): box is FvgBox => box !== null);

    setBoxes(nextBoxes);
  }
}

function toChartTime(tsMs: number): UTCTimestamp {
  return Math.floor(tsMs / 1000) as UTCTimestamp;
}

function formatChartPrice(value: number): string {
  if (value >= 100) {
    return value.toFixed(2);
  }
  if (value >= 1) {
    return value.toFixed(4);
  }
  return value.toFixed(6);
}
