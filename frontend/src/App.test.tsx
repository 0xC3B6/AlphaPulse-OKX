import "@testing-library/jest-dom/vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import type {
  BtcMacroSnapshot,
  ChartSnapshot,
  DashboardSnapshot,
  PaperAccountSnapshot,
} from "./types";

const paper: PaperAccountSnapshot = {
  mode: "paper",
  initial_balance: 10000,
  realized_pnl: 0,
  unrealized_pnl: 0,
  equity: 10000,
  used_margin: 0,
  available_balance: 10000,
  positions: [],
  trades: [],
};

const snapshot: DashboardSnapshot = {
  last_scan_at_ms: 1782400000000,
  websocket_connected: true,
  paper,
  symbols: [
    {
      inst_id: "LAB-USDT-SWAP",
      price: 17.2,
      change_5m_pct: -0.03,
      change_15m_pct: -0.07,
      change_1h_pct: -0.11,
      trend_score: {
        value: 84,
        direction: "short",
        reasons: ["15m move -7.0% aligns with 1h move -11.0%", "volume 3.1x"],
      },
      range_score: {
        value: 42,
        direction: "neutral",
        reasons: ["clear recent range"],
      },
      pool_tags: ["dynamic", "new_listing"],
      trigger_reason: "trend short 84: volume 3.1x",
      funding_rate: -0.003,
      fvgs: [
        {
          timeframe: "m15",
          direction: "short",
          start_ts_ms: 1782399100000,
          end_ts_ms: 1782400000000,
          lower: 16.2,
          upper: 16.8,
          gap_pct: 0.03,
          distance_pct: 0.02,
          filled: false,
        },
      ],
      levels: [
        {
          kind: "resistance",
          lower: 17.4,
          upper: 17.6,
          touches: 3,
          distance_pct: 0.015,
        },
      ],
      updated_at_ms: 1782400000000,
    },
    {
      inst_id: "DOGE-USDT-SWAP",
      price: 0.18,
      change_5m_pct: 0.002,
      change_15m_pct: 0.004,
      change_1h_pct: 0.01,
      trend_score: { value: 20, direction: "neutral", reasons: [] },
      range_score: { value: 82, direction: "long", reasons: ["near support"] },
      pool_tags: ["fixed"],
      trigger_reason: "range long 82: near support",
      funding_rate: null,
      fvgs: [],
      levels: [],
      updated_at_ms: 1782400000000,
    },
  ],
};

const chart: ChartSnapshot = {
  inst_id: "LAB-USDT-SWAP",
  timeframe: "m15",
  updated_at_ms: 1782400000000,
  candles: [
    {
      ts_ms: 1782399100000,
      open: 17.1,
      high: 17.4,
      low: 16.9,
      close: 17.2,
      volume: 100,
    },
  ],
  fvgs: [
    {
      timeframe: "m15",
      direction: "short",
      start_ts_ms: 1782399100000,
      end_ts_ms: 1782400000000,
      lower: 16.2,
      upper: 16.8,
      gap_pct: 0.03,
      distance_pct: 0.02,
      filled: false,
    },
  ],
};

function analogWindow(startMs: number, days: number, drift: number) {
  const candles = Array.from({ length: days + 1 }, (_, index) => {
    const base = 100 + index * drift;
    return {
      ts_ms: startMs + index * 86400000,
      offset_days: index,
      open: base,
      high: base + 3,
      low: base - 3,
      close: base + 1,
      index_open: base,
      index_high: base + 3,
      index_low: base - 3,
      index_close: base + 1,
    };
  });
  return {
    start_ts_ms: candles[0].ts_ms,
    end_ts_ms: candles[candles.length - 1].ts_ms,
    final_return_pct: candles[candles.length - 1].index_close / 100 - 1,
    max_drawdown_pct: Math.min(...candles.map((candle) => candle.index_low / 100 - 1)),
    max_runup_pct: Math.max(...candles.map((candle) => candle.index_high / 100 - 1)),
    candles,
    path: candles.map((candle) => ({
      offset_days: candle.offset_days,
      return_pct: candle.index_close / 100 - 1,
    })),
  };
}

const macro = {
  asset: "BTC",
  updated_at_ms: 1782400000000,
  price: 60000,
  regime: "bear_market_rally",
  confidence: 80,
  summary: "bear market rally; cycle day 801",
  cycle: {
    last_halving_ms: 1713571200000,
    next_halving_estimate_ms: 1839801600000,
    days_since_halving: 801,
    estimated_cycle_progress_pct: 0.55,
    cycle_year: 3,
    cycle_quarter: 9,
    phase: "late_cycle_distribution_window",
  },
  trend: {
    window_ath: 100000,
    window_ath_ts_ms: 1780000000000,
    drawdown_from_window_ath_pct: -0.4,
    ma_200w: 55000,
    price_vs_200w_pct: 0.09,
    weekly_ma200_status: "above_200w_ma",
  },
  momentum: {
    change_30d_pct: 0.05,
    change_90d_pct: 0.12,
    change_26w_pct: -0.2,
    volatility_90d_pct: 0.6,
  },
  events: [
    {
      id: "us_midterm_2026",
      title: "2026 US midterm elections",
      event_type: "us_midterm",
      date_ms: 1793664000000,
      days_to_event: 127,
      phase: "pre_election_background",
      impact_tags: ["policy_uncertainty"],
    },
  ],
  valuation_metrics: [
    {
      id: "ahr999",
      name: "AHR999",
      status: "data_source_pending",
      note: "requires external BTC valuation data source",
    },
  ],
  analogs: [
    {
      label: "bear_market_slow_rebound",
      score: 65,
      rationale: ["cycle_day=801", "drawdown=-40.0%"],
      components: [
        {
          label: "drawdown",
          points: 20,
          max_points: 25,
          detail: "-40.0% from window ATH",
        },
      ],
    },
  ],
  ahr999_history: {
    source: "self_calculated_okx",
    points: Array.from({ length: 25 }, (_, index) => ({
      ts_ms: 1780329600000 + index * 86400000,
      date: `2026/06/${String(index + 1).padStart(2, "0")}`,
      value: 0.35 + index * 0.01,
      btc_price: 58000 + index * 100,
      gma200: 74000 + index * 50,
      model_price: 160000 + index * 20,
      zone: index < 10 ? "deep_value" : "accumulation",
    })),
    bands: [
      {
        id: "deep_value",
        label: "AHR999 < 0.45",
        lower: null,
        upper: 0.45,
        days: 10,
        recommendation: "deep value zone; spot accumulation or very low leverage only",
      },
      {
        id: "accumulation",
        label: "0.45 <= AHR999 < 1.2",
        lower: 0.45,
        upper: 1.2,
        days: 15,
        recommendation: "accumulation zone; staged entries can be scored higher",
      },
      {
        id: "neutral",
        label: "1.2 <= AHR999 < 5",
        lower: 1.2,
        upper: 5,
        days: 0,
        recommendation: "neutral trend zone",
      },
      {
        id: "overheated",
        label: "AHR999 >= 5",
        lower: 5,
        upper: null,
        days: 0,
        recommendation: "overheated zone",
      },
    ],
  },
  analog_comparisons: [
    {
      timeframe_days: 30,
      current: analogWindow(1779811200000, 30, -0.15),
      matches: [],
    },
    {
      timeframe_days: 90,
      current: analogWindow(1774627200000, 90, -0.13),
      matches: [
        {
          id: "90-1609459200000",
          label: "after 90D 2021/01/01",
          score: 82,
          start_ts_ms: 1609459200000,
          end_ts_ms: 1617235200000,
          final_return_pct: -0.1,
          max_drawdown_pct: -0.2,
          max_runup_pct: 0.04,
          components: [
            {
              label: "path_shape",
              points: 39,
              max_points: 45,
              detail: "distance=3.00%",
            },
          ],
          lookback: analogWindow(1601683200000, 90, -0.12),
          forward: analogWindow(1609545600000, 90, 0.22),
          path: analogWindow(1609545600000, 90, 0.22).path,
        },
      ],
    },
  ],
  trading_bias: ["alts_rebounds_should_be_treated_as_lower_confidence_longs"],
} as BtcMacroSnapshot;

afterEach(() => {
  vi.unstubAllGlobals();
  localStorage.clear();
  document.documentElement.removeAttribute("data-theme");
});

function mockSnapshot(data: DashboardSnapshot = snapshot) {
  vi.stubGlobal(
    "fetch",
    vi.fn(async (input: RequestInfo | URL) => ({
      ok: true,
      json: async () => {
        if (String(input).includes("/chart")) {
          return chart;
        }
        if (String(input).includes("/macro/btc")) {
          return macro;
        }
        return data;
      },
    })),
  );
}

describe("App", () => {
  it("renders the radar title and connection status", async () => {
    mockSnapshot({ symbols: [], last_scan_at_ms: null, websocket_connected: false, paper });
    render(<App />);
    expect(screen.getByText("AlphaPulse OKX")).toBeInTheDocument();
    expect(screen.getByText("后端")).toBeInTheDocument();
    await screen.findByText("暂无合约数据");
  });

  it("loads symbols and filters trend opportunities", async () => {
    mockSnapshot();
    render(<App />);

    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);
    expect(screen.getByText("动态 / 新币")).toBeInTheDocument();
    expect(screen.getByText("DOGE-USDT-SWAP")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "趋势" }));

    expect(screen.getAllByText("LAB-USDT-SWAP").length).toBeGreaterThan(0);
    await waitFor(() => {
      expect(screen.queryByText("DOGE-USDT-SWAP")).not.toBeInTheDocument();
    });
  });

  it("defaults to following the system theme", async () => {
    mockSnapshot({ symbols: [], last_scan_at_ms: null, websocket_connected: false, paper });

    render(<App />);

    await screen.findByText("暂无合约数据");
    expect(screen.getByRole("button", { name: "跟随系统" })).toHaveClass("active");
    expect(document.documentElement).toHaveAttribute("data-theme", "system");
    expect(localStorage.getItem("alphapulse-theme")).toBeNull();
  });

  it("stores and applies explicit theme choices", async () => {
    mockSnapshot({ symbols: [], last_scan_at_ms: null, websocket_connected: false, paper });

    render(<App />);
    await screen.findByText("暂无合约数据");

    fireEvent.click(screen.getByRole("button", { name: "深色主题" }));

    expect(document.documentElement).toHaveAttribute("data-theme", "dark");
    expect(localStorage.getItem("alphapulse-theme")).toBe("dark");
    expect(screen.getByRole("button", { name: "深色主题" })).toHaveClass("active");

    fireEvent.click(screen.getByRole("button", { name: "浅色主题" }));

    expect(document.documentElement).toHaveAttribute("data-theme", "light");
    expect(localStorage.getItem("alphapulse-theme")).toBe("light");
  });

  it("defaults to Chinese language", async () => {
    mockSnapshot({ symbols: [], last_scan_at_ms: null, websocket_connected: false, paper });

    render(<App />);

    await screen.findByText("暂无合约数据");
    expect(screen.getByRole("button", { name: "中文" })).toHaveClass("active");
    expect(screen.getByText("后端")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "趋势" })).toBeInTheDocument();
    expect(localStorage.getItem("alphapulse-language")).toBeNull();
  });

  it("stores and applies English language choice", async () => {
    mockSnapshot();

    render(<App />);
    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: "English" }));

    expect(localStorage.getItem("alphapulse-language")).toBe("en");
    expect(screen.getByRole("button", { name: "English" })).toHaveClass("active");
    expect(screen.getByText("Backend")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Trend" })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "Signal" })).toBeInTheDocument();
  });

  it("renders the macro cycle view", async () => {
    mockSnapshot();

    render(<App />);
    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: "大周期" }));

    expect(await screen.findByText("BTC 大周期")).toBeInTheDocument();
    expect(screen.getByText("熊市反弹")).toBeInTheDocument();
    expect(screen.getByText("2026 US midterm elections")).toBeInTheDocument();
    expect(screen.getAllByText("AHR999").length).toBeGreaterThan(0);
  });

  it("renders AHR999 history guidance and paginated rows", async () => {
    mockSnapshot();

    render(<App />);
    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: "大周期" }));

    expect(await screen.findByText("AHR999 历史")).toBeInTheDocument();
    expect(screen.getAllByText("Ahr999 Index").length).toBeGreaterThan(0);
    expect(screen.getAllByText("200 Day Cost").length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "全部" })).toHaveClass("active");
    expect(screen.getByLabelText("开始")).toHaveValue("2026-06-01");
    expect(screen.getByLabelText("结束")).toHaveValue("2026-06-25");
    expect(screen.getByRole("cell", { name: "2026/06/25" })).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("结束"), { target: { value: "2026-06-10" } });
    expect(screen.queryByRole("cell", { name: "2026/06/25" })).not.toBeInTheDocument();
    expect(screen.getByRole("cell", { name: "2026/06/10" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "全部" }));
    fireEvent.change(screen.getByLabelText("每页"), { target: { value: "50" } });
    expect(screen.getByRole("cell", { name: "2026/06/01" })).toBeInTheDocument();
  });

  it("renders analog forward K-line windows with score components", async () => {
    mockSnapshot();

    render(<App />);
    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: "大周期" }));

    expect(await screen.findByText("历史 K 线对比")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "90D" })).toHaveClass("active");
    expect(screen.getByText("当前节点前 90D")).toBeInTheDocument();
    expect(screen.getAllByText("历史节点后 90D 2021/01/01 · 82/100").length).toBeGreaterThan(0);
    expect(screen.getByLabelText("当前节点前 90D K线")).toBeInTheDocument();
    expect(screen.getByLabelText("历史节点后 90D 2021/01/01 K线")).toBeInTheDocument();
    expect(screen.getByText("price $100.00 -> $89.30 · range $85.30-$103.00")).toBeInTheDocument();
    expect(screen.getAllByText("path shape 39/45").length).toBeGreaterThan(0);
    fireEvent.click(screen.getByRole("button", { name: "30D" }));
    expect(screen.getAllByText("当前节点前 30D").length).toBeGreaterThan(0);
  });

  it("shows an animated macro loading state", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL) => {
        if (String(input).includes("/macro/btc")) {
          return new Promise(() => {});
        }
        if (String(input).includes("/chart")) {
          return {
            ok: true,
            json: async () => chart,
          };
        }
        return {
          ok: true,
          json: async () => snapshot,
        };
      }),
    );

    render(<App />);
    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: "大周期" }));

    expect(await screen.findByTestId("macro-loading")).toHaveClass("macro-loading");
    expect(screen.getByText("加载大周期数据中")).toBeInTheDocument();
  });

  it("submits a paper long order for the selected symbol", async () => {
    const paperWithPosition: PaperAccountSnapshot = {
      ...paper,
      available_balance: 9900,
      equity: 10000,
      used_margin: 100,
      positions: [
        {
          inst_id: "LAB-USDT-SWAP",
          side: "long",
          qty: 58.1395348837,
          entry_price: 17.2,
          mark_price: 17.2,
          margin: 100,
          leverage: 10,
          notional: 1000,
          unrealized_pnl: 0,
          pnl_pct: 0,
          opened_at_ms: 1782400000000,
        },
      ],
      trades: [
        {
          id: 1,
          inst_id: "LAB-USDT-SWAP",
          side: "long",
          action: "open",
          price: 17.2,
          qty: 58.1395348837,
          margin: 100,
          notional: 1000,
          realized_pnl: 0,
          ts_ms: 1782400000000,
        },
      ],
    };
    const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      if (String(input).endsWith("/api/paper/orders")) {
        expect(init?.method).toBe("POST");
        expect(JSON.parse(String(init?.body))).toEqual({
          inst_id: "LAB-USDT-SWAP",
          side: "long",
          margin: 100,
          leverage: 10,
        });
        return {
          ok: true,
          json: async () => paperWithPosition,
        };
      }
      if (String(input).includes("/chart")) {
        return {
          ok: true,
          json: async () => chart,
        };
      }
      if (String(input).includes("/macro/btc")) {
        return {
          ok: true,
          json: async () => macro,
        };
      }
      return {
        ok: true,
        json: async () => snapshot,
      };
    });
    vi.stubGlobal("fetch", fetchMock);

    render(<App />);
    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: "模拟买入 / 开多" }));

    expect(await screen.findByRole("button", { name: "模拟平仓" })).toBeInTheDocument();
    await waitFor(() => {
      expect(
        fetchMock.mock.calls.some((call) => String(call[0]).endsWith("/api/paper/orders")),
      ).toBe(true);
    });
  });
});
