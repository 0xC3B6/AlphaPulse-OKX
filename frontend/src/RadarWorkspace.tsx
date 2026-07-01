import type { Copy } from "./i18n";
import { RadarTable } from "./RadarTable";
import { SymbolDetailPanel } from "./SymbolDetailPanel";
import type { PaperAccountSnapshot, PaperSide, SymbolSnapshot } from "./types";
import type { Filter, ThemeMode } from "./uiFormat";

export function RadarWorkspace({
  copy,
  filter,
  filteredSymbols,
  onClosePaper,
  onFilterChange,
  onLeverageChange,
  onMarginChange,
  onOpenPaper,
  onOpenTradingView,
  onSelectSymbol,
  orderLeverage,
  orderMargin,
  paper,
  selected,
  themeMode,
  tradeBusy,
  tradeError,
}: {
  copy: Copy;
  filter: Filter;
  filteredSymbols: SymbolSnapshot[];
  onClosePaper: () => void;
  onFilterChange: (filter: Filter) => void;
  onLeverageChange: (value: string) => void;
  onMarginChange: (value: string) => void;
  onOpenPaper: (side: PaperSide) => void;
  onOpenTradingView: (symbol: SymbolSnapshot) => void;
  onSelectSymbol: (symbolId: string) => void;
  orderLeverage: string;
  orderMargin: string;
  paper: PaperAccountSnapshot;
  selected: SymbolSnapshot | null;
  themeMode: ThemeMode;
  tradeBusy: boolean;
  tradeError: string | null;
}) {
  return (
    <>
      <section className="toolbar radar-filterbar" aria-label={copy.aria.radarControls}>
        <div className="toolbar-group" role="group" aria-label={copy.aria.opportunityFilters}>
          {[
            ["all", copy.filters.all],
            ["trend", copy.filters.trend],
            ["range", copy.filters.range],
            ["hot", copy.filters.hot],
            ["fixed", copy.filters.fixed],
          ].map(([value, label]) => (
            <button
              className={filter === value ? "active" : ""}
              key={value}
              onClick={() => onFilterChange(value as Filter)}
              type="button"
            >
              {label}
            </button>
          ))}
        </div>
      </section>

      {filteredSymbols.length === 0 ? (
        <section className="empty-state">
          <h2>{copy.empty.title}</h2>
          <p>{copy.empty.body}</p>
        </section>
      ) : (
        <section className="radar-grid radar-workspace">
          <RadarTable
            copy={copy}
            onOpenTradingView={onOpenTradingView}
            onSelectSymbol={onSelectSymbol}
            selectedId={selected?.inst_id ?? null}
            symbols={filteredSymbols}
          />
          <aside className="detail-panel">
            {selected ? (
              <SymbolDetailPanel
                copy={copy}
                onClosePaper={onClosePaper}
                onLeverageChange={onLeverageChange}
                onMarginChange={onMarginChange}
                onOpenPaper={onOpenPaper}
                onOpenTradingView={onOpenTradingView}
                orderLeverage={orderLeverage}
                orderMargin={orderMargin}
                paper={paper}
                symbol={selected}
                themeMode={themeMode}
                tradeBusy={tradeBusy}
                tradeError={tradeError}
              />
            ) : null}
          </aside>
        </section>
      )}
    </>
  );
}
