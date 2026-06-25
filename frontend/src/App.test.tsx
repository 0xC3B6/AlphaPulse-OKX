import "@testing-library/jest-dom/vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import type { DashboardSnapshot } from "./types";

const snapshot: DashboardSnapshot = {
  last_scan_at_ms: 1782400000000,
  websocket_connected: true,
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
      pool_tags: ["dynamic"],
      trigger_reason: "trend short 84: volume 3.1x",
      funding_rate: -0.003,
      fvgs: [
        {
          timeframe: "m15",
          direction: "short",
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

afterEach(() => {
  vi.unstubAllGlobals();
});

function mockSnapshot(data: DashboardSnapshot = snapshot) {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => ({
      ok: true,
      json: async () => data,
    })),
  );
}

describe("App", () => {
  it("renders the radar title and connection status", async () => {
    mockSnapshot({ symbols: [], last_scan_at_ms: null, websocket_connected: false });
    render(<App />);
    expect(screen.getByText("AlphaPulse OKX")).toBeInTheDocument();
    expect(screen.getByText("Backend")).toBeInTheDocument();
    await screen.findByText("No symbols loaded");
  });

  it("loads symbols and filters trend opportunities", async () => {
    mockSnapshot();
    render(<App />);

    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);
    expect(screen.getByText("DOGE-USDT-SWAP")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Trend" }));

    expect(screen.getAllByText("LAB-USDT-SWAP").length).toBeGreaterThan(0);
    await waitFor(() => {
      expect(screen.queryByText("DOGE-USDT-SWAP")).not.toBeInTheDocument();
    });
  });
});
