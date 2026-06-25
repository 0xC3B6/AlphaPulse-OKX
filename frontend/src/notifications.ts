import type { SymbolSnapshot } from "./types";

export function shouldNotify(
  symbol: SymbolSnapshot,
  seen: Map<string, string>,
  threshold: number,
): boolean {
  const score =
    symbol.trend_score.value >= symbol.range_score.value
      ? symbol.trend_score
      : symbol.range_score;
  if (score.value < threshold || score.direction === "neutral") {
    return false;
  }

  const key = `${symbol.inst_id}:${score.direction}`;
  const value = `${Math.floor(score.value / 10)}:${symbol.trigger_reason}`;
  if (seen.get(key) === value) {
    return false;
  }

  seen.set(key, value);
  return true;
}

export function sendBrowserNotification(symbol: SymbolSnapshot): void {
  if (!("Notification" in window) || Notification.permission !== "granted") {
    return;
  }

  const score =
    symbol.trend_score.value >= symbol.range_score.value
      ? symbol.trend_score
      : symbol.range_score;
  new Notification(`${symbol.inst_id} ${score.direction} ${score.value}`, {
    body: symbol.trigger_reason,
  });
}
