import { useMemo, useState } from "react";
import type { Copy } from "./i18n";
import type { PaperAccountSnapshot, Score, SymbolSnapshot } from "./types";
import {
  formatPct,
  formatPrice,
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

type StrategyTab = "attribution" | "features";

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
  const featureStats = useMemo(() => buildFeatureStats(symbols, copy), [copy, symbols]);
  const longCount = activeCandidates.filter((candidate) => candidate.primaryScore.direction === "long").length;
  const shortCount = activeCandidates.filter((candidate) => candidate.primaryScore.direction === "short").length;
  const [activeTab, setActiveTab] = useState<StrategyTab>("attribution");
  const strategyTabs: Array<{ id: StrategyTab; label: string }> = [
    { id: "attribution", label: copy.strategy.attributionTab },
    { id: "features", label: copy.strategy.featureTab },
  ];

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
        <MetricCard label={copy.strategy.candidateSignals} value={String(activeCandidates.length)} />
        <MetricCard label={copy.strategy.longShort} value={`${longCount} / ${shortCount}`} />
        <MetricCard label={copy.strategy.featureHits} value={String(featureStats.length)} />
        <MetricCard label={copy.strategy.currentPositions} value={String(paper.positions.length)} />
        <MetricCard label={copy.strategy.lastScan} value={formatTimestamp(lastScanAt)} />
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
              <h2>{copy.strategy.attributionTitle}</h2>
              <p>{copy.strategy.attributionDescription}</p>
            </div>
          </header>
          {candidates.length === 0 ? (
            <p className="muted panel-empty">{copy.strategy.noAttribution}</p>
          ) : (
            <div className="strategy-signal-list">
              {candidates.slice(0, 12).map((candidate) => (
                <article className="strategy-signal-card" key={candidate.symbol.inst_id}>
                  <header>
                    <div>
                      <strong>{candidate.symbol.inst_id}</strong>
                      <span>
                        {candidate.primaryKind === "trend"
                          ? copy.strategy.trendModel
                          : copy.strategy.rangeModel}
                      </span>
                    </div>
                    <SignalBadge copy={copy} score={candidate.primaryScore} />
                  </header>
                  <dl className="strategy-score-grid">
                    <Metric label={copy.strategy.price} value={formatPrice(candidate.symbol.price)} />
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
                    <span>{candidate.featureCount} {copy.strategy.features}</span>
                    {candidate.hasPosition ? <b>{copy.strategy.inPosition}</b> : null}
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
              <h2>{copy.strategy.featureTitle}</h2>
              <p>{copy.strategy.featureSummary}</p>
            </div>
          </header>
          <aside className="strategy-feature-explainer">
            <strong>{copy.strategy.featureExplanationTitle}</strong>
            <p>{copy.strategy.featureExplanationBody}</p>
          </aside>
          {featureStats.length === 0 ? (
            <p className="muted panel-empty">{copy.strategy.noFeatures}</p>
          ) : (
            <ul className="strategy-feature-list">
              {featureStats.slice(0, 12).map((feature) => (
                <li key={feature.label}>
                  <div>
                    <strong>{feature.label}</strong>
                    <span>
                      {feature.count} {copy.strategy.hits} · {copy.strategy.average} {feature.averageScore.toFixed(0)}
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

function buildFeatureStats(symbols: SymbolSnapshot[], copy: Copy): FeatureStat[] {
  const stats = new Map<string, { count: number; scoreTotal: number; longCount: number; shortCount: number }>();

  for (const symbol of symbols) {
    const score = symbol.trend_score.value >= symbol.range_score.value
      ? symbol.trend_score
      : symbol.range_score;
    const features = uniqueStrings([
      symbol.trigger_reason,
      ...score.reasons,
      ...symbol.pool_tags.map((tag) => `${copy.strategy.tagFeature} ${tag}`),
      ...symbol.fvgs.map((zone) => `FVG ${zone.timeframe} ${zone.direction}`),
      ...symbol.levels.map(
        (level) =>
          `${copy.levelKinds[level.kind]} ${level.touches} ${copy.strategy.touches}`,
      ),
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

function SignalBadge({ copy, score }: { copy: Copy; score: Score }) {
  const tone = score.direction === "long" ? "positive" : score.direction === "short" ? "negative" : "";
  return (
    <span className={`signal-pill ${tone}`}>
      {copy.directions[score.direction]} {Math.round(score.value)}
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
