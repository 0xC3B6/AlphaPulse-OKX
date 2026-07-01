import { describe, expect, it } from "vitest";
import { translations } from "./i18n";
import {
  formatPct,
  formatPrice,
  formatRegime,
  formatSignalDirection,
  formatTags,
  maxScore,
  primaryScore,
} from "./uiFormat";
import type { SymbolSnapshot } from "./types";

const copy = translations.zh;

const symbol: SymbolSnapshot = {
  inst_id: "ETH-USDT-SWAP",
  price: 1585.292,
  change_5m_pct: 0.0013,
  change_15m_pct: 0.0022,
  change_1h_pct: 0.0118,
  trend_score: { value: 82, direction: "long", reasons: ["trend"] },
  range_score: { value: 90, direction: "long", reasons: ["range"] },
  pool_tags: ["fixed", "manual_watch"],
  trigger_reason: "range long 90: clear recent range",
  funding_rate: null,
  fvgs: [],
  levels: [],
  updated_at_ms: 1782400000000,
};

describe("uiFormat", () => {
  it("formats table values and tags", () => {
    expect(formatPrice(symbol.price)).toBe("1585.29");
    expect(formatPct(symbol.change_15m_pct)).toBe("0.22%");
    expect(formatTags(symbol.pool_tags, copy)).toBe("固定 / 手动关注");
  });

  it("selects the primary score for a symbol", () => {
    expect(maxScore(symbol)).toBe(90);
    expect(primaryScore(symbol)).toEqual(symbol.range_score);
    expect(formatSignalDirection(symbol.range_score.direction)).toBe("LONG");
  });

  it("formats macro regimes through translations", () => {
    expect(formatRegime("bear_market_rally", copy)).toBe("熊市反弹");
  });
});
