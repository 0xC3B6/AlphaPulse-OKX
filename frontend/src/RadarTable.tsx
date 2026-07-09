import { toTerminalSymbol, type TerminalSignal, type TerminalTimeframeDirection } from "./figmaTerminal";
import type { Copy } from "./i18n";
import type { PaperAccountSnapshot, SymbolSnapshot } from "./types";
import {
  formatPct,
  formatPrice,
  formatTags,
  formatTemplate,
} from "./uiFormat";

export function RadarTable({
  copy,
  onOpenTradingView,
  onSelectSymbol,
  paper,
  selectedId,
  symbols,
}: {
  copy: Copy;
  onOpenTradingView: (symbol: SymbolSnapshot) => void;
  onSelectSymbol: (symbolId: string | null) => void;
  paper: PaperAccountSnapshot;
  selectedId: string | null;
  symbols: SymbolSnapshot[];
}) {
  const rows = symbols.map((symbol) => toTerminalSymbol(symbol, paper));

  return (
    <div className="table-panel radar-table-panel" data-testid="radar-terminal-table">
      <table className="radar-table">
        <thead>
          <tr>
            <th>SYMBOL<span className="sr-only">{copy.table.symbol}</span></th>
            <th>PRICE<span className="sr-only">{copy.table.price}</span></th>
            <th>Chg%</th>
            <th>5m</th>
            <th>15m</th>
            <th>1h</th>
            <th>TREND<span className="sr-only">{copy.table.trend}</span></th>
            <th>RANGE<span className="sr-only">{copy.table.range}</span></th>
            <th>SIGNAL<span className="sr-only">{copy.table.signal}</span></th>
            <th>Tags</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => {
            const symbol = row.source;
            return (
              <tr
                className={symbol.inst_id === selectedId ? "selected" : ""}
                key={symbol.inst_id}
                onClick={() => onSelectSymbol(symbol.inst_id === selectedId ? null : symbol.inst_id)}
              >
                <td>
                  <div className="symbol-cell">
                    <div className="symbol-cell-main">
                      <strong>
                        {row.hasPosition ? <i className="position-dot" aria-label="open position" /> : null}
                        <span>{row.base}</span>
                        <small>/USDT</small>
                        <span className="sr-only">{row.id}</span>
                      </strong>
                      <span className="sr-only">{formatTags(symbol.pool_tags, copy)}</span>
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
                <td className={row.chg < 0 ? "negative" : "positive"}>{formatPct(row.chg)}</td>
                <td>
                  <TimeframeBadge value={row.m5} />
                </td>
                <td>
                  <TimeframeBadge value={row.m15} />
                </td>
                <td>
                  <TimeframeBadge value={row.h1} />
                </td>
                <td>
                  <ScoreBar value={row.trend} tone="trend" />
                </td>
                <td>
                  <ScoreBar value={row.range} tone="range" />
                </td>
                <td>
                  <div className="signal-cell">
                    <SignalBadge value={row.signal} />
                    <span className="sr-only">{row.triggerReason || copy.misc.watching}</span>
                  </div>
                </td>
                <td>
                  <div className="terminal-tag-list">
                    {row.tags.map((tag) => (
                      <span className="terminal-tag-pill" key={tag}>
                        {tag}
                      </span>
                    ))}
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

function TimeframeBadge({ value }: { value: TerminalTimeframeDirection }) {
  if (value === "UP") {
    return <span className="terminal-tf-badge positive">▲UP</span>;
  }
  if (value === "DOWN") {
    return <span className="terminal-tf-badge negative">▼DN</span>;
  }
  return <span className="terminal-tf-badge">─FL</span>;
}

function SignalBadge({ value }: { value: TerminalSignal }) {
  const tone = value === "LONG" ? "positive" : value === "SHORT" ? "negative" : "";
  return <span className={`signal-pill ${tone}`}>{value}</span>;
}

function ScoreBar({ tone, value }: { tone: "range" | "trend"; value: number }) {
  return (
    <div className="terminal-score-bar">
      <span className={`terminal-score-track ${tone}`}>
        <i style={{ width: `${Math.max(0, Math.min(100, value))}%` }} />
      </span>
      <b>{value}</b>
    </div>
  );
}
