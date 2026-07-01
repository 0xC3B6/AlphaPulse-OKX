import type { Copy } from "./i18n";
import type { MacroRegime, Score, SymbolSnapshot } from "./types";

export type Filter = "all" | "trend" | "range" | "hot" | "fixed";
export type ThemeMode = "light" | "dark" | "system";
export type ViewMode = "radar" | "macro";

export function matchesFilter(symbol: SymbolSnapshot, filter: Filter): boolean {
  if (filter === "trend") {
    return symbol.trend_score.value >= 65;
  }
  if (filter === "range") {
    return symbol.range_score.value >= 65;
  }
  if (filter === "hot") {
    return symbol.pool_tags.includes("dynamic");
  }
  if (filter === "fixed") {
    return symbol.pool_tags.includes("fixed");
  }
  return true;
}

export function maxScore(symbol: SymbolSnapshot): number {
  return Math.max(symbol.trend_score.value, symbol.range_score.value);
}

export function primaryScore(symbol: SymbolSnapshot): Score {
  return symbol.trend_score.value >= symbol.range_score.value
    ? symbol.trend_score
    : symbol.range_score;
}

export function scoreTone(score: Score): "positive" | "negative" | "" {
  if (score.value < 80) {
    return "";
  }
  return score.direction === "short" ? "negative" : "positive";
}

export function formatScore(score: Score, copy: Copy): string {
  return `${score.value} ${copy.directions[score.direction]}`;
}

export function formatSignalDirection(direction: Score["direction"]): string {
  return direction.toUpperCase();
}

export function formatTags(tags: string[], copy: Copy): string {
  if (tags.length === 0) {
    return copy.misc.unlabeled;
  }
  return tags.map((tag) => formatTag(tag, copy)).join(" / ");
}

export function formatTag(tag: string, copy: Copy): string {
  const labels = copy.poolTags as unknown as Record<string, string>;
  return labels[tag] ?? tag;
}

export function formatState(value: string, copy: Copy): string {
  return copy.states[value as keyof Copy["states"]] ?? value;
}

export function formatPrice(value: number): string {
  if (value >= 100) {
    return value.toLocaleString(undefined, {
      maximumFractionDigits: 2,
      minimumFractionDigits: 2,
    });
  }
  if (value >= 1) {
    return value.toFixed(4);
  }
  return value.toFixed(6);
}

export function formatQuantity(value: number): string {
  if (value >= 100) {
    return value.toFixed(2);
  }
  if (value >= 1) {
    return value.toFixed(4);
  }
  return value.toFixed(6);
}

export function formatUsdt(value: number): string {
  return `${value.toLocaleString(undefined, {
    maximumFractionDigits: 2,
    minimumFractionDigits: 2,
  })} USDT`;
}

export function formatUsd(value: number): string {
  return `$${value.toLocaleString(undefined, {
    maximumFractionDigits: 2,
    minimumFractionDigits: 2,
  })}`;
}

export function formatSignedUsdt(value: number): string {
  const prefix = value > 0 ? "+" : "";
  return `${prefix}${formatUsdt(value)}`;
}

export function formatPct(value: number): string {
  return `${(value * 100).toFixed(2)}%`;
}

export function pnlClass(value: number): string {
  if (value > 0) {
    return "positive";
  }
  if (value < 0) {
    return "negative";
  }
  return "";
}

export function formatTemplate(template: string, symbol: string): string {
  return template.replace("{symbol}", symbol);
}

export function formatTimestamp(value: number | null): string {
  if (value === null) {
    return "-";
  }
  return new Date(value).toLocaleTimeString();
}

export function formatDate(value: number): string {
  return new Date(value).toLocaleDateString();
}

export function formatRegime(regime: MacroRegime, copy: Copy): string {
  return copy.macro.regimes[regime] ?? formatSnake(regime);
}

export function formatSnake(value: string): string {
  return value.replace(/_/g, " ");
}
