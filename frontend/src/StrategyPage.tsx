import { useMemo, useState } from "react";
import type { Copy } from "./i18n";
import type {
  AttributionRow,
  PaperAccountSnapshot,
  RiskGuardEvent,
  Score,
  StrategyCenterSnapshot,
  StrategyVersionSnapshot,
  SymbolSnapshot,
} from "./types";
import {
  formatPct,
  formatPrice,
  formatSignalDirection,
  formatSignedUsdt,
  formatTags,
  formatTimestamp,
  formatUsdt,
  maxScore,
  pnlClass,
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
  onResetStrategyVersion,
  onStartStrategyVersion,
  onStopStrategyVersion,
  paper,
  strategyCenter,
  symbols,
}: {
  copy: Copy;
  lastScanAt: number | null;
  onResetStrategyVersion: (versionCode: string) => Promise<void>;
  onStartStrategyVersion: (versionCode: string) => Promise<void>;
  onStopStrategyVersion: (versionCode: string) => Promise<void>;
  paper: PaperAccountSnapshot;
  strategyCenter?: StrategyCenterSnapshot;
  symbols: SymbolSnapshot[];
}) {
  const candidates = useMemo(() => buildCandidates(symbols, paper), [paper, symbols]);
  const activeCandidates = candidates.filter((candidate) => isActionable(candidate.primaryScore));
  const featureStats = useMemo(() => buildFeatureStats(symbols), [symbols]);
  const longCount = activeCandidates.filter((candidate) => candidate.primaryScore.direction === "long").length;
  const shortCount = activeCandidates.filter((candidate) => candidate.primaryScore.direction === "short").length;
  const [activeTab, setActiveTab] = useState<StrategyTab>("attribution");
  const [selectedVersionCode, setSelectedVersionCode] = useState<string | null>(null);
  const selectedVersion =
    strategyCenter?.versions.find(
      (version) =>
        version.version.version_code ===
        (selectedVersionCode ?? strategyCenter.versions[0]?.version.version_code),
    ) ?? strategyCenter?.versions[0] ?? null;

  return (
    <section className="strategy-page page-surface" data-testid="strategy-page">
      {strategyCenter ? (
        <StrategyVersionCenter
          center={strategyCenter}
          onReset={onResetStrategyVersion}
          onSelectVersion={setSelectedVersionCode}
          onStart={onStartStrategyVersion}
          onStop={onStopStrategyVersion}
          selectedVersion={selectedVersion}
        />
      ) : null}

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

function StrategyVersionCenter({
  center,
  onReset,
  onSelectVersion,
  onStart,
  onStop,
  selectedVersion,
}: {
  center: StrategyCenterSnapshot;
  onReset: (versionCode: string) => Promise<void>;
  onSelectVersion: (versionCode: string) => void;
  onStart: (versionCode: string) => Promise<void>;
  onStop: (versionCode: string) => Promise<void>;
  selectedVersion: StrategyVersionSnapshot | null;
}) {
  const versions = center.versions;

  return (
    <section className="strategy-version-center detail-section" data-testid="strategy-version-center">
      <header className="panel-heading compact">
        <div>
          <h2>Strategy Version Runner</h2>
          <p>v0.1.3 / v0.1.4 paper accounts share market data but keep isolated positions, PnL and risk logs.</p>
        </div>
        <span className="strategy-center-updated">{formatTimestamp(center.last_updated_ms)}</span>
      </header>

      {versions.length === 0 ? (
        <p className="muted panel-empty">暂无策略版本</p>
      ) : (
        <div className="strategy-version-table-wrap">
          <table className="strategy-version-table">
            <thead>
              <tr>
                <th>Version</th>
                <th>Mode</th>
                <th>Equity</th>
                <th>PnL</th>
                <th>Risk</th>
                <th>Trades</th>
                <th>Config</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              {versions.map((version) => {
                const isSelected =
                  selectedVersion?.version.version_code === version.version.version_code;
                return (
                  <tr className={isSelected ? "active" : ""} key={version.version.version_code}>
                    <td>
                      <button
                        aria-label={`查看 ${version.version.version_code}`}
                        className="strategy-version-select"
                        onClick={() => onSelectVersion(version.version.version_code)}
                        type="button"
                      >
                        <strong>{version.version.version_code}</strong>
                        <span>{version.version.name}</span>
                      </button>
                    </td>
                    <td>
                      <span className={`strategy-status-pill ${version.run.status === "running" ? "active" : ""}`}>
                        {version.run.mode} {version.run.status}
                      </span>
                    </td>
                    <td>
                      <strong>{formatUsdt(version.overview.current_equity)}</strong>
                      <span className={pnlClass(version.overview.return_pct)}>
                        {formatPct(version.overview.return_pct)}
                      </span>
                    </td>
                    <td>
                      <span className={pnlClass(version.overview.realized_pnl)}>
                        {formatSignedUsdt(version.overview.realized_pnl)}
                      </span>
                      <small>{formatSignedUsdt(version.overview.unrealized_pnl)} UPNL</small>
                    </td>
                    <td>
                      <span>PF {formatNullableRatio(version.overview.profit_factor)}</span>
                      <span>DD {formatPct(version.overview.max_drawdown)}</span>
                    </td>
                    <td>
                      <span>{version.overview.closed_trades} closed</span>
                      <span>{version.overview.open_positions} open</span>
                    </td>
                    <td>
                      <code>{version.overview.config_hash}</code>
                    </td>
                    <td>
                      <div className="strategy-version-actions">
                        {version.run.status === "running" ? (
                          <button onClick={() => void onStop(version.version.version_code)} type="button">
                            停止 {version.version.version_code}
                          </button>
                        ) : (
                          <button onClick={() => void onStart(version.version.version_code)} type="button">
                            启动 {version.version.version_code}
                          </button>
                        )}
                        <button onClick={() => void onReset(version.version.version_code)} type="button">
                          重置 {version.version.version_code}
                        </button>
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      {selectedVersion ? <StrategyVersionDetail version={selectedVersion} /> : null}
    </section>
  );
}

function StrategyVersionDetail({ version }: { version: StrategyVersionSnapshot }) {
  return (
    <section className="strategy-version-detail" data-testid="strategy-version-detail">
      <header>
        <div>
          <h3>
            {version.version.version_code} · {version.version.name}
          </h3>
          <span>{version.run.run_id}</span>
        </div>
        <p>{version.version.description}</p>
      </header>

      <div className="strategy-version-detail-grid">
        <DetailMetric label="Equity" value={formatUsdt(version.overview.current_equity)} />
        <DetailMetric label="Return" value={formatPct(version.overview.return_pct)} />
        <DetailMetric label="PF" value={formatNullableRatio(version.overview.profit_factor)} />
        <DetailMetric label="Win" value={formatNullablePct(version.overview.win_rate)} />
        <DetailMetric label="Fee" value={formatUsdt(version.overview.total_fee)} />
        <DetailMetric label="Runtime" value={formatDuration(version.overview.run_time_ms)} />
      </div>

      <div className="strategy-version-panels">
        <AttributionMiniTable title="Signal Attribution" rows={version.signal_attribution} />
        <AttributionMiniTable title="Tag Attribution" rows={version.tag_attribution} />
        <AttributionMiniTable title="Combo Attribution" rows={version.combo_attribution} />
        <AttributionMiniTable title="Symbol Attribution" rows={version.symbol_attribution} />
        <PositionsMiniTable version={version} />
        <RiskGuardLog events={version.risk_guard_events} />
        <section className="strategy-config-panel">
          <h4>Config</h4>
          <pre>{JSON.stringify(version.version.config_json, null, 2)}</pre>
        </section>
      </div>
    </section>
  );
}

function DetailMetric({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function AttributionMiniTable({ rows, title }: { rows: AttributionRow[]; title: string }) {
  return (
    <section className="strategy-attribution-mini">
      <h4>{title}</h4>
      {rows.length === 0 ? (
        <p className="muted">暂无归因样本</p>
      ) : (
        <table>
          <thead>
            <tr>
              <th>Key</th>
              <th>N</th>
              <th>PF</th>
              <th>PnL</th>
              <th>SL</th>
              <th>Suggestion</th>
            </tr>
          </thead>
          <tbody>
            {rows.slice(0, 5).map((row) => (
              <tr key={row.key}>
                <td>{row.key}</td>
                <td>{row.sample_count}</td>
                <td>{formatNullableRatio(row.profit_factor)}</td>
                <td className={pnlClass(row.net_pnl)}>{formatSignedUsdt(row.net_pnl)}</td>
                <td>{formatNullablePct(row.stop_loss_rate)}</td>
                <td>{row.confidence} / {row.suggestion}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}

function PositionsMiniTable({ version }: { version: StrategyVersionSnapshot }) {
  return (
    <section className="strategy-positions-mini">
      <h4>Open Positions</h4>
      {version.paper.positions.length === 0 ? (
        <p className="muted">暂无当前持仓</p>
      ) : (
        <table>
          <thead>
            <tr>
              <th>Symbol</th>
              <th>Side</th>
              <th>Margin</th>
              <th>PnL</th>
              <th>Signal</th>
              <th>Risk</th>
            </tr>
          </thead>
          <tbody>
            {version.paper.positions.map((position) => (
              <tr key={`${position.run_id}-${position.inst_id}`}>
                <td>{position.inst_id}</td>
                <td>{position.side}</td>
                <td>{formatUsdt(position.margin)}</td>
                <td className={pnlClass(position.unrealized_pnl)}>
                  {formatSignedUsdt(position.unrealized_pnl)}
                </td>
                <td>{position.primary_signal ?? "-"}</td>
                <td>{(position.risk_flags ?? []).join(", ") || "-"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}

function RiskGuardLog({ events }: { events: RiskGuardEvent[] }) {
  return (
    <section className="strategy-risk-log">
      <h4>Risk Guard Log</h4>
      {events.length === 0 ? (
        <p className="muted">暂无风控拦截</p>
      ) : (
        <table>
          <thead>
            <tr>
              <th>Time</th>
              <th>Symbol</th>
              <th>Signal</th>
              <th>Reason</th>
              <th>Flags</th>
            </tr>
          </thead>
          <tbody>
            {events.slice(0, 8).map((event) => (
              <tr key={event.id}>
                <td>{formatTimestamp(event.timestamp_ms)}</td>
                <td>{event.symbol}</td>
                <td>{event.original_signal}</td>
                <td>{event.reason}</td>
                <td>{event.risk_flags.join(", ")}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}

function formatNullableRatio(value: number | null | undefined): string {
  return value === null || value === undefined ? "-" : value.toFixed(2);
}

function formatNullablePct(value: number | null | undefined): string {
  return value === null || value === undefined ? "-" : formatPct(value);
}

function formatDuration(ms: number): string {
  if (ms < 60_000) {
    return `${Math.max(0, Math.round(ms / 1_000))}s`;
  }
  if (ms < 3_600_000) {
    return `${Math.round(ms / 60_000)}m`;
  }
  if (ms < 86_400_000) {
    return `${Math.round(ms / 3_600_000)}h`;
  }
  return `${Math.round(ms / 86_400_000)}d`;
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
