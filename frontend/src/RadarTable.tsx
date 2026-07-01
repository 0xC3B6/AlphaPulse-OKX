import type { Copy } from "./i18n";
import type { SymbolSnapshot } from "./types";
import {
  formatPct,
  formatPrice,
  formatSignalDirection,
  formatTags,
  formatTemplate,
  primaryScore,
  scoreTone,
} from "./uiFormat";

export function RadarTable({
  copy,
  onOpenTradingView,
  onSelectSymbol,
  selectedId,
  symbols,
}: {
  copy: Copy;
  onOpenTradingView: (symbol: SymbolSnapshot) => void;
  onSelectSymbol: (symbolId: string) => void;
  selectedId: string | null;
  symbols: SymbolSnapshot[];
}) {
  return (
    <div className="table-panel radar-table-panel">
      <table className="radar-table">
        <thead>
          <tr>
            <th>{copy.table.symbol}</th>
            <th>{copy.table.price}</th>
            <th>5m</th>
            <th>15m</th>
            <th>1h</th>
            <th>{copy.table.trend}</th>
            <th>{copy.table.range}</th>
            <th>{copy.table.signal}</th>
          </tr>
        </thead>
        <tbody>
          {symbols.map((symbol) => {
            const signal = primaryScore(symbol);
            const signalTone = scoreTone(signal);
            return (
              <tr
                className={symbol.inst_id === selectedId ? "selected" : ""}
                key={symbol.inst_id}
                onClick={() => onSelectSymbol(symbol.inst_id)}
              >
                <td>
                  <div className="symbol-cell">
                    <div className="symbol-cell-main">
                      <strong>{symbol.inst_id}</strong>
                      <span>{formatTags(symbol.pool_tags, copy)}</span>
                    </div>
                    <button
                      aria-label={formatTemplate(copy.actions.openTradingViewChart, symbol.inst_id)}
                      className="symbol-tv-button"
                      onClick={(event) => {
                        event.stopPropagation();
                        onOpenTradingView(symbol);
                      }}
                      title={copy.actions.openTradingView}
                      type="button"
                    >
                      TV
                    </button>
                  </div>
                </td>
                <td>{formatPrice(symbol.price)}</td>
                <td className={symbol.change_5m_pct < 0 ? "negative" : "positive"}>
                  {formatPct(symbol.change_5m_pct)}
                </td>
                <td className={symbol.change_15m_pct < 0 ? "negative" : "positive"}>
                  {formatPct(symbol.change_15m_pct)}
                </td>
                <td className={symbol.change_1h_pct < 0 ? "negative" : "positive"}>
                  {formatPct(symbol.change_1h_pct)}
                </td>
                <td>
                  <span className={`score-badge ${scoreTone(symbol.trend_score)}`}>
                    {symbol.trend_score.value}
                  </span>
                </td>
                <td>
                  <span className={`score-badge ${scoreTone(symbol.range_score)}`}>
                    {symbol.range_score.value}
                  </span>
                </td>
                <td>
                  <div className="signal-cell">
                    <span className={`signal-pill ${signalTone}`}>
                      {formatSignalDirection(signal.direction)}
                    </span>
                    <span>{symbol.trigger_reason || copy.misc.watching}</span>
                  </div>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
