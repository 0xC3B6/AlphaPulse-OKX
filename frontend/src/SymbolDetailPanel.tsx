import { ChartPanel } from "./ChartPanel";
import type { Copy } from "./i18n";
import type { PaperAccountSnapshot, PaperSide, SymbolSnapshot } from "./types";
import {
  formatPct,
  formatPrice,
  formatQuantity,
  formatSignedUsdt,
  formatTemplate,
  formatTimestamp,
  formatUsdt,
  pnlClass,
} from "./uiFormat";
import type { ThemeMode } from "./uiFormat";

export function SymbolDetailPanel({
  copy,
  onClosePaper,
  onLeverageChange,
  onMarginChange,
  onOpenPaper,
  onOpenTradingView,
  orderLeverage,
  orderMargin,
  paper,
  symbol,
  themeMode,
  tradeBusy,
  tradeError,
}: {
  copy: Copy;
  onClosePaper: () => void;
  onLeverageChange: (value: string) => void;
  onMarginChange: (value: string) => void;
  onOpenPaper: (side: PaperSide) => void;
  onOpenTradingView: (symbol: SymbolSnapshot) => void;
  orderLeverage: string;
  orderMargin: string;
  paper: PaperAccountSnapshot;
  symbol: SymbolSnapshot;
  themeMode: ThemeMode;
  tradeBusy: boolean;
  tradeError: string | null;
}) {
  const position = paper.positions.find((item) => item.inst_id === symbol.inst_id);
  const trades = paper.trades
    .filter((trade) => trade.inst_id === symbol.inst_id)
    .slice(0, 5);

  return (
    <section className="symbol-detail-panel" data-testid="symbol-detail-panel">
      <header className="detail-header">
        <div>
          <h2>{symbol.inst_id}</h2>
          <p>{symbol.trigger_reason || copy.detail.noActiveTrigger}</p>
        </div>
        <button
          aria-label={formatTemplate(copy.actions.openTradingViewChart, symbol.inst_id)}
          className="detail-tv-button"
          onClick={() => onOpenTradingView(symbol)}
          type="button"
        >
          {copy.actions.openTradingView}
        </button>
      </header>
      <section className="detail-section detail-section-market">
        <h3>{copy.detail.market}</h3>
        <dl className="detail-metric-strip">
          <div>
            <dt>{copy.table.price}</dt>
            <dd>{formatPrice(symbol.price)}</dd>
          </div>
          <div>
            <dt>5m</dt>
            <dd className={pnlClass(symbol.change_5m_pct)}>{formatPct(symbol.change_5m_pct)}</dd>
          </div>
          <div>
            <dt>15m</dt>
            <dd className={pnlClass(symbol.change_15m_pct)}>{formatPct(symbol.change_15m_pct)}</dd>
          </div>
          <div>
            <dt>1h</dt>
            <dd className={pnlClass(symbol.change_1h_pct)}>{formatPct(symbol.change_1h_pct)}</dd>
          </div>
        </dl>
      </section>
      <ChartPanel copy={copy} symbol={symbol} themeMode={themeMode} />
      <section className="detail-section detail-section-structure">
        <h3>{copy.detail.structure}</h3>
        <dl className="detail-list">
          <div>
            <dt>{copy.detail.funding}</dt>
            <dd>{symbol.funding_rate === null ? "-" : formatPct(symbol.funding_rate)}</dd>
          </div>
          <div>
            <dt>{copy.detail.updated}</dt>
            <dd>{formatTimestamp(symbol.updated_at_ms)}</dd>
          </div>
        </dl>
        <section>
          <h3>{copy.detail.fvg}</h3>
          {symbol.fvgs.length === 0 ? (
            <p className="muted">{copy.detail.noFvgZones}</p>
          ) : (
            <ul>
              {symbol.fvgs.map((zone, index) => (
                <li key={`${zone.timeframe}-${zone.direction}-${index}`}>
                  {zone.timeframe} {copy.directions[zone.direction]}{" "}
                  {formatPrice(zone.lower)}-{formatPrice(zone.upper)} {copy.detail.distance}{" "}
                  {formatPct(zone.distance_pct)}
                </li>
              ))}
            </ul>
          )}
        </section>
        <section>
          <h3>{copy.detail.levels}</h3>
          {symbol.levels.length === 0 ? (
            <p className="muted">{copy.detail.noLevels}</p>
          ) : (
            <ul>
              {symbol.levels.map((level, index) => (
                <li key={`${level.kind}-${index}`}>
                  {copy.levelKinds[level.kind]} {formatPrice(level.lower)}-
                  {formatPrice(level.upper)} {copy.detail.touches} {level.touches}
                </li>
              ))}
            </ul>
          )}
        </section>
      </section>
      <section className="detail-section detail-section-paper">
        <h3>{copy.detail.paperTrading}</h3>
        <dl className="paper-metrics">
          <div>
            <dt>{copy.paper.equity}</dt>
            <dd>{formatUsdt(paper.equity)}</dd>
          </div>
          <div>
            <dt>{copy.paper.available}</dt>
            <dd>{formatUsdt(paper.available_balance)}</dd>
          </div>
          <div>
            <dt>{copy.paper.usedMargin}</dt>
            <dd>{formatUsdt(paper.used_margin)}</dd>
          </div>
          <div>
            <dt>{copy.paper.unrealized}</dt>
            <dd className={pnlClass(paper.unrealized_pnl)}>
              {formatSignedUsdt(paper.unrealized_pnl)}
            </dd>
          </div>
        </dl>
        <div className="paper-order">
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
            disabled={tradeBusy}
            onClick={() => onOpenPaper("long")}
            type="button"
          >
            {copy.actions.openLong}
          </button>
          <button
            className="sell-button"
            disabled={tradeBusy}
            onClick={() => onOpenPaper("short")}
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
        <section className="paper-subsection">
          <h3>{copy.paper.position}</h3>
          {position ? (
            <>
              <dl className="paper-position">
                <div>
                  <dt>{copy.paper.side}</dt>
                  <dd>{copy.directions[position.side]}</dd>
                </div>
                <div>
                  <dt>{copy.paper.entry}</dt>
                  <dd>{formatPrice(position.entry_price)}</dd>
                </div>
                <div>
                  <dt>{copy.paper.mark}</dt>
                  <dd>{formatPrice(position.mark_price)}</dd>
                </div>
                <div>
                  <dt>{copy.paper.qty}</dt>
                  <dd>{formatQuantity(position.qty)}</dd>
                </div>
                <div>
                  <dt>{copy.paper.notional}</dt>
                  <dd>{formatUsdt(position.notional)}</dd>
                </div>
                <div>
                  <dt>{copy.paper.pnl}</dt>
                  <dd className={pnlClass(position.unrealized_pnl)}>
                    {formatSignedUsdt(position.unrealized_pnl)} / {formatPct(position.pnl_pct)}
                  </dd>
                </div>
              </dl>
              <button
                className="close-button"
                disabled={tradeBusy}
                onClick={onClosePaper}
                type="button"
              >
                {copy.actions.closePosition}
              </button>
            </>
          ) : (
            <p className="muted">{copy.paper.noPosition}</p>
          )}
        </section>
        <section className="paper-subsection">
          <h3>{copy.paper.history}</h3>
          {trades.length === 0 ? (
            <p className="muted">{copy.paper.noTrades}</p>
          ) : (
            <ul className="trade-list">
              {trades.map((trade) => (
                <li key={trade.id}>
                  <span>
                    {copy.paper.tradeActions[trade.action]} {copy.directions[trade.side]} @{" "}
                    {formatPrice(trade.price)}
                  </span>
                  <strong className={pnlClass(trade.realized_pnl)}>
                    {formatSignedUsdt(trade.realized_pnl)}
                  </strong>
                </li>
              ))}
            </ul>
          )}
        </section>
      </section>
    </section>
  );
}
