import { useMemo, useState } from "react";
import type { Copy } from "./i18n";
import type { PaperAccountSnapshot, Score, SymbolSnapshot } from "./types";
import {
  formatPct,
  formatPrice,
  formatSignalDirection,
  formatTags,
  formatTimestamp,
  maxScore,
} from "./uiFormat";

interface StrategyCandidate {
  featureCount: number;
  hasPosition: boolean;
  primaryKind: "trend" | "range";
  primaryScore: Score;
  reasons: string[];
  secondaryScore: Score;
  symbol: SymbolSnapshot;
}

interface FeatureStat {
  averageScore: number;
  count: number;
  label: string;
  longCount: number;
  shortCount: number;
}

type StrategyTab = "attribution" | "features" | "shadow";

const strategyTabs: Array<{ id: StrategyTab; label: string }> = [
  { id: "attribution", label: "信号归因" },
  { id: "features", label: "特征分析" },
  { id: "shadow", label: "Shadow 持仓" },
];

export function StrategyPage({
  copy,
  lastScanAt,
  paper,
  symbols,
}: {
  copy: Copy;
  lastScanAt: number | null;
  paper: PaperAccountSnapshot;
  symbols: SymbolSnapshot[];
}) {
  const candidates = useMemo(() => buildCandidates(symbols, paper), [paper, symbols]);
  const activeCandidates = candidates.filter((candidate) => isActionable(candidate.primaryScore));
  const featureStats = useMemo(() => buildFeatureStats(symbols), [symbols]);
  const longCount = activeCandidates.filter((candidate) => candidate.primaryScore.direction === "long").length;
  const shortCount = activeCandidates.filter((candidate) => candidate.primaryScore.direction === "short").length;
  const [activeTab, setActiveTab] = useState<StrategyTab>("attribution");

  return (
    <section className="strategy-page page-surface" data-testid="strategy-page">
      <section className="page-local-tabs" role="tablist" aria-label={copy.views.strategy}>
        {strategyTabs.map((tab) => (
          <button
            aria-controls={`strategy-panel-${tab.id}`}
            aria-selected={activeTab === tab.id}
            className={activeTab === tab.id ? "active" : undefined}
            id={`strategy-tab-${tab.id}`}
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            role="tab"
            type="button"
          >
            {tab.label}
          </button>
        ))}
      </section>

      <section className="page-metric-grid strategy-summary">
        <MetricCard label="候选信号" value={String(activeCandidates.length)} />
        <MetricCard label="LONG / SHORT" value={`${longCount} / ${shortCount}`} />
        <MetricCard label="特征命中" value={String(featureStats.length)} />
        <MetricCard label="当前持仓" value={String(paper.positions.length)} />
        <MetricCard label="最近扫描" value={formatTimestamp(lastScanAt)} />
      </section>

      {activeTab === "attribution" ? (
        <section
          aria-labelledby="strategy-tab-attribution"
          className="detail-section strategy-signal-panel"
          data-testid="strategy-attribution-panel"
          id="strategy-panel-attribution"
          role="tabpanel"
        >
          <header className="panel-heading compact">
            <div>
              <h2>信号归因</h2>
              <p>按趋势/震荡主评分排序，展示触发原因和贡献特征。</p>
            </div>
          </header>
          {candidates.length === 0 ? (
            <p className="muted panel-empty">暂无可归因信号</p>
          ) : (
            <div className="strategy-signal-list">
              {candidates.slice(0, 12).map((candidate) => (
                <article className="strategy-signal-card" key={candidate.symbol.inst_id}>
                  <header>
                    <div>
                      <strong>{candidate.symbol.inst_id}</strong>
                      <span>{candidate.primaryKind === "trend" ? "趋势模型" : "震荡模型"}</span>
                    </div>
                    <SignalBadge score={candidate.primaryScore} />
                  </header>
                  <dl className="strategy-score-grid">
                    <Metric label="价格" value={formatPrice(candidate.symbol.price)} />
                    <Metric label="1h" value={formatPct(candidate.symbol.change_1h_pct)} />
                    <Metric label="Trend" value={scoreText(candidate.symbol.trend_score, copy)} />
                    <Metric label="Range" value={scoreText(candidate.symbol.range_score, copy)} />
                  </dl>
                  <p className="strategy-trigger">{candidate.symbol.trigger_reason || copy.misc.watching}</p>
                  <ul className="strategy-reason-list">
                    {candidate.reasons.map((reason) => (
                      <li key={reason}>{reason}</li>
                    ))}
                  </ul>
                  <footer>
                    <span>{formatTags(candidate.symbol.pool_tags, copy)}</span>
                    <span>{candidate.featureCount} features</span>
                    {candidate.hasPosition ? <b>IN POSITION</b> : null}
                  </footer>
                </article>
              ))}
            </div>
          )}
        </section>
      ) : null}

      {activeTab === "features" ? (
        <section
          aria-labelledby="strategy-tab-features"
          className="detail-section strategy-feature-panel"
          data-testid="strategy-feature-panel"
          id="strategy-panel-features"
          role="tabpanel"
        >
          <header className="panel-heading compact">
            <div>
              <h2>特征分析</h2>
              <p>聚合 score reasons、触发原因、标签、FVG 和支撑阻力。</p>
            </div>
          </header>
          {featureStats.length === 0 ? (
            <p className="muted panel-empty">暂无特征命中</p>
          ) : (
            <ul className="strategy-feature-list">
              {featureStats.slice(0, 12).map((feature) => (
                <li key={feature.label}>
                  <div>
                    <strong>{feature.label}</strong>
                    <span>
                      {feature.count} hits · avg {feature.averageScore.toFixed(0)}
                    </span>
                  </div>
                  <span className="strategy-feature-bias">
                    L{feature.longCount} / S{feature.shortCount}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </section>
      ) : null}

      {activeTab === "shadow" ? (
        <section
          aria-labelledby="strategy-tab-shadow"
          className="detail-section strategy-shadow-panel"
          data-testid="strategy-shadow-panel"
          id="strategy-panel-shadow"
          role="tabpanel"
        >
        <header className="panel-heading compact">
          <div>
            <h2>Shadow 持仓对照</h2>
            <p>把当前持仓和实时信号放在同一个策略视图里。</p>
          </div>
        </header>
        {paper.positions.length === 0 ? (
          <p className="muted panel-empty">暂无 Shadow 持仓</p>
        ) : (
          <ul className="strategy-position-list">
            {paper.positions.map((position) => (
              <li key={position.inst_id}>
                <strong>{position.inst_id}</strong>
                <span>{copy.directions[position.side]} · {position.leverage.toFixed(1)}x</span>
                <span>{position.reason || "manual / paper position"}</span>
              </li>
            ))}
          </ul>
        )}
      </section>
      ) : null}
    </section>
  );
}

function buildCandidates(
  symbols: SymbolSnapshot[],
  paper: PaperAccountSnapshot,
): StrategyCandidate[] {
  const positionIds = new Set(paper.positions.map((position) => position.inst_id));
  return symbols
    .map((symbol) => {
      const primaryKind: StrategyCandidate["primaryKind"] =
        symbol.trend_score.value >= symbol.range_score.value ? "trend" : "range";
      const primaryScore = primaryKind === "trend" ? symbol.trend_score : symbol.range_score;
      const secondaryScore = primaryKind === "trend" ? symbol.range_score : symbol.trend_score;
      const reasons = uniqueStrings([
        ...primaryScore.reasons,
        ...secondaryScore.reasons,
      ]);

      return {
        featureCount: reasons.length + symbol.pool_tags.length + symbol.fvgs.length + symbol.levels.length,
        hasPosition: positionIds.has(symbol.inst_id),
        primaryKind,
        primaryScore,
        reasons,
        secondaryScore,
        symbol,
      };
    })
    .sort((left, right) => maxScore(right.symbol) - maxScore(left.symbol));
}

function buildFeatureStats(symbols: SymbolSnapshot[]): FeatureStat[] {
  const stats = new Map<string, { count: number; scoreTotal: number; longCount: number; shortCount: number }>();

  for (const symbol of symbols) {
    const score = symbol.trend_score.value >= symbol.range_score.value
      ? symbol.trend_score
      : symbol.range_score;
    const features = uniqueStrings([
      symbol.trigger_reason,
      ...score.reasons,
      ...symbol.pool_tags.map((tag) => `标签 ${tag}`),
      ...symbol.fvgs.map((zone) => `FVG ${zone.timeframe} ${zone.direction}`),
      ...symbol.levels.map((level) => `${level.kind} ${level.touches} touches`),
    ]);

    for (const feature of features) {
      const current = stats.get(feature) ?? {
        count: 0,
        longCount: 0,
        scoreTotal: 0,
        shortCount: 0,
      };
      current.count += 1;
      current.scoreTotal += score.value;
      if (score.direction === "long") {
        current.longCount += 1;
      }
      if (score.direction === "short") {
        current.shortCount += 1;
      }
      stats.set(feature, current);
    }
  }

  return [...stats.entries()]
    .map(([label, stat]) => ({
      averageScore: stat.scoreTotal / stat.count,
      count: stat.count,
      label,
      longCount: stat.longCount,
      shortCount: stat.shortCount,
    }))
    .sort((left, right) => right.count - left.count || right.averageScore - left.averageScore);
}

function isActionable(score: Score): boolean {
  return score.direction !== "neutral" && score.value >= 65;
}

function scoreText(score: Score, copy: Copy): string {
  return `${Math.round(score.value)} ${copy.directions[score.direction]}`;
}

function uniqueStrings(values: string[]): string[] {
  return [...new Set(values.map((value) => value.trim()).filter(Boolean))];
}

function SignalBadge({ score }: { score: Score }) {
  const tone = score.direction === "long" ? "positive" : score.direction === "short" ? "negative" : "";
  return (
    <span className={`signal-pill ${tone}`}>
      {formatSignalDirection(score.direction)} {Math.round(score.value)}
    </span>
  );
}

function MetricCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="metric-card">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
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
