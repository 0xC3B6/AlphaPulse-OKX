import { useState } from "react";
import type { Copy } from "./i18n";
import type {
  PaperAccountSnapshot,
  PaperPositionSnapshot,
  PaperSide,
  SymbolSnapshot,
} from "./types";
import {
  formatPct,
  formatPrice,
  formatQuantity,
  formatSignedUsdt,
  formatTimestamp,
  formatUsdt,
  pnlClass,
} from "./uiFormat";

export function TradePage({
  copy,
  onClosePaper,
  onInstrumentChange,
  onLeverageChange,
  onMarginChange,
  onOpenPaper,
  onSelectPosition,
  orderInstrument,
  orderLeverage,
  orderMargin,
  paper,
  selectedPosition,
  symbols,
  tradeBusy,
  tradeError,
}: {
  copy: Copy;
  onClosePaper: (instId: string) => void;
  onInstrumentChange: (value: string) => void;
  onLeverageChange: (value: string) => void;
  onMarginChange: (value: string) => void;
  onOpenPaper: (side: PaperSide, instId: string) => void;
  onSelectPosition: (instId: string) => void;
  orderInstrument: string;
  orderLeverage: string;
  orderMargin: string;
  paper: PaperAccountSnapshot;
  selectedPosition: PaperPositionSnapshot | null;
  symbols: SymbolSnapshot[];
  tradeBusy: boolean;
  tradeError: string | null;
}) {
  const knownSymbolIds = symbols.map((symbol) => symbol.inst_id);
  const [reasonPosition, setReasonPosition] = useState<PaperPositionSnapshot | null>(null);

  return (
    <section className="trade-page page-surface" data-testid="trade-page">
      <section className="page-local-tabs" aria-label={copy.views.trade}>
        <span className="active">{copy.trade.currentPositions}</span>
        <span>{copy.trade.quickOrder}</span>
        <span>{copy.trade.recentTrades}</span>
      </section>
      <section className="trade-strategy-identity" aria-label={copy.trade.restoredStrategy}>
        <div>
          <span>{copy.trade.restoredStrategy}</span>
          <strong>{paper.strategy_version}</strong>
        </div>
        <div>
          <span>{copy.trade.build}</span>
          <code>{paper.strategy_build_id}</code>
        </div>
        <div>
          <span>{copy.trade.run}</span>
          <code>{paper.run_id}</code>
        </div>
        <div title={paper.config_hash}>
          <span>{copy.trade.config}</span>
          <code>{shortHash(paper.config_hash)}</code>
        </div>
        <div>
          <span>{copy.trade.persistence}</span>
          <strong
            className={
              paper.persistence.status === "healthy"
                ? "persistence-healthy"
                : "persistence-paused"
            }
          >
            {paper.persistence.status === "healthy"
              ? copy.trade.persisted
              : copy.trade.persistencePaused}
          </strong>
          {paper.persistence.last_committed_at_ms ? (
            <small>{formatTimestamp(paper.persistence.last_committed_at_ms)}</small>
          ) : null}
        </div>
        {paper.persistence.last_error ? (
          <p className="trade-persistence-error">{paper.persistence.last_error}</p>
        ) : null}
      </section>
      <section className="page-metric-grid trade-summary">
        <MetricCard label={copy.paper.equity} value={formatUsdt(paper.equity)} />
        <MetricCard label={copy.paper.available} value={formatUsdt(paper.available_balance)} />
        <MetricCard label={copy.paper.usedMargin} value={formatUsdt(paper.used_margin)} />
        <MetricCard
          label={copy.paper.unrealized}
          tone={pnlClass(paper.unrealized_pnl)}
          value={formatSignedUsdt(paper.unrealized_pnl)}
        />
      </section>
      <section className="trade-grid">
        <section className="table-panel trade-positions-panel">
          <header className="panel-heading">
            <div>
              <h2>{copy.trade.allPositions}</h2>
              <p>
                {paper.positions.length} {copy.trade.currentPositions}
              </p>
            </div>
          </header>
          {paper.positions.length === 0 ? (
            <p className="muted panel-empty">{copy.trade.noPositions}</p>
          ) : (
            <table className="trade-table">
              <thead>
                <tr>
                  <th>{copy.table.symbol}</th>
                  <th>{copy.paper.side}</th>
                  <th>{copy.paper.leverage}</th>
                  <th>{copy.paper.margin}</th>
                  <th>{copy.paper.pnl}</th>
                  <th>{copy.paper.mark}</th>
                  <th>{copy.trade.stopLoss}</th>
                  <th>{copy.trade.takeProfit}</th>
                  <th>{copy.trade.signal}</th>
                  <th>{copy.trade.reason}</th>
                </tr>
              </thead>
              <tbody>
                {paper.positions.map((position) => (
                  <tr
                    className={position.inst_id === selectedPosition?.inst_id ? "selected" : ""}
                    key={position.inst_id}
                    onClick={() => onSelectPosition(position.inst_id)}
                  >
                    <td>{position.inst_id}</td>
                    <td>{copy.directions[position.side]}</td>
                    <td>{position.leverage.toFixed(1)}x</td>
                    <td>{formatUsdt(position.margin)}</td>
                    <td className={pnlClass(position.unrealized_pnl)}>
                      {formatSignedUsdt(position.unrealized_pnl)} / {formatPct(position.pnl_pct)}
                    </td>
                    <td>{formatPrice(position.mark_price)}</td>
                    <td>{formatOptionalPrice(position.stop_loss)}</td>
                    <td>{formatOptionalPrice(position.take_profit)}</td>
                    <td>{position.primary_signal || "—"}</td>
                    <td>
                      <div className="trade-reason-cell">
                        <span title={position.reason}>{position.reason || "—"}</span>
                        {position.reason ? (
                          <button
                            aria-label={copy.trade.viewReasonDetails.replace(
                              "{symbol}",
                              position.inst_id,
                            )}
                            className="trade-reason-detail-button"
                            onClick={(event) => {
                              event.stopPropagation();
                              onSelectPosition(position.inst_id);
                              setReasonPosition(position);
                            }}
                            type="button"
                          >
                            {copy.trade.reasonDetails}
                          </button>
                        ) : null}
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </section>
        <aside className="trade-side-panel">
          <section className="detail-section">
            <h2>{copy.trade.quickOrder}</h2>
            <div className="paper-order trade-order">
              <label>
                <span>{copy.trade.orderInstrument}</span>
                <input
                  aria-label={copy.trade.orderInstrument}
                  list="trade-symbols"
                  onChange={(event) => onInstrumentChange(event.target.value)}
                  value={orderInstrument}
                />
                <datalist id="trade-symbols">
                  {knownSymbolIds.map((instId) => (
                    <option key={instId} value={instId} />
                  ))}
                </datalist>
              </label>
              <label>
                <span>{copy.paper.margin}</span>
                <input
                  min="1"
                  onChange={(event) => onMarginChange(event.target.value)}
                  step="1"
                  type="number"
                  value={orderMargin}
                />
              </label>
              <label>
                <span>{copy.paper.leverage}</span>
                <input
                  max="50"
                  min="1"
                  onChange={(event) => onLeverageChange(event.target.value)}
                  step="1"
                  type="number"
                  value={orderLeverage}
                />
              </label>
            </div>
            <div className="paper-actions">
              <button
                className="buy-button"
                disabled={tradeBusy || orderInstrument.trim().length === 0}
                onClick={() => onOpenPaper("long", orderInstrument)}
                type="button"
              >
                {copy.actions.openLong}
              </button>
              <button
                className="sell-button"
                disabled={tradeBusy || orderInstrument.trim().length === 0}
                onClick={() => onOpenPaper("short", orderInstrument)}
                type="button"
              >
                {copy.actions.openShort}
              </button>
            </div>
            {tradeError ? (
              <p className="paper-error">
                {copy.paper.orderError}: {tradeError}
              </p>
            ) : null}
          </section>
          <section className="detail-section">
            <h2>{copy.trade.selectedPosition}</h2>
            {selectedPosition ? (
              <>
                <dl className="paper-position">
                  <div>
                    <dt>{copy.paper.side}</dt>
                    <dd>{copy.directions[selectedPosition.side]}</dd>
                  </div>
                  <div>
                    <dt>{copy.paper.entry}</dt>
                    <dd>{formatPrice(selectedPosition.entry_price)}</dd>
                  </div>
                  <div>
                    <dt>{copy.paper.mark}</dt>
                    <dd>{formatPrice(selectedPosition.mark_price)}</dd>
                  </div>
                  <div>
                    <dt>{copy.paper.qty}</dt>
                    <dd>{formatQuantity(selectedPosition.qty)}</dd>
                  </div>
                  <div>
                    <dt>{copy.paper.notional}</dt>
                    <dd>{formatUsdt(selectedPosition.notional)}</dd>
                  </div>
                  <div>
                    <dt>{copy.paper.pnl}</dt>
                    <dd className={pnlClass(selectedPosition.unrealized_pnl)}>
                      {formatSignedUsdt(selectedPosition.unrealized_pnl)} /{" "}
                      {formatPct(selectedPosition.pnl_pct)}
                    </dd>
                  </div>
                  <div>
                    <dt>{copy.trade.stopLoss}</dt>
                    <dd>{formatOptionalPrice(selectedPosition.stop_loss)}</dd>
                  </div>
                  <div>
                    <dt>{copy.trade.takeProfit}</dt>
                    <dd>{formatOptionalPrice(selectedPosition.take_profit)}</dd>
                  </div>
                  <div>
                    <dt>{copy.trade.signal}</dt>
                    <dd>{selectedPosition.primary_signal || "—"}</dd>
                  </div>
                  <div>
                    <dt>{copy.trade.entryFee}</dt>
                    <dd>{formatUsdt(selectedPosition.fee ?? 0)}</dd>
                  </div>
                  <div className="paper-position-wide">
                    <dt>{copy.trade.reason}</dt>
                    <dd>{selectedPosition.reason || "—"}</dd>
                  </div>
                  <div>
                    <dt>{copy.detail.updated}</dt>
                    <dd>{formatTimestamp(selectedPosition.opened_at_ms)}</dd>
                  </div>
                </dl>
                <button
                  className="close-button"
                  disabled={tradeBusy}
                  onClick={() => onClosePaper(selectedPosition.inst_id)}
                  type="button"
                >
                  {copy.actions.closePosition}
                </button>
              </>
            ) : (
              <p className="muted">{copy.trade.noSelectedPosition}</p>
            )}
          </section>
          <section className="detail-section">
            <h2>{copy.trade.recentTrades}</h2>
            {paper.trades.length === 0 ? (
              <p className="muted">{copy.paper.noTrades}</p>
            ) : (
              <ul className="trade-list">
                {paper.trades.slice(0, 6).map((trade) => (
                  <li key={trade.id}>
                    <span>
                      {trade.inst_id} · {copy.paper.tradeActions[trade.action]}{" "}
                      {copy.directions[trade.side]} @ {formatPrice(trade.price)}
                    </span>
                    <strong className={pnlClass(trade.realized_pnl)}>
                      {formatSignedUsdt(trade.realized_pnl)}
                    </strong>
                  </li>
                ))}
              </ul>
            )}
          </section>
        </aside>
      </section>
      {reasonPosition ? (
        <ReasonDetailDialog
          copy={copy}
          onClose={() => setReasonPosition(null)}
          position={reasonPosition}
        />
      ) : null}
    </section>
  );
}

function ReasonDetailDialog({
  copy,
  onClose,
  position,
}: {
  copy: Copy;
  onClose: () => void;
  position: PaperPositionSnapshot;
}) {
  const title = `${position.inst_id} ${copy.trade.reasonDetails}`;
  const tags = position.tags ?? [];

  return (
    <div className="reason-modal-backdrop" onClick={onClose}>
      <section
        aria-label={title}
        aria-modal="true"
        className="reason-modal"
        onClick={(event) => event.stopPropagation()}
        role="dialog"
      >
        <header>
          <div>
            <h2>{title}</h2>
            <p>{position.strategy_version || position.source || "—"}</p>
          </div>
          <button aria-label={copy.trade.closeReasonDetails} onClick={onClose} type="button">
            ×
          </button>
        </header>
        <div className="reason-modal-body">
          <dl className="reason-detail-grid">
            <div>
              <dt>{copy.trade.signal}</dt>
              <dd>{position.primary_signal || "—"}</dd>
            </div>
            <div className="reason-detail-wide">
              <dt>{copy.trade.decisionReason}</dt>
              <dd>{position.reason || "—"}</dd>
            </div>
          </dl>
          <section className="reason-tag-section">
            <h3>{copy.trade.decisionTags}</h3>
            {(position.signal_tags?.length ?? 0) === 0 && tags.length === 0 ? (
              <p className="muted">{copy.trade.noDecisionTags}</p>
            ) : (
              <ul>
                {(position.signal_tags ?? []).map((tag) => (
                  <li key={`signal-${tag}`}>
                    <strong>{tag}</strong>
                  </li>
                ))}
                {tags.map((tag, index) => (
                  <li key={`${tag.kind}-${tag.label}-${tag.ts_ms}-${index}`}>
                    <div>
                      <strong>{tag.label}</strong>
                      <span>
                        {copy.trade.scoreImpact}: {tag.score_impact >= 0 ? "+" : ""}
                        {tag.score_impact}
                      </span>
                    </div>
                    <p>{tag.reason}</p>
                  </li>
                ))}
              </ul>
            )}
          </section>
        </div>
      </section>
    </div>
  );
}

function formatOptionalPrice(value: number | null | undefined) {
  return value == null ? "—" : formatPrice(value);
}

function shortHash(value: string) {
  return value.length > 16 ? `${value.slice(0, 12)}…` : value || "—";
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
