import { ChartPanel } from "./ChartPanel";
import type { Copy } from "./i18n";
import type { SymbolSnapshot } from "./types";
import {
  formatPct,
  formatPrice,
  formatTemplate,
  formatTimestamp,
  pnlClass,
} from "./uiFormat";
import type { ThemeMode } from "./uiFormat";

export function SymbolDetailPanel({
  copy,
  onOpenTradingView,
  onTradeSymbol,
  symbol,
  themeMode,
}: {
  copy: Copy;
  onOpenTradingView: (symbol: SymbolSnapshot) => void;
  onTradeSymbol: (symbol: SymbolSnapshot) => void;
  symbol: SymbolSnapshot;
  themeMode: ThemeMode;
}) {
  return (
    <section className="symbol-detail-panel" data-testid="symbol-detail-panel">
      <header className="detail-header">
        <div>
          <h2>{symbol.inst_id}</h2>
          <p>{symbol.trigger_reason || copy.detail.noActiveTrigger}</p>
        </div>
        <div className="detail-header-actions">
          <button
            aria-label={formatTemplate(copy.actions.openTradingViewChart, symbol.inst_id)}
            className="detail-tv-button"
            onClick={() => onOpenTradingView(symbol)}
            type="button"
          >
            {copy.actions.openTradingView}
          </button>
          <button
            className="detail-trade-button"
            onClick={() => onTradeSymbol(symbol)}
            type="button"
          >
            {copy.actions.goTrade}
          </button>
        </div>
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
    </section>
  );
}
