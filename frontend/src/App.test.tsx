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

const activePaper: PaperAccountSnapshot = {
  ...paper,
  realized_pnl: 207.58,
  unrealized_pnl: 42.86,
  equity: 10250.44,
  used_margin: 459.44,
  available_balance: 9791,
  total_fees: 4.2,
  total_trades: 3,
  closed_position_count: 2,
  winning_closed_position_count: 1,
  losing_closed_position_count: 1,
  win_rate: 0.5,
  average_holding_duration_ms: 1800000,
  average_closed_position_pnl: 89.36,
  average_winning_pnl: 236.94,
  average_losing_pnl: -58.22,
  profit_factor: 4.07,
  largest_winning_pnl: 236.94,
  largest_losing_pnl: -58.22,
  strategy_stats: [
    {
      strategy_name: "Scalping Optimization Design",
      strategy_version: "v0.1.3",
      total_trades: 3,
      closed_position_count: 2,
      winning_closed_position_count: 1,
      losing_closed_position_count: 1,
      win_rate: 0.5,
      realized_pnl: 178.72,
      total_fees: 4.2,
      first_trade_ts_ms: 1782392800000,
      last_trade_ts_ms: 1782400000000,
      running_duration_ms: 7200000,
      average_holding_duration_ms: 1800000,
      average_position_pnl: 89.36,
      profit_factor: 4.07,
      largest_winning_pnl: 236.94,
      largest_losing_pnl: -58.22,
    },
  ],
  positions: [
    {
      inst_id: "LAB-USDT-SWAP",
      side: "long",
      qty: 58.1395348837,
      entry_price: 17.2,
      mark_price: 17.9,
      margin: 100,
      leverage: 10,
      notional: 1000,
      unrealized_pnl: 40.7,
      pnl_pct: 0.407,
      opened_at_ms: 1782400000000,
    },
    {
      inst_id: "DOGE-USDT-SWAP",
      side: "short",
      qty: 1000,
      entry_price: 0.18,
      mark_price: 0.17784,
      margin: 180,
      leverage: 1,
      notional: 180,
      unrealized_pnl: 2.16,
      pnl_pct: 0.012,
      opened_at_ms: 1782400000000,
    },
  ],
  trades: [
    {
      id: 3,
      inst_id: "BREV-USDT-SWAP",
      side: "short",
      action: "close",
      price: 0.083787,
      qty: 2000,
      margin: 100,
      notional: 167.57,
      source: "scalping_optimization_design",
      strategy_name: "Scalping Optimization Design",
      strategy_version: "v0.1.3",
      reason: "scalping v0.1.3 take profit 132.68%",
      realized_pnl: 236.94,
      ts_ms: 1782400000000,
    },
    {
      id: 2,
      inst_id: "NES-USDT-SWAP",
      side: "short",
      action: "close",
      price: 0.218644,
      qty: 1000,
      margin: 100,
      notional: 218.64,
      source: "scalping_optimization_design",
      strategy_name: "Scalping Optimization Design",
      strategy_version: "v0.1.3",
      reason: "scalping v0.1.3 stop loss -30.12%",
      realized_pnl: -58.22,
      ts_ms: 1782396400000,
    },
    {
      id: 1,
      inst_id: "LAB-USDT-SWAP",
      side: "long",
      action: "open",
      price: 17.2,
      qty: 58.1395348837,
      margin: 100,
      notional: 1000,
      source: "scalping_optimization_design",
      strategy_name: "Scalping Optimization Design",
      strategy_version: "v0.1.3",
      reason: "scalping v0.1.3 trend long 82",
      realized_pnl: 0,
      ts_ms: 1782392800000,
    },
  ],
  position_history: [
    {
      id: 2,
      inst_id: "BREV-USDT-SWAP",
      side: "short",
      qty: 2000,
      entry_price: 0.089722,
      exit_price: 0.083787,
      margin: 100,
      leverage: 20,
      notional: 167.57,
      fees: 2.1,
      realized_pnl: 236.94,
      pnl_pct: 2.3694,
      opened_at_ms: 1782392800000,
      closed_at_ms: 1782400000000,
      duration_ms: 7200000,
      source: "scalping_optimization_design",
      strategy_name: "Scalping Optimization Design",
      strategy_version: "v0.1.3",
      reason: "scalping v0.1.3 multiday extension reversal short 90",
      close_source: "scalping_optimization_design",
      close_reason: "scalping v0.1.3 take profit 132.68%",
    },
    {
      id: 1,
      inst_id: "NES-USDT-SWAP",
      side: "short",
      qty: 1000,
      entry_price: 0.215357,
      exit_price: 0.218644,
      margin: 100,
      leverage: 20,
      notional: 218.64,
      fees: 2.1,
      realized_pnl: -58.22,
      pnl_pct: -0.5822,
      opened_at_ms: 1782392800000,
      closed_at_ms: 1782396400000,
      duration_ms: 3600000,
      source: "scalping_optimization_design",
      strategy_name: "Scalping Optimization Design",
      strategy_version: "v0.1.3",
      reason: "scalping v0.1.3 multiday extension reversal short 88",
      close_source: "scalping_optimization_design",
      close_reason: "scalping v0.1.3 stop loss -30.12%",
    },
  ],
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
  market_permission: {
    state: "reduced_risk",
    radar_policy: {
      altcoin_notify: false,
      max_priority: "medium",
      leverage_hint: "reduced",
    },
    allowed_behaviors: ["intraday_or_short_duration_only", "avoid_altcoin_overnight_exposure"],
    reasons: ["repair_rally_requires_short_duration_trades"],
  },
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
    ma_50d: 62000,
    ma_200d: 70000,
    price_vs_50d_pct: -0.032,
    price_vs_200d_pct: -0.143,
    ma_50d_slope_30d_pct: 0.025,
    ma_200d_slope_30d_pct: -0.015,
    structure: "repair_rally",
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
      cohort_stats: [],
      matches: [],
    },
    {
      timeframe_days: 90,
      current: analogWindow(1774627200000, 90, -0.13),
      cohort_stats: [
        {
          requested_size: 20,
          sample_size: 20,
          up_probability: 0.45,
          median_forward_return_pct: -0.0177,
          lower_quartile_forward_return_pct: -0.1565,
          median_forward_drawdown_pct: -0.1021,
          median_forward_runup_pct: 0.3055,
          score_floor: 67,
        },
      ],
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

  it("uses four top-level task rail pages and keeps radar filters inside Monitor", async () => {
    mockSnapshot({ ...snapshot, paper: activePaper });

    render(<App />);
    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);

    const taskRail = screen.getByRole("navigation", { name: "主导航" });
    expect(taskRail).toHaveTextContent("监控");
    expect(taskRail).toHaveTextContent("交易");
    expect(taskRail).toHaveTextContent("复盘");
    expect(taskRail).toHaveTextContent("宏观");
    expect(screen.getByRole("button", { name: "监控" })).toHaveAttribute(
      "aria-current",
      "page",
    );
    expect(screen.getByRole("button", { name: "趋势" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "交易" }));

    expect(screen.getByRole("heading", { name: "交易" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "趋势" })).not.toBeInTheDocument();
    expect(screen.getByText("当前持仓")).toBeInTheDocument();
  });

  it("shows all current positions on the Trade page and preloads the selected Monitor symbol", async () => {
    mockSnapshot({ ...snapshot, paper: activePaper });

    render(<App />);
    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("row", { name: /DOGE-USDT-SWAP/ }));
    fireEvent.click(screen.getByRole("button", { name: "去交易" }));

    expect(screen.getByRole("heading", { name: "交易" })).toBeInTheDocument();
    expect(screen.getByTestId("trade-page")).toHaveTextContent("LAB-USDT-SWAP");
    expect(screen.getByTestId("trade-page")).toHaveTextContent("DOGE-USDT-SWAP");
    expect(screen.getByLabelText("交易合约")).toHaveValue("DOGE-USDT-SWAP");
    expect(screen.getByText("全部当前持仓")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "模拟卖出 / 开空" })).toBeInTheDocument();
  });

  it("shows Review performance and trade records without Monitor filters", async () => {
    mockSnapshot({ ...snapshot, paper: activePaper });

    render(<App />);
    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: "复盘" }));

    expect(screen.getByRole("heading", { name: "复盘" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "趋势" })).not.toBeInTheDocument();
    expect(screen.getByTestId("review-page")).toHaveTextContent("已实现盈亏");
    expect(screen.getByTestId("review-page")).toHaveTextContent("+207.58 USDT");
    expect(screen.getByTestId("review-page")).toHaveTextContent("初始资金");
    expect(screen.getByTestId("review-page")).toHaveTextContent("10,000.00 USDT");
    expect(screen.getByTestId("review-page")).toHaveTextContent("权益");
    expect(screen.getByTestId("review-page")).toHaveTextContent("10,250.44 USDT");
    expect(screen.getByTestId("review-page")).toHaveTextContent("可用");
    expect(screen.getByTestId("review-page")).toHaveTextContent("9,791.00 USDT");
    expect(screen.getByTestId("review-page")).toHaveTextContent("占用保证金");
    expect(screen.getByTestId("review-page")).toHaveTextContent("459.44 USDT");
    expect(screen.getByTestId("review-page")).toHaveTextContent("胜率");
    expect(screen.getByTestId("review-page")).toHaveTextContent("50.00%");
    expect(screen.getByTestId("review-page")).toHaveTextContent("亏损单");
    expect(screen.getByTestId("review-page")).toHaveTextContent("平均持仓");
    expect(screen.getByTestId("review-page")).toHaveTextContent("30m");
    expect(screen.getByTestId("review-page")).toHaveTextContent("平均单笔盈亏");
    expect(screen.getByTestId("review-page")).toHaveTextContent("+89.36 USDT");
    expect(screen.getByTestId("review-page")).not.toHaveTextContent("暂不可用");
    expect(screen.getByTestId("paper-strategy-stats")).toHaveTextContent("Scalping Optimization Design");
    expect(screen.getByTestId("review-page")).toHaveTextContent("历史持仓");

    fireEvent.click(screen.getByRole("button", { name: "策略版本对比" }));
    fireEvent.click(screen.getByRole("row", { name: /Scalping Optimization Design v0\.1\.3/ }));
    expect(screen.getByTestId("paper-strategy-curve")).toHaveTextContent("v0.1.3");

    fireEvent.click(screen.getByRole("button", { name: "历史持仓" }));
    expect(screen.getByLabelText("历史持仓币种")).toBeInTheDocument();
    expect(screen.getByLabelText("历史持仓开始日期")).toBeInTheDocument();
    expect(screen.getByLabelText("历史持仓结束日期")).toBeInTheDocument();
    expect(screen.getByLabelText("历史持仓版本")).toBeInTheDocument();
    expect(screen.getByTestId("review-page")).toHaveTextContent("scalping v0.1.3 take profit 132.68%");
    expect(screen.getAllByText(/BREV-USDT-SWAP/).length).toBeGreaterThan(0);
    fireEvent.change(screen.getByLabelText("历史持仓币种"), { target: { value: "nes" } });
    expect(screen.getByTestId("review-page")).toHaveTextContent("NES-USDT-SWAP");
    expect(screen.getByTestId("review-page")).not.toHaveTextContent("BREV-USDT-SWAP");
  });

  it("opens a TradingView chart modal from the radar only", async () => {
    mockSnapshot();
    render(<App />);

    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);

    fireEvent.click(screen.getAllByRole("button", { name: "打开 LAB-USDT-SWAP TradingView 图表" })[0]);

    const dialog = screen.getByRole("dialog", { name: "LAB-USDT-SWAP TradingView" });
    expect(dialog).toBeInTheDocument();
    const frame = screen.getByTitle("LAB-USDT-SWAP TradingView");
    const source = decodeURIComponent(frame.getAttribute("src") ?? "");
    expect(source).toContain("https://s.tradingview.com/widgetembed/");
    expect(source).toContain("symbol=OKX:LABUSDT.P");
    expect(source).toContain("interval=15");

    fireEvent.click(screen.getByRole("button", { name: "关闭 TradingView" }));
    expect(screen.queryByRole("dialog", { name: "LAB-USDT-SWAP TradingView" })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "宏观" }));
    expect(screen.queryByRole("button", { name: /TradingView/ })).not.toBeInTheDocument();
  });

  it("defaults to following the system theme", async () => {
    mockSnapshot({ symbols: [], last_scan_at_ms: null, websocket_connected: false, paper });

    render(<App />);

    await screen.findByText("暂无合约数据");
    expect(screen.getByRole("button", { name: "主题模式: System" })).toHaveTextContent("System");
    expect(screen.getByRole("menuitemradio", { name: "System" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    expect(document.documentElement).toHaveAttribute("data-theme", "system");
    expect(localStorage.getItem("alphapulse-theme")).toBeNull();
  });

  it("stores and applies explicit theme choices", async () => {
    mockSnapshot({ symbols: [], last_scan_at_ms: null, websocket_connected: false, paper });

    render(<App />);
    await screen.findByText("暂无合约数据");

    fireEvent.click(screen.getByRole("menuitemradio", { name: "Dark" }));

    expect(document.documentElement).toHaveAttribute("data-theme", "dark");
    expect(localStorage.getItem("alphapulse-theme")).toBe("dark");
    expect(screen.getByRole("button", { name: "主题模式: Dark" })).toHaveTextContent("Dark");
    expect(screen.getByRole("menuitemradio", { name: "Dark" })).toHaveAttribute(
      "aria-checked",
      "true",
    );

    fireEvent.click(screen.getByRole("menuitemradio", { name: "Light" }));

    expect(document.documentElement).toHaveAttribute("data-theme", "light");
    expect(localStorage.getItem("alphapulse-theme")).toBe("light");
  });

  it("defaults to Chinese language", async () => {
    mockSnapshot({ symbols: [], last_scan_at_ms: null, websocket_connected: false, paper });

    render(<App />);

    await screen.findByText("暂无合约数据");
    expect(screen.getByRole("button", { name: "语言: ZH" })).toHaveTextContent("ZH");
    expect(screen.getByRole("menuitemradio", { name: "ZH" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    expect(screen.getByText("后端")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "趋势" })).toBeInTheDocument();
    expect(localStorage.getItem("alphapulse-language")).toBeNull();
  });

  it("stores and applies English language choice", async () => {
    mockSnapshot();

    render(<App />);
    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("menuitemradio", { name: "EN" }));

    expect(localStorage.getItem("alphapulse-language")).toBe("en");
    expect(screen.getByRole("button", { name: "language: EN" })).toHaveTextContent("EN");
    expect(screen.getByRole("menuitemradio", { name: "EN" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    expect(screen.getByText("Backend")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Trend" })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "Signal" })).toBeInTheDocument();
  });

  it("renders the macro cycle view", async () => {
    mockSnapshot();

    render(<App />);
    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: "宏观" }));

    expect(await screen.findByText("BTC 大周期")).toBeInTheDocument();
    expect(screen.getByTestId("macro-regime-card")).toHaveTextContent("熊市反弹");
    expect(screen.getByTestId("macro-cycle-progress")).toHaveTextContent("55.00%");
    expect(screen.getByText("熊市反弹")).toBeInTheDocument();
    expect(screen.getByText("2026 US midterm elections")).toBeInTheDocument();
    expect(screen.getAllByText("AHR999").length).toBeGreaterThan(0);
  });

  it("renders macro permission, trend structure, and analog cohort statistics", async () => {
    mockSnapshot();

    render(<App />);
    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: "宏观" }));

    expect((await screen.findAllByText("交易许可")).length).toBeGreaterThan(0);
    expect(screen.getByTestId("macro-permission-card")).toHaveTextContent("降风险");
    expect(screen.getByTestId("macro-permission-card")).toHaveTextContent("山寨通知 关闭");
    expect(screen.getByTestId("macro-trend-structure-card")).toHaveTextContent("修复反弹");
    expect(screen.getByTestId("macro-trend-structure-card")).toHaveTextContent("日线 MA50");
    fireEvent.click(screen.getByRole("button", { name: "90D" }));
    expect(await screen.findByText("相似样本统计")).toBeInTheDocument();
    expect(screen.getByTestId("analog-cohort-20")).toHaveTextContent("Top 20");
    expect(screen.getByTestId("analog-cohort-20")).toHaveTextContent("上涨概率 45.00%");
    expect(screen.getByTestId("analog-cohort-20")).toHaveTextContent("中位回撤 -10.21%");
  });

  it("prefetches macro data before the macro view is opened", async () => {
    let macroRequests = 0;
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL) => {
        if (String(input).includes("/macro/btc")) {
          macroRequests += 1;
          return {
            ok: true,
            json: async () => macro,
          };
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
    await waitFor(() => expect(macroRequests).toBe(1));

    fireEvent.click(screen.getByRole("button", { name: "宏观" }));

    expect(screen.queryByTestId("macro-loading")).not.toBeInTheDocument();
    expect(await screen.findByText("BTC 大周期")).toBeInTheDocument();
    expect(macroRequests).toBe(1);
  });

  it("shows a compact macro summary on the radar console", async () => {
    mockSnapshot();

    render(<App />);

    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);
    const summary = await screen.findByTestId("macro-summary-strip");

    expect(summary).toHaveTextContent("BTC 大周期摘要");
    expect(summary).toHaveTextContent("熊市反弹");
    expect(summary).toHaveTextContent("$60,000.00");
    expect(summary).toHaveTextContent("80/100");
    expect(summary).toHaveTextContent("-40.00%");
    expect(summary).toHaveTextContent("55.00%");
  });

  it("keeps the radar workspace usable when the macro summary request fails", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL) => {
        if (String(input).includes("/macro/btc")) {
          return {
            ok: false,
            json: async () => ({ message: "macro unavailable" }),
          };
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
    const summary = await screen.findByTestId("macro-summary-strip");
    expect(summary).toHaveTextContent("大周期数据不可用");
    expect(screen.getByRole("button", { name: "趋势" })).toBeInTheDocument();
  });

  it("selects symbols from the dense radar table", async () => {
    mockSnapshot();

    render(<App />);
    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("row", { name: /DOGE-USDT-SWAP/ }));

    const detail = screen.getByTestId("symbol-detail-panel");
    expect(detail).toHaveTextContent("DOGE-USDT-SWAP");
    expect(detail).toHaveTextContent("range long 82: near support");
  });

  it("opens TradingView from a table button without changing the selected detail symbol", async () => {
    mockSnapshot();

    render(<App />);
    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("row", { name: /DOGE-USDT-SWAP/ }));
    expect(screen.getByTestId("symbol-detail-panel")).toHaveTextContent("DOGE-USDT-SWAP");

    fireEvent.click(screen.getAllByRole("button", { name: "打开 LAB-USDT-SWAP TradingView 图表" })[0]);

    expect(screen.getByRole("dialog", { name: "LAB-USDT-SWAP TradingView" })).toBeInTheDocument();
    expect(screen.getByTestId("symbol-detail-panel")).toHaveTextContent("DOGE-USDT-SWAP");
  });

  it("renders AHR999 history guidance and paginated rows", async () => {
    mockSnapshot();

    render(<App />);
    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: "宏观" }));

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

  it("supports AHR999 tooltip, legend toggles, and page jump controls", async () => {
    const manyRowsMacro = {
      ...macro,
      ahr999_history: {
        ...macro.ahr999_history,
        points: Array.from({ length: 50 }, (_, index) => ({
          ts_ms: 1777670400000 + index * 86400000,
          date: `2026/05/${String(index + 1).padStart(2, "0")}`,
          value: 0.35 + index * 0.01,
          btc_price: 58000 + index * 100,
          gma200: 74000 + index * 50,
          model_price: 160000 + index * 20,
          zone: index < 10 ? "deep_value" : "accumulation",
        })),
      },
    } as BtcMacroSnapshot;
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL) => ({
        ok: true,
        json: async () => {
          if (String(input).includes("/chart")) {
            return chart;
          }
          if (String(input).includes("/macro/btc")) {
            return manyRowsMacro;
          }
          return snapshot;
        },
      })),
    );

    render(<App />);
    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: "宏观" }));

    expect(await screen.findByText("AHR999 历史")).toBeInTheDocument();
    fireEvent.pointerMove(screen.getByLabelText("AHR999 chart"), {
      clientX: 300,
      clientY: 200,
    });
    expect(screen.getByTestId("ahr999-tooltip")).toHaveTextContent("BTC Price");
    expect(screen.getByTestId("ahr999-tooltip")).toHaveTextContent("Ahr999 Index");

    const btcToggle = screen.getByRole("button", { name: "BTC Price" });
    fireEvent.click(btcToggle);
    expect(btcToggle).toHaveAttribute("aria-pressed", "false");
    expect(screen.getByLabelText("BTC Price series")).toHaveAttribute("aria-hidden", "true");

    fireEvent.change(screen.getByLabelText("跳转页码"), { target: { value: "2" } });
    expect(screen.getByRole("cell", { name: "2026/05/30" })).toBeInTheDocument();
  });

  it("renders analog forward K-line windows with score components", async () => {
    mockSnapshot();

    render(<App />);
    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: "宏观" }));

    expect(await screen.findByText("历史 K 线对比")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "90D" })).toHaveClass("active");
    expect(screen.getByText("当前节点前 90D")).toBeInTheDocument();
    expect(screen.getAllByText("历史相似+后续 90D 2021/01/01 · 82/100").length).toBeGreaterThan(0);
    expect(screen.getByLabelText("当前节点前 90D K线")).toBeInTheDocument();
    expect(screen.getByLabelText("历史相似+后续 90D 2021/01/01 K线")).toBeInTheDocument();
    expect(screen.getByText("锚点 2021/01/01 · $90.20")).toBeInTheDocument();
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

    fireEvent.click(screen.getByRole("button", { name: "宏观" }));

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

    fireEvent.click(screen.getByRole("button", { name: "交易" }));
    fireEvent.click(screen.getByRole("button", { name: "模拟买入 / 开多" }));

    expect(await screen.findByRole("button", { name: "模拟平仓" })).toBeInTheDocument();
    await waitFor(() => {
      expect(
        fetchMock.mock.calls.some((call) => String(call[0]).endsWith("/api/paper/orders")),
      ).toBe(true);
    });
  });
});
