import type { Copy } from "./i18n";
import type { PaperAccountSnapshot } from "./types";
import {
  formatPct,
  formatPrice,
  formatSignedUsdt,
  formatTimestamp,
  formatUsdt,
  pnlClass,
  summarizePaperReview,
} from "./uiFormat";

export function ReviewPage({ copy, paper }: { copy: Copy; paper: PaperAccountSnapshot }) {
  const summary = summarizePaperReview(paper);
  const profitFactor =
    summary.profitFactor === null ? "-" : summary.profitFactor.toFixed(2);

  return (
    <section className="review-page page-surface" data-testid="review-page">
      <section className="page-local-tabs" aria-label={copy.views.review}>
        <span className="active">{copy.review.performance}</span>
        <span>{copy.review.tradeRecords}</span>
        <span>{copy.review.historyUnavailable}</span>
      </section>
      <section className="page-metric-grid review-summary">
        <MetricCard label={copy.paper.realized} value={formatSignedUsdt(paper.realized_pnl)} tone={pnlClass(paper.realized_pnl)} />
        <MetricCard label={copy.paper.unrealized} value={formatSignedUsdt(paper.unrealized_pnl)} tone={pnlClass(paper.unrealized_pnl)} />
        <MetricCard label={copy.review.winRate} value={formatPct(summary.winRate)} />
        <MetricCard label={copy.review.closedTrades} value={String(summary.closedCount)} />
        <MetricCard label={copy.review.averageWin} value={formatSignedUsdt(summary.averageWin)} tone={pnlClass(summary.averageWin)} />
        <MetricCard label={copy.review.averageLoss} value={formatSignedUsdt(summary.averageLoss)} tone={pnlClass(summary.averageLoss)} />
        <MetricCard label={copy.review.maxWin} value={formatSignedUsdt(summary.maxWin)} tone={pnlClass(summary.maxWin)} />
        <MetricCard label={copy.review.maxLoss} value={formatSignedUsdt(summary.maxLoss)} tone={pnlClass(summary.maxLoss)} />
        <MetricCard label={copy.review.profitFactor} value={profitFactor} />
        <MetricCard label={copy.paper.equity} value={formatUsdt(paper.equity)} />
      </section>
      <section className="review-grid">
        <section className="detail-section review-chart-card">
          <header className="panel-heading">
            <div>
              <h2>{copy.review.realizedCurve}</h2>
              <p>{copy.paper.realized}: {formatSignedUsdt(paper.realized_pnl)}</p>
            </div>
          </header>
          <RealizedCurve points={summary.realizedPath} />
        </section>
        <section className="detail-section review-unavailable">
          <h2>{copy.review.strategyUnavailable}</h2>
          <p className="muted">{copy.review.historyUnavailable}</p>
        </section>
        <section className="table-panel review-trades-panel">
          <header className="panel-heading">
            <div>
              <h2>{copy.review.tradeRecords}</h2>
              <p>{paper.trades.length} {copy.paper.history}</p>
            </div>
          </header>
          {paper.trades.length === 0 ? (
            <p className="muted panel-empty">{copy.paper.noTrades}</p>
          ) : (
            <table className="review-trade-table">
              <thead>
                <tr>
                  <th>{copy.table.symbol}</th>
                  <th>{copy.paper.tradeActions.open}</th>
                  <th>{copy.paper.side}</th>
                  <th>{copy.table.price}</th>
                  <th>{copy.paper.realized}</th>
                  <th>{copy.status.lastScan}</th>
                </tr>
              </thead>
              <tbody>
                {paper.trades.map((trade) => (
                  <tr key={trade.id}>
                    <td>{trade.inst_id}</td>
                    <td>{copy.paper.tradeActions[trade.action]}</td>
                    <td>{copy.directions[trade.side]}</td>
                    <td>{formatPrice(trade.price)}</td>
                    <td className={pnlClass(trade.realized_pnl)}>
                      {formatSignedUsdt(trade.realized_pnl)}
                    </td>
                    <td>{formatTimestamp(trade.ts_ms)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </section>
      </section>
    </section>
  );
}

function MetricCard({
  label,
  tone,
  value,
}: {
  label: string;
  tone?: string;
  value: string;
}) {
  return (
    <div className="metric-card">
      <span>{label}</span>
      <strong className={tone ?? ""}>{value}</strong>
    </div>
  );
}

function RealizedCurve({ points }: { points: Array<{ id: number; value: number }> }) {
  if (points.length === 0) {
    return <div className="review-empty-chart" />;
  }

  const values = points.map((point) => point.value);
  const min = Math.min(0, ...values);
  const max = Math.max(0, ...values);
  const span = max - min || 1;
  const path = points
    .map((point, index) => {
      const x = points.length === 1 ? 100 : (index / (points.length - 1)) * 100;
      const y = 100 - ((point.value - min) / span) * 100;
      return `${index === 0 ? "M" : "L"} ${x.toFixed(2)} ${y.toFixed(2)}`;
    })
    .join(" ");

  return (
    <svg className="review-curve" role="img" aria-label="Realized PnL curve" viewBox="0 0 100 100" preserveAspectRatio="none">
      <path d="M 0 100 L 100 100" className="review-curve-axis" />
      <path d={path} className="review-curve-line" />
    </svg>
  );
}
