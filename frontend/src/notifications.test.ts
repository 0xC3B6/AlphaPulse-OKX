import { describe, expect, it } from "vitest";
import { shouldNotify } from "./notifications";
import type { SymbolSnapshot } from "./types";

function symbol(value: number, direction: "long" | "short" | "neutral"): SymbolSnapshot {
  return {
    inst_id: "LAB-USDT-SWAP",
    price: 17.2,
    change_5m_pct: -0.03,
    change_15m_pct: -0.07,
    change_1h_pct: -0.11,
    trend_score: { value, direction, reasons: ["volume 3.1x"] },
    range_score: { value: 20, direction: "neutral", reasons: [] },
    pool_tags: ["dynamic"],
    trigger_reason: "trend short 84: volume 3.1x",
    funding_rate: -0.003,
    fvgs: [],
    levels: [],
    updated_at_ms: 1782400000000,
  };
}

describe("shouldNotify", () => {
  it("notifies when a symbol newly enters high trend score", () => {
    const seen = new Map<string, string>();
    expect(shouldNotify(symbol(84, "short"), seen, 80)).toBe(true);
    expect(shouldNotify(symbol(84, "short"), seen, 80)).toBe(false);
  });

  it("notifies again when direction changes", () => {
    const seen = new Map<string, string>();
    expect(shouldNotify(symbol(84, "short"), seen, 80)).toBe(true);
    expect(shouldNotify(symbol(86, "long"), seen, 80)).toBe(true);
  });
});
