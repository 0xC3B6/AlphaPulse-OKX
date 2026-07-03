import type { Copy } from "./i18n";
import { MacroSummaryStrip } from "./MacroSummaryStrip";
import { RadarTable } from "./RadarTable";
import { SymbolDetailPanel } from "./SymbolDetailPanel";
import type { BtcMacroSnapshot, SymbolSnapshot } from "./types";
import type { Filter, ThemeMode } from "./uiFormat";

export function MonitorPage({
  copy,
  filter,
  filteredSymbols,
  macroError,
  macroLoading,
  macroSnapshot,
  onFilterChange,
  onOpenTradingView,
  onSelectSymbol,
  onTradeSymbol,
  selected,
  themeMode,
}: {
  copy: Copy;
  filter: Filter;
  filteredSymbols: SymbolSnapshot[];
  macroError: string | null;
  macroLoading: boolean;
  macroSnapshot: BtcMacroSnapshot | null;
  onFilterChange: (filter: Filter) => void;
  onOpenTradingView: (symbol: SymbolSnapshot) => void;
  onSelectSymbol: (symbolId: string) => void;
  onTradeSymbol: (symbol: SymbolSnapshot) => void;
  selected: SymbolSnapshot | null;
  themeMode: ThemeMode;
}) {
  return (
    <>
      <MacroSummaryStrip
        copy={copy}
        error={macroError}
        loading={macroLoading}
        snapshot={macroSnapshot}
      />
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
        <section className="radar-grid radar-workspace monitor-page">
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
                onOpenTradingView={onOpenTradingView}
                onTradeSymbol={onTradeSymbol}
                symbol={selected}
                themeMode={themeMode}
              />
            ) : null}
          </aside>
        </section>
      )}
    </>
  );
}
