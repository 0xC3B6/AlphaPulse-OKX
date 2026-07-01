# Frontend Console Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert the existing AlphaPulse OKX frontend into the approved componentized trading-console layout using the Radar reference color style.

**Architecture:** Keep `App` as the state owner, then extract presentational and workflow components around clear UI boundaries. Shared formatting and filtering helpers move into a small utility module so `App`, Radar components, Macro summary, and Macro page do not copy logic.

**Tech Stack:** React 18, TypeScript, Vite, Vitest, Testing Library, CSS variables, existing `lightweight-charts` chart components.

---

## Scope Check

This plan covers one subsystem: the React frontend. It does not require backend API changes, new trading logic, a new charting package, or a new component library.

## File Structure

- Create `frontend/src/uiFormat.ts`: shared filters, score selection, price/percentage/date/state formatting, regime formatting, class helpers.
- Create `frontend/src/uiFormat.test.ts`: focused unit tests for shared formatting helpers.
- Create `frontend/src/ConsoleShell.tsx`: top-level console frame, status pills, view switcher, theme/language/notification controls, content slot.
- Create `frontend/src/MacroSummaryStrip.tsx`: compact Radar-page macro summary using the macro snapshot already loaded by `App`.
- Create `frontend/src/RadarWorkspace.tsx`: Radar view layout, filter group, empty state, table/detail composition.
- Create `frontend/src/RadarTable.tsx`: dense symbol table, selected row, score cells, direction label, TV button propagation behavior.
- Create `frontend/src/SymbolDetailPanel.tsx`: selected-symbol detail panel, grouped market/structure/paper sections, existing `ChartPanel` composition.
- Create `frontend/src/TradingViewModal.tsx`: current TradingView modal moved out of `App`.
- Modify `frontend/src/App.tsx`: keep application state and data flow, wire extracted components, remove moved render helpers.
- Modify `frontend/src/MacroPanel.tsx`: restyle the first viewport into the approved card hierarchy and import shared formatters where useful.
- Modify `frontend/src/i18n.ts`: add labels used by macro summary and detail groups in both languages.
- Modify `frontend/src/styles.css`: apply the Radar palette, console layout, table/detail styling, Macro first-viewport styling, and responsive rules.
- Modify `frontend/src/App.test.tsx`: add integration tests for macro summary, row selection, TV behavior, and preserved Macro behavior.
- Modify `frontend/src/styles.test.ts`: assert key Radar palette variables and responsive rules remain present.

### Task 1: Add Failing Integration Tests

**Files:**
- Modify: `frontend/src/App.test.tsx`

- [ ] **Step 1: Add tests for the approved console behavior**

Append these tests inside the existing `describe("App", () => { })` block, after `prefetches macro data before the macro view is opened`:

```tsx
  it("shows a compact macro summary on the radar console", async () => {
    mockSnapshot();

    render(<App />);

    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);
    const summary = await screen.findByTestId("macro-summary-strip");

    expect(summary).toHaveTextContent("BTC 大周期摘要");
    expect(summary).toHaveTextContent("熊市反弹");
    expect(summary).toHaveTextContent("$60,000.00");
    expect(summary).toHaveTextContent("80/100");
    expect(summary).toHaveTextContent("-40.00%");
    expect(summary).toHaveTextContent("55.00%");
  });

  it("keeps the radar workspace usable when the macro summary request fails", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL) => {
        if (String(input).includes("/macro/btc")) {
          return {
            ok: false,
            json: async () => ({ message: "macro unavailable" }),
          };
        }
        if (String(input).includes("/chart")) {
          return {
            ok: true,
            json: async () => chart,
          };
        }
        return {
          ok: true,
          json: async () => snapshot,
        };
      }),
    );

    render(<App />);

    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);
    const summary = await screen.findByTestId("macro-summary-strip");
    expect(summary).toHaveTextContent("大周期数据不可用");
    expect(screen.getByRole("button", { name: "趋势" })).toBeInTheDocument();
  });

  it("selects symbols from the dense radar table", async () => {
    mockSnapshot();

    render(<App />);
    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("row", { name: /DOGE-USDT-SWAP/ }));

    const detail = screen.getByTestId("symbol-detail-panel");
    expect(detail).toHaveTextContent("DOGE-USDT-SWAP");
    expect(detail).toHaveTextContent("range long 82: near support");
  });

  it("opens TradingView from a table button without changing the selected detail symbol", async () => {
    mockSnapshot();

    render(<App />);
    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("row", { name: /DOGE-USDT-SWAP/ }));
    expect(screen.getByTestId("symbol-detail-panel")).toHaveTextContent("DOGE-USDT-SWAP");

    fireEvent.click(screen.getAllByRole("button", { name: "打开 LAB-USDT-SWAP TradingView 图表" })[0]);

    expect(screen.getByRole("dialog", { name: "LAB-USDT-SWAP TradingView" })).toBeInTheDocument();
    expect(screen.getByTestId("symbol-detail-panel")).toHaveTextContent("DOGE-USDT-SWAP");
  });
```

- [ ] **Step 2: Run the focused test file and verify the new tests fail**

Run:

```bash
cd frontend
npm test -- App.test.tsx
```

Expected: FAIL. The output should mention missing `macro-summary-strip`, missing `symbol-detail-panel`, or missing new copy text such as `BTC 大周期摘要`.

- [ ] **Step 3: Commit the failing tests**

```bash
git add frontend/src/App.test.tsx
git commit -m "test: cover frontend console redesign behavior"
```

### Task 2: Extract Shared UI Format Helpers

**Files:**
- Create: `frontend/src/uiFormat.ts`
- Create: `frontend/src/uiFormat.test.ts`
- Modify: `frontend/src/App.tsx`
- Modify: `frontend/src/MacroPanel.tsx`

- [ ] **Step 1: Write unit tests for shared helpers**

Create `frontend/src/uiFormat.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { translations } from "./i18n";
import {
  formatPct,
  formatPrice,
  formatRegime,
  formatSignalDirection,
  formatTags,
  maxScore,
  primaryScore,
} from "./uiFormat";
import type { SymbolSnapshot } from "./types";

const copy = translations.zh;

const symbol: SymbolSnapshot = {
  inst_id: "ETH-USDT-SWAP",
  price: 1585.292,
  change_5m_pct: 0.0013,
  change_15m_pct: 0.0022,
  change_1h_pct: 0.0118,
  trend_score: { value: 82, direction: "long", reasons: ["trend"] },
  range_score: { value: 90, direction: "long", reasons: ["range"] },
  pool_tags: ["fixed", "manual_watch"],
  trigger_reason: "range long 90: clear recent range",
  funding_rate: null,
  fvgs: [],
  levels: [],
  updated_at_ms: 1782400000000,
};

describe("uiFormat", () => {
  it("formats table values and tags", () => {
    expect(formatPrice(symbol.price)).toBe("1,585.29");
    expect(formatPct(symbol.change_15m_pct)).toBe("0.22%");
    expect(formatTags(symbol.pool_tags, copy)).toBe("固定 / 手动关注");
  });

  it("selects the primary score for a symbol", () => {
    expect(maxScore(symbol)).toBe(90);
    expect(primaryScore(symbol)).toEqual(symbol.range_score);
    expect(formatSignalDirection(symbol.range_score.direction)).toBe("LONG");
  });

  it("formats macro regimes through translations", () => {
    expect(formatRegime("bear_market_rally", copy)).toBe("熊市反弹");
  });
});
```

- [ ] **Step 2: Run the new unit test and verify it fails**

Run:

```bash
cd frontend
npm test -- uiFormat.test.ts
```

Expected: FAIL with an import error because `frontend/src/uiFormat.ts` does not exist.

- [ ] **Step 3: Create the shared helper module**

Create `frontend/src/uiFormat.ts`:

```ts
import type { Copy } from "./i18n";
import type { MacroRegime, Score, SymbolSnapshot } from "./types";

export type Filter = "all" | "trend" | "range" | "hot" | "fixed";
export type ThemeMode = "light" | "dark" | "system";
export type ViewMode = "radar" | "macro";

export function matchesFilter(symbol: SymbolSnapshot, filter: Filter): boolean {
  if (filter === "trend") {
    return symbol.trend_score.value >= 65;
  }
  if (filter === "range") {
    return symbol.range_score.value >= 65;
  }
  if (filter === "hot") {
    return symbol.pool_tags.includes("dynamic");
  }
  if (filter === "fixed") {
    return symbol.pool_tags.includes("fixed");
  }
  return true;
}

export function maxScore(symbol: SymbolSnapshot): number {
  return Math.max(symbol.trend_score.value, symbol.range_score.value);
}

export function primaryScore(symbol: SymbolSnapshot): Score {
  return symbol.trend_score.value >= symbol.range_score.value
    ? symbol.trend_score
    : symbol.range_score;
}

export function scoreTone(score: Score): "positive" | "negative" | "" {
  if (score.value < 80) {
    return "";
  }
  return score.direction === "short" ? "negative" : "positive";
}

export function formatScore(score: Score, copy: Copy): string {
  return `${score.value} ${copy.directions[score.direction]}`;
}

export function formatSignalDirection(direction: Score["direction"]): string {
  return direction.toUpperCase();
}

export function formatTags(tags: string[], copy: Copy): string {
  if (tags.length === 0) {
    return copy.misc.unlabeled;
  }
  return tags.map((tag) => formatTag(tag, copy)).join(" / ");
}

export function formatTag(tag: string, copy: Copy): string {
  const labels = copy.poolTags as unknown as Record<string, string>;
  return labels[tag] ?? tag;
}

export function formatState(value: string, copy: Copy): string {
  return copy.states[value as keyof Copy["states"]] ?? value;
}

export function formatPrice(value: number): string {
  if (value >= 100) {
    return value.toLocaleString(undefined, {
      maximumFractionDigits: 2,
      minimumFractionDigits: 2,
    });
  }
  if (value >= 1) {
    return value.toFixed(4);
  }
  return value.toFixed(6);
}

export function formatQuantity(value: number): string {
  if (value >= 100) {
    return value.toFixed(2);
  }
  if (value >= 1) {
    return value.toFixed(4);
  }
  return value.toFixed(6);
}

export function formatUsdt(value: number): string {
  return `${value.toLocaleString(undefined, {
    maximumFractionDigits: 2,
    minimumFractionDigits: 2,
  })} USDT`;
}

export function formatUsd(value: number): string {
  return `$${value.toLocaleString(undefined, {
    maximumFractionDigits: 2,
    minimumFractionDigits: 2,
  })}`;
}

export function formatSignedUsdt(value: number): string {
  const prefix = value > 0 ? "+" : "";
  return `${prefix}${formatUsdt(value)}`;
}

export function formatPct(value: number): string {
  return `${(value * 100).toFixed(2)}%`;
}

export function pnlClass(value: number): string {
  if (value > 0) {
    return "positive";
  }
  if (value < 0) {
    return "negative";
  }
  return "";
}

export function formatTemplate(template: string, symbol: string): string {
  return template.replace("{symbol}", symbol);
}

export function formatTimestamp(value: number | null): string {
  if (value === null) {
    return "-";
  }
  return new Date(value).toLocaleTimeString();
}

export function formatDate(value: number): string {
  return new Date(value).toLocaleDateString();
}

export function formatRegime(regime: MacroRegime, copy: Copy): string {
  return copy.macro.regimes[regime] ?? formatSnake(regime);
}

export function formatSnake(value: string): string {
  return value.replace(/_/g, " ");
}
```

- [ ] **Step 4: Import helpers into `App.tsx` and delete duplicate helper functions**

In `frontend/src/App.tsx`, replace local `Filter`, `ThemeMode`, and `ViewMode` type declarations with imports:

```ts
import {
  formatPct,
  formatPrice,
  formatQuantity,
  formatScore,
  formatSignedUsdt,
  formatState,
  formatTags,
  formatTemplate,
  formatTimestamp,
  formatUsdt,
  matchesFilter,
  maxScore,
  pnlClass,
  type Filter,
  type ThemeMode,
  type ViewMode,
} from "./uiFormat";
```

Delete these local functions from the bottom of `App.tsx` after imports are in place:

```ts
matchesFilter;
maxScore;
formatScore;
formatTags;
formatTag;
formatState;
formatPrice;
formatQuantity;
formatUsdt;
formatSignedUsdt;
formatPct;
pnlClass;
formatTemplate;
formatTimestamp;
```

The deletion is textual: remove the complete function declarations with those names, not just the identifiers shown above.

- [ ] **Step 5: Import shared macro formatters into `MacroPanel.tsx`**

At the top of `frontend/src/MacroPanel.tsx`, add:

```ts
import {
  formatDate,
  formatPct,
  formatRegime,
  formatSnake,
  formatUsd,
} from "./uiFormat";
```

Delete the duplicate local `formatRegime`, `formatSnake`, `formatUsd`, `formatPct`, and `formatDate` functions from the bottom of `MacroPanel.tsx`.

- [ ] **Step 6: Run tests for helper extraction**

Run:

```bash
cd frontend
npm test -- uiFormat.test.ts App.test.tsx
```

Expected: `uiFormat.test.ts` passes. `App.test.tsx` still fails on the new console behavior from Task 1.

- [ ] **Step 7: Commit helper extraction**

```bash
git add frontend/src/uiFormat.ts frontend/src/uiFormat.test.ts frontend/src/App.tsx frontend/src/MacroPanel.tsx
git commit -m "refactor: extract frontend ui format helpers"
```

### Task 3: Add Copy Keys and Console Shell

**Files:**
- Modify: `frontend/src/i18n.ts`
- Create: `frontend/src/ConsoleShell.tsx`
- Modify: `frontend/src/App.tsx`

- [ ] **Step 1: Add copy keys for the console redesign**

In both `zh` and `en` translation objects in `frontend/src/i18n.ts`, add these keys.

For `zh.detail`:

```ts
      market: "市场",
      structure: "结构",
      paperTrading: "模拟盘",
      recentSignals: "近期信号",
      noRecentSignals: "暂无近期信号",
```

For `zh.macro`:

```ts
      summaryLabel: "BTC 大周期摘要",
      summaryUnavailable: "大周期数据不可用",
      summaryLoading: "加载大周期摘要中",
```

For `en.detail`:

```ts
      market: "Market",
      structure: "Structure",
      paperTrading: "Paper trading",
      recentSignals: "Recent signals",
      noRecentSignals: "No recent signals",
```

For `en.macro`:

```ts
      summaryLabel: "BTC macro summary",
      summaryUnavailable: "Macro data unavailable",
      summaryLoading: "Loading macro summary",
```

- [ ] **Step 2: Create `ConsoleShell.tsx`**

Create `frontend/src/ConsoleShell.tsx`:

```tsx
import type { ReactNode } from "react";
import type { Copy, Language } from "./i18n";
import {
  formatState,
  formatTimestamp,
  type ThemeMode,
  type ViewMode,
} from "./uiFormat";

export function ConsoleShell({
  backendState,
  children,
  copy,
  language,
  lastScanAt,
  notificationPermission,
  onLanguageChange,
  onRequestNotifications,
  onThemeModeChange,
  onViewModeChange,
  streamState,
  symbolCount,
  themeMode,
  viewMode,
}: {
  backendState: "connected" | "disconnected";
  children: ReactNode;
  copy: Copy;
  language: Language;
  lastScanAt: number | null;
  notificationPermission: string;
  onLanguageChange: (language: Language) => void;
  onRequestNotifications: () => void;
  onThemeModeChange: (themeMode: ThemeMode) => void;
  onViewModeChange: (viewMode: ViewMode) => void;
  streamState: "connected" | "idle";
  symbolCount: number;
  themeMode: ThemeMode;
  viewMode: ViewMode;
}) {
  const statusItems = [
    { label: copy.status.backend, value: formatState(backendState, copy), tone: backendState },
    { label: copy.status.stream, value: formatState(streamState, copy), tone: streamState },
    {
      label: copy.status.notifications,
      value: formatState(notificationPermission, copy),
      tone: notificationPermission,
    },
    { label: copy.status.lastScan, value: formatTimestamp(lastScanAt), tone: "neutral" },
    { label: copy.status.symbols, value: String(symbolCount), tone: "neutral" },
  ];

  return (
    <main className="app-shell console-shell">
      <header className="console-topbar">
        <div className="console-brand">
          <h1>
            AlphaPulse <span>OKX</span>
          </h1>
          <p>{copy.subtitle}</p>
        </div>
        <div className="console-nav" role="group" aria-label={copy.aria.viewMode}>
          {[
            ["radar", copy.views.radar],
            ["macro", copy.views.macro],
          ].map(([value, label]) => (
            <button
              className={viewMode === value ? "active" : ""}
              key={value}
              onClick={() => onViewModeChange(value as ViewMode)}
              type="button"
            >
              {label}
            </button>
          ))}
        </div>
        <dl className="console-status" aria-label={copy.aria.connectionStatus}>
          {statusItems.map((item) => (
            <div className={`status-pill status-pill-${item.tone}`} key={item.label}>
              <dt>{item.label}</dt>
              <dd>{item.value}</dd>
            </div>
          ))}
        </dl>
        <div className="console-actions">
          <div className="toolbar-group" role="group" aria-label={copy.aria.themeMode}>
            {[
              ["light", copy.themes.light],
              ["dark", copy.themes.dark],
              ["system", copy.themes.system],
            ].map(([value, label]) => (
              <button
                className={themeMode === value ? "active" : ""}
                key={value}
                onClick={() => onThemeModeChange(value as ThemeMode)}
                type="button"
              >
                {label}
              </button>
            ))}
          </div>
          <div className="toolbar-group" role="group" aria-label={copy.aria.languageMode}>
            {[
              ["zh", copy.languages.zh],
              ["en", copy.languages.en],
            ].map(([value, label]) => (
              <button
                className={language === value ? "active" : ""}
                key={value}
                onClick={() => onLanguageChange(value as Language)}
                type="button"
              >
                {label}
              </button>
            ))}
          </div>
          <button onClick={onRequestNotifications} type="button">
            {copy.actions.enableNotifications}
          </button>
        </div>
      </header>
      {children}
    </main>
  );
}
```

- [ ] **Step 3: Wire `ConsoleShell` into `App.tsx`**

In `frontend/src/App.tsx`, add:

```ts
import { ConsoleShell } from "./ConsoleShell";
```

Replace the current top-level `app-shell` wrapper and header/toolbar rendering with:

```tsx
  return (
    <ConsoleShell
      backendState={backendState}
      copy={copy}
      language={language}
      lastScanAt={snapshot.last_scan_at_ms}
      notificationPermission={notificationPermission}
      onLanguageChange={setLanguage}
      onRequestNotifications={requestNotifications}
      onThemeModeChange={setThemeMode}
      onViewModeChange={setViewMode}
      streamState={streamState}
      symbolCount={snapshot.symbols.length}
      themeMode={themeMode}
      viewMode={viewMode}
    >
      {viewMode === "macro" ? (
        <MacroPanel
          copy={copy}
          error={macroError}
          loading={macroLoading}
          onRefresh={() => {
            void loadMacro(true);
          }}
          snapshot={macroSnapshot}
          themeMode={themeMode}
        />
      ) : filteredSymbols.length === 0 ? (
        <section className="empty-state">
          <h2>{copy.empty.title}</h2>
          <p>{copy.empty.body}</p>
        </section>
      ) : (
        <section className="radar-grid">
          <div className="table-panel">
            <table>
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
                {filteredSymbols.map((symbol) => (
                  <tr
                    className={symbol.inst_id === selected?.inst_id ? "selected" : ""}
                    key={symbol.inst_id}
                    onClick={() => setSelectedId(symbol.inst_id)}
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
                            openTradingView(symbol);
                          }}
                          title={copy.actions.openTradingView}
                          type="button"
                        >
                          TV
                        </button>
                      </div>
                    </td>
                    <td>{formatPrice(symbol.price)}</td>
                    <td>{formatPct(symbol.change_5m_pct)}</td>
                    <td>{formatPct(symbol.change_15m_pct)}</td>
                    <td>{formatPct(symbol.change_1h_pct)}</td>
                    <td>{formatScore(symbol.trend_score, copy)}</td>
                    <td>{formatScore(symbol.range_score, copy)}</td>
                    <td>{symbol.trigger_reason || copy.misc.watching}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <aside className="detail-panel">
            {selected ? (
              <SymbolDetail
                copy={copy}
                onClosePaper={submitPaperClose}
                onOpenTradingView={openTradingView}
                onLeverageChange={setOrderLeverage}
                onMarginChange={setOrderMargin}
                onOpenPaper={submitPaperOrder}
                orderLeverage={orderLeverage}
                orderMargin={orderMargin}
                paper={snapshot.paper}
                symbol={selected}
                themeMode={themeMode}
                tradeBusy={tradeBusy}
                tradeError={tradeError}
              />
            ) : null}
          </aside>
        </section>
      )}
      {viewMode === "radar" && tradingViewSymbol ? (
        <TradingViewModal
          copy={copy}
          language={language}
          onClose={() => setTradingViewSymbol(null)}
          symbol={tradingViewSymbol}
          themeMode={themeMode}
        />
      ) : null}
    </ConsoleShell>
  );
```

Move the Radar filter buttons into the Radar branch in this task. Delete the old `.topbar` and global `.toolbar` JSX from `App.tsx`; theme/language/notification controls now live in `ConsoleShell`.

- [ ] **Step 4: Run the integration tests**

Run:

```bash
cd frontend
npm test -- App.test.tsx
```

Expected: existing theme, language, and view-switching tests pass. New Task 1 tests still fail because `MacroSummaryStrip`, `RadarWorkspace`, and `SymbolDetailPanel` are not implemented.

- [ ] **Step 5: Commit shell extraction**

```bash
git add frontend/src/i18n.ts frontend/src/ConsoleShell.tsx frontend/src/App.tsx frontend/src/App.test.tsx
git commit -m "refactor: add frontend console shell"
```

### Task 4: Add Macro Summary Strip

**Files:**
- Create: `frontend/src/MacroSummaryStrip.tsx`
- Modify: `frontend/src/App.tsx`

- [ ] **Step 1: Create the macro summary component**

Create `frontend/src/MacroSummaryStrip.tsx`:

```tsx
import type { Copy } from "./i18n";
import type { BtcMacroSnapshot } from "./types";
import { formatPct, formatRegime, formatUsd } from "./uiFormat";

export function MacroSummaryStrip({
  copy,
  error,
  loading,
  snapshot,
}: {
  copy: Copy;
  error: string | null;
  loading: boolean;
  snapshot: BtcMacroSnapshot | null;
}) {
  if (loading && snapshot === null) {
    return (
      <section className="macro-summary-strip is-loading" data-testid="macro-summary-strip">
        <div>
          <span>{copy.macro.summaryLabel}</span>
          <strong>{copy.macro.summaryLoading}</strong>
        </div>
      </section>
    );
  }

  if (snapshot === null) {
    return (
      <section className="macro-summary-strip is-unavailable" data-testid="macro-summary-strip">
        <div>
          <span>{copy.macro.summaryLabel}</span>
          <strong>{copy.macro.summaryUnavailable}</strong>
          {error ? <em>{error}</em> : null}
        </div>
      </section>
    );
  }

  const metrics = [
    { label: copy.macro.regime, value: formatRegime(snapshot.regime, copy) },
    { label: copy.macro.price, value: formatUsd(snapshot.price) },
    { label: copy.macro.confidence, value: `${snapshot.confidence}/100`, tone: "positive" },
    {
      label: copy.macro.drawdown,
      value: formatPct(snapshot.trend.drawdown_from_window_ath_pct),
      tone: snapshot.trend.drawdown_from_window_ath_pct < 0 ? "negative" : "positive",
    },
    {
      label: copy.macro.cycleProgress,
      value: formatPct(snapshot.cycle.estimated_cycle_progress_pct),
    },
    {
      label: copy.macro.ma200w,
      value: snapshot.trend.ma_200w === null ? "-" : formatUsd(snapshot.trend.ma_200w),
    },
  ];

  return (
    <section className="macro-summary-strip" data-testid="macro-summary-strip">
      <div className="macro-summary-heading">
        <span>{copy.macro.summaryLabel}</span>
        <strong>{snapshot.summary}</strong>
      </div>
      {metrics.map((metric) => (
        <div className="macro-summary-tile" key={metric.label}>
          <span>{metric.label}</span>
          <strong className={metric.tone ?? ""}>{metric.value}</strong>
        </div>
      ))}
    </section>
  );
}
```

- [ ] **Step 2: Render the summary in the Radar branch**

In `frontend/src/App.tsx`, import:

```ts
import { MacroSummaryStrip } from "./MacroSummaryStrip";
```

Inside the Radar branch, render the summary before the Radar content:

```tsx
      {viewMode === "macro" ? (
        <MacroPanel
          copy={copy}
          error={macroError}
          loading={macroLoading}
          onRefresh={() => {
            void loadMacro(true);
          }}
          snapshot={macroSnapshot}
          themeMode={themeMode}
        />
      ) : (
        <>
          <MacroSummaryStrip
            copy={copy}
            error={macroError}
            loading={macroLoading}
            snapshot={macroSnapshot}
          />
          {radarView}
        </>
      )}
```

- [ ] **Step 3: Run tests and verify macro summary tests pass**

Run:

```bash
cd frontend
npm test -- App.test.tsx
```

Expected: the two macro summary tests from Task 1 pass. Row selection and TV non-selection tests still fail because Radar components are absent.

- [ ] **Step 4: Commit macro summary**

```bash
git add frontend/src/MacroSummaryStrip.tsx frontend/src/App.tsx frontend/src/App.test.tsx
git commit -m "feat: add radar macro summary strip"
```

### Task 5: Extract Radar Workspace, Table, Detail Panel, and TradingView Modal

**Files:**
- Create: `frontend/src/RadarWorkspace.tsx`
- Create: `frontend/src/RadarTable.tsx`
- Create: `frontend/src/SymbolDetailPanel.tsx`
- Create: `frontend/src/TradingViewModal.tsx`
- Modify: `frontend/src/App.tsx`

- [ ] **Step 1: Create `RadarTable.tsx`**

Create `frontend/src/RadarTable.tsx`:

```tsx
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
```

- [ ] **Step 2: Create `TradingViewModal.tsx`**

Create `frontend/src/TradingViewModal.tsx` with the current modal behavior:

```tsx
import type { Copy, Language } from "./i18n";
import type { SymbolSnapshot } from "./types";
import type { ThemeMode } from "./uiFormat";

export function TradingViewModal({
  copy,
  language,
  onClose,
  symbol,
  themeMode,
}: {
  copy: Copy;
  language: Language;
  onClose: () => void;
  symbol: SymbolSnapshot;
  themeMode: ThemeMode;
}) {
  const tradingViewSymbol = resolveTradingViewSymbol(symbol.inst_id);
  const title = `${symbol.inst_id} TradingView`;
  return (
    <div className="tv-modal-backdrop" onClick={onClose}>
      <section
        aria-label={title}
        aria-modal="true"
        className="tv-modal"
        onClick={(event) => event.stopPropagation()}
        role="dialog"
      >
        <header>
          <div>
            <h2>{symbol.inst_id}</h2>
            <p>{tradingViewSymbol ?? copy.detail.tradingViewUnavailable}</p>
          </div>
          <button
            aria-label={copy.actions.closeTradingView}
            className="tv-modal-close"
            onClick={onClose}
            type="button"
          >
            x
          </button>
        </header>
        <div className="tv-modal-frame-wrap">
          {tradingViewSymbol ? (
            <iframe
              allow="fullscreen"
              src={buildTradingViewEmbedUrl(tradingViewSymbol, themeMode, language)}
              title={title}
            />
          ) : (
            <div className="tv-modal-empty">{copy.detail.tradingViewUnavailable}</div>
          )}
        </div>
      </section>
    </div>
  );
}

function resolveTradingViewSymbol(instId: string): string | null {
  const normalized = instId.toUpperCase().replace(/[^A-Z0-9-]/g, "");
  const swapMatch = normalized.match(/^([A-Z0-9]+)-USDT-SWAP$/);
  if (swapMatch) {
    return `OKX:${swapMatch[1]}USDT.P`;
  }

  const spotMatch = normalized.match(/^([A-Z0-9]+)-USDT$/);
  if (spotMatch) {
    return `OKX:${spotMatch[1]}USDT`;
  }

  const compact = normalized.replace(/-/g, "");
  return compact.length > 0 ? `OKX:${compact}` : null;
}

function buildTradingViewEmbedUrl(symbol: string, themeMode: ThemeMode, language: Language): string {
  const params = new URLSearchParams({
    symbol,
    interval: "15",
    theme: themeMode === "light" ? "light" : "dark",
    style: "1",
    locale: language === "zh" ? "zh_CN" : "en",
    enable_publishing: "0",
    allow_symbol_change: "0",
    hide_top_toolbar: "0",
    withdateranges: "1",
  });
  return `https://s.tradingview.com/widgetembed/?${params.toString()}`;
}
```

- [ ] **Step 3: Create `SymbolDetailPanel.tsx`**

Create `frontend/src/SymbolDetailPanel.tsx` by moving the current `SymbolDetail` body out of `App.tsx`, then wrap the existing sections with the approved group classes. The exported component signature should be:

```tsx
import { ChartPanel } from "./ChartPanel";
import type { Copy } from "./i18n";
import type {
  PaperAccountSnapshot,
  PaperSide,
  SymbolSnapshot,
} from "./types";
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
  const trades = paper.trades.filter((trade) => trade.inst_id === symbol.inst_id).slice(0, 5);

  return (
    <section className="symbol-detail-panel" data-testid="symbol-detail-panel">
      <header className="detail-header">
        <div>
          <h2>{symbol.inst_id}</h2>
          <p>{symbol.trigger_reason || copy.detail.noActiveTrigger}</p>
        </div>
        <button
          aria-label={`${copy.actions.openTradingView} ${symbol.inst_id}`}
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
            <dd className={symbol.change_5m_pct < 0 ? "negative" : "positive"}>
              {formatPct(symbol.change_5m_pct)}
            </dd>
          </div>
          <div>
            <dt>15m</dt>
            <dd className={symbol.change_15m_pct < 0 ? "negative" : "positive"}>
              {formatPct(symbol.change_15m_pct)}
            </dd>
          </div>
          <div>
            <dt>1h</dt>
            <dd className={symbol.change_1h_pct < 0 ? "negative" : "positive"}>
              {formatPct(symbol.change_1h_pct)}
            </dd>
          </div>
        </dl>
      </section>

      <ChartPanel copy={copy} symbol={symbol} themeMode={themeMode} />

      <section className="detail-section">
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
                  {formatPrice(zone.lower)}-{formatPrice(zone.upper)}{" "}
                  {copy.detail.distance} {formatPct(zone.distance_pct)}
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

      <section className="detail-section">
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
                    {formatSignedUsdt(position.unrealized_pnl)} /{" "}
                    {formatPct(position.pnl_pct)}
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
                    {copy.paper.tradeActions[trade.action]}{" "}
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
      </section>
    </section>
  );
}
```

Use the formatter imports shown above for FVG, Levels, paper metrics, order form, position, and trade list JSX.

- [ ] **Step 4: Create `RadarWorkspace.tsx`**

Create `frontend/src/RadarWorkspace.tsx`:

```tsx
import type { Copy } from "./i18n";
import { RadarTable } from "./RadarTable";
import { SymbolDetailPanel } from "./SymbolDetailPanel";
import type {
  PaperAccountSnapshot,
  PaperSide,
  SymbolSnapshot,
} from "./types";
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
```

- [ ] **Step 5: Wire Radar components and modal into `App.tsx`**

In `frontend/src/App.tsx`, add:

```ts
import { RadarWorkspace } from "./RadarWorkspace";
import { TradingViewModal } from "./TradingViewModal";
```

Change `openTradingView` so it does not select the symbol:

```ts
  function openTradingView(symbol: SymbolSnapshot) {
    setTradingViewSymbol(symbol);
  }
```

Replace the Radar table/detail JSX with:

```tsx
          <RadarWorkspace
            copy={copy}
            filter={filter}
            filteredSymbols={filteredSymbols}
            onClosePaper={submitPaperClose}
            onFilterChange={setFilter}
            onLeverageChange={setOrderLeverage}
            onMarginChange={setOrderMargin}
            onOpenPaper={submitPaperOrder}
            onOpenTradingView={openTradingView}
            onSelectSymbol={setSelectedId}
            orderLeverage={orderLeverage}
            orderMargin={orderMargin}
            paper={snapshot.paper}
            selected={selected}
            themeMode={themeMode}
            tradeBusy={tradeBusy}
            tradeError={tradeError}
          />
```

Delete the local `SymbolDetail` and `TradingViewModal` declarations from `App.tsx` after their imports compile.

- [ ] **Step 6: Run focused tests for Radar behavior**

Run:

```bash
cd frontend
npm test -- App.test.tsx
```

Expected: all Task 1 tests pass. Existing tests for paper trading, TradingView, Macro view, language, and theme still pass.

- [ ] **Step 7: Commit Radar extraction**

```bash
git add frontend/src/RadarWorkspace.tsx frontend/src/RadarTable.tsx frontend/src/SymbolDetailPanel.tsx frontend/src/TradingViewModal.tsx frontend/src/App.tsx frontend/src/App.test.tsx
git commit -m "refactor: extract radar workspace components"
```

### Task 6: Apply Console, Radar, and Detail Styling

**Files:**
- Modify: `frontend/src/styles.css`
- Modify: `frontend/src/styles.test.ts`

- [ ] **Step 1: Extend CSS tests for the approved palette and responsive rules**

In `frontend/src/styles.test.ts`, add:

```ts
describe("console palette", () => {
  it("uses the approved radar reference colors and responsive console classes", () => {
    expect(css).toContain("--app-bg: #0b0e14");
    expect(css).toContain("--surface: #151924");
    expect(css).toContain("--border: #2b313f");
    expect(css).toContain(".console-topbar");
    expect(css).toContain(".macro-summary-strip");
    expect(css).toContain(".radar-workspace");
    expect(css).toContain("@media (max-width: 960px)");
  });
});
```

- [ ] **Step 2: Run CSS tests and verify they fail**

Run:

```bash
cd frontend
npm test -- styles.test.ts
```

Expected: FAIL because the approved Radar palette and new class names are absent from `styles.css`.

- [ ] **Step 3: Update dark theme variables**

In `frontend/src/styles.css`, replace the `:root[data-theme="dark"]` variable block and the matching `@media (prefers-color-scheme: dark)` fallback variables with:

```css
:root[data-theme="dark"] {
  --app-bg: #0b0e14;
  --surface: #151924;
  --surface-elevated: #1a2030;
  --text-primary: #f8fafc;
  --text-muted: #94a3b8;
  --text-soft: #cbd5e1;
  --border: #2b313f;
  --border-strong: #3a4354;
  --border-subtle: rgba(43, 49, 63, 0.62);
  --accent: #3b82f6;
  --accent-contrast: #ffffff;
  --positive: #34d399;
  --negative: #fb7185;
  --fvg-long-fill: rgba(52, 211, 153, 0.2);
  --fvg-long-border: rgba(52, 211, 153, 0.78);
  --fvg-short-fill: rgba(251, 113, 133, 0.2);
  --fvg-short-border: rgba(251, 113, 133, 0.78);
  --selected: rgba(59, 130, 246, 0.14);
  --button-bg: #0b0e14;
  color-scheme: dark;
}
```

- [ ] **Step 4: Add console shell styles**

Add these styles after the `h1` rules:

```css
.console-shell {
  display: flex;
  min-height: 100vh;
  flex-direction: column;
  padding: 0;
}

.console-topbar {
  display: grid;
  grid-template-columns: minmax(180px, auto) auto minmax(320px, 1fr) auto;
  gap: 14px;
  align-items: center;
  border-bottom: 1px solid var(--border);
  background: var(--surface);
  padding: 12px 16px;
}

.console-brand h1 {
  font-size: 18px;
  letter-spacing: 0.02em;
}

.console-brand h1 span {
  color: var(--accent);
}

.console-brand p {
  margin-top: 3px;
  color: var(--text-muted);
  font-size: 12px;
}

.console-nav,
.console-actions,
.console-status {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
}

.console-status {
  justify-content: flex-end;
  margin: 0;
}

.status-pill {
  display: inline-flex;
  gap: 7px;
  align-items: center;
  border: 1px solid var(--border);
  border-radius: 999px;
  background: var(--app-bg);
  padding: 6px 10px;
}

.status-pill dt,
.status-pill dd {
  margin: 0;
  font-size: 11px;
}

.status-pill dt {
  color: var(--text-muted);
}

.status-pill dd {
  color: var(--text-soft);
  font-weight: 700;
}

.status-pill::before {
  width: 7px;
  height: 7px;
  flex: 0 0 auto;
  border-radius: 999px;
  background: var(--text-muted);
  content: "";
}

.status-pill-connected::before,
.status-pill-granted::before {
  background: var(--positive);
  box-shadow: 0 0 8px color-mix(in srgb, var(--positive) 55%, transparent);
}

.status-pill-disconnected::before,
.status-pill-denied::before {
  background: var(--negative);
}
```

- [ ] **Step 5: Add macro summary, Radar, and detail styles**

Add these blocks near the existing Radar and detail styles:

```css
.macro-summary-strip {
  display: grid;
  grid-template-columns: minmax(220px, 1.4fr) repeat(6, minmax(120px, 1fr));
  gap: 10px;
  border-bottom: 1px solid var(--border);
  background: var(--app-bg);
  padding: 12px 16px;
}

.macro-summary-heading,
.macro-summary-tile {
  min-width: 0;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--surface);
  padding: 10px 12px;
}

.macro-summary-strip span,
.macro-summary-strip em {
  color: var(--text-muted);
  font-size: 11px;
}

.macro-summary-strip strong {
  display: block;
  margin-top: 5px;
  overflow: hidden;
  color: var(--text-primary);
  font-size: 13px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.macro-summary-strip.is-loading,
.macro-summary-strip.is-unavailable {
  grid-template-columns: minmax(0, 1fr);
}

.radar-filterbar {
  margin: 0;
  border-bottom: 1px solid var(--border);
  background: var(--app-bg);
  padding: 10px 16px;
}

.radar-workspace {
  flex: 1;
  grid-template-columns: minmax(0, 1fr) 390px;
  gap: 0;
  margin-top: 0;
  min-height: 0;
}

.radar-table-panel,
.detail-panel {
  border-radius: 0;
}

.radar-table th {
  position: sticky;
  top: 0;
  z-index: 2;
  background: var(--surface);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.radar-table tr.selected {
  box-shadow: inset 3px 0 0 var(--accent);
}

.score-badge,
.signal-pill {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--app-bg);
  padding: 3px 7px;
  font-size: 11px;
  font-weight: 800;
}

.score-badge.positive,
.signal-pill.positive {
  border-color: color-mix(in srgb, var(--positive) 35%, transparent);
  background: color-mix(in srgb, var(--positive) 12%, transparent);
  color: var(--positive);
}

.score-badge.negative,
.signal-pill.negative {
  border-color: color-mix(in srgb, var(--negative) 35%, transparent);
  background: color-mix(in srgb, var(--negative) 12%, transparent);
  color: var(--negative);
}

.signal-cell {
  display: flex;
  min-width: 220px;
  gap: 8px;
  align-items: center;
}

.symbol-detail-panel {
  display: grid;
  gap: 14px;
}

.detail-section {
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--surface);
  padding: 12px;
}

.detail-section h3 {
  margin: 0 0 10px;
}

.detail-metric-strip {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 8px;
  margin: 0;
}

.detail-metric-strip div {
  border: 1px solid var(--border-subtle);
  border-radius: 7px;
  background: var(--app-bg);
  padding: 8px;
}
```

- [ ] **Step 6: Add responsive styles**

Add:

```css
@media (max-width: 960px) {
  .console-topbar {
    grid-template-columns: 1fr;
  }

  .console-status,
  .console-actions {
    justify-content: flex-start;
  }

  .macro-summary-strip {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .radar-workspace {
    grid-template-columns: 1fr;
  }

  .detail-panel {
    border-top: 1px solid var(--border);
  }
}

@media (max-width: 640px) {
  .macro-summary-strip {
    grid-template-columns: 1fr;
  }

  .detail-metric-strip {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
```

- [ ] **Step 7: Run CSS and app tests**

Run:

```bash
cd frontend
npm test -- styles.test.ts App.test.tsx
```

Expected: both test files pass.

- [ ] **Step 8: Commit console styling**

```bash
git add frontend/src/styles.css frontend/src/styles.test.ts
git commit -m "style: apply radar console layout"
```

### Task 7: Restyle Macro First Viewport

**Files:**
- Modify: `frontend/src/MacroPanel.tsx`
- Modify: `frontend/src/styles.css`
- Modify: `frontend/src/App.test.tsx`

- [ ] **Step 1: Add a test for the Macro first viewport structure**

In `frontend/src/App.test.tsx`, add this assertion to the existing `renders the macro cycle view` test after `expect(await screen.findByText("BTC 大周期")).toBeInTheDocument();`:

```tsx
    expect(screen.getByTestId("macro-regime-card")).toHaveTextContent("熊市反弹");
    expect(screen.getByTestId("macro-cycle-progress")).toHaveTextContent("55.00%");
```

- [ ] **Step 2: Run the focused test and verify it fails**

Run:

```bash
cd frontend
npm test -- App.test.tsx
```

Expected: FAIL because `macro-regime-card` and `macro-cycle-progress` do not exist.

- [ ] **Step 3: Update the Macro first viewport markup**

In `frontend/src/MacroPanel.tsx`, replace the current `macro-regime-band`, `macro-grid`, and `macro-progress` first-viewport block with:

```tsx
      <section className="macro-overview-grid">
        <div className="macro-regime-card" data-testid="macro-regime-card">
          <span>{copy.macro.regime}</span>
          <strong>{formatRegime(snapshot.regime, copy)}</strong>
          <div className="macro-regime-card-footer">
            <div>
              <span>{copy.macro.price}</span>
              <b>{formatUsd(snapshot.price)}</b>
            </div>
            <div>
              <span>{copy.macro.confidence}</span>
              <b className="positive">{snapshot.confidence}/100</b>
            </div>
          </div>
        </div>
        <MetricTile
          label={copy.macro.daysSinceHalving}
          value={`${snapshot.cycle.days_since_halving}`}
        />
        <MetricTile
          label={copy.macro.drawdown}
          value={formatPct(snapshot.trend.drawdown_from_window_ath_pct)}
          tone={snapshot.trend.drawdown_from_window_ath_pct < -0.3 ? "negative" : undefined}
        />
        <MetricTile
          label={copy.macro.ma200w}
          value={snapshot.trend.ma_200w === null ? "-" : formatUsd(snapshot.trend.ma_200w)}
        />
        <MetricTile
          label={copy.macro.change90d}
          value={snapshot.momentum.change_90d_pct === null ? "-" : formatPct(snapshot.momentum.change_90d_pct)}
          tone={
            snapshot.momentum.change_90d_pct !== null && snapshot.momentum.change_90d_pct < 0
              ? "negative"
              : "positive"
          }
        />
        <div className="macro-cycle-card" data-testid="macro-cycle-progress">
          <div className="macro-cycle-card-dates">
            <span>
              {copy.macro.lastHalving}: <strong>{formatDate(snapshot.cycle.last_halving_ms)}</strong>
            </span>
            <span>
              {copy.macro.nextHalving}:{" "}
              <strong>{formatDate(snapshot.cycle.next_halving_estimate_ms)}</strong>
            </span>
          </div>
          <div className="macro-progress-track">
            <span
              style={{
                width: `${Math.min(
                  100,
                  Math.max(0, snapshot.cycle.estimated_cycle_progress_pct * 100),
                )}%`,
              }}
            />
          </div>
          <p>
            {copy.macro.cycleProgress}:{" "}
            <strong>{formatPct(snapshot.cycle.estimated_cycle_progress_pct)}</strong>
          </p>
        </div>
      </section>
```

- [ ] **Step 4: Add Macro overview styles using the Radar palette**

In `frontend/src/styles.css`, add:

```css
.macro-overview-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 12px;
  margin-top: 16px;
}

.macro-regime-card {
  grid-column: span 2;
  min-height: 156px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--surface);
  padding: 18px;
}

.macro-regime-card span,
.macro-cycle-card span,
.macro-cycle-card p {
  color: var(--text-muted);
  font-size: 12px;
}

.macro-regime-card > strong {
  display: block;
  margin-top: 8px;
  color: var(--text-primary);
  font-size: 28px;
  line-height: 1.1;
}

.macro-regime-card-footer {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  margin-top: 28px;
}

.macro-regime-card-footer b {
  display: block;
  margin-top: 5px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

.macro-cycle-card {
  grid-column: span 2;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--surface);
  padding: 16px;
}

.macro-cycle-card-dates {
  display: flex;
  justify-content: space-between;
  gap: 12px;
}

.macro-cycle-card p {
  margin-top: 10px;
  text-align: right;
}

.macro-cycle-card p strong {
  color: var(--accent);
}

@media (max-width: 960px) {
  .macro-overview-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 640px) {
  .macro-overview-grid,
  .macro-regime-card-footer,
  .macro-cycle-card-dates {
    grid-template-columns: 1fr;
  }

  .macro-regime-card,
  .macro-cycle-card {
    grid-column: span 1;
  }
}
```

- [ ] **Step 5: Run Macro tests**

Run:

```bash
cd frontend
npm test -- App.test.tsx
```

Expected: all App tests pass.

- [ ] **Step 6: Commit Macro restyle**

```bash
git add frontend/src/MacroPanel.tsx frontend/src/styles.css frontend/src/App.test.tsx
git commit -m "style: restyle macro overview"
```

### Task 8: Full Verification and Visual Check

**Files:**
- Modify only files needed to fix failures found by verification.

- [ ] **Step 1: Run all frontend tests**

Run:

```bash
cd frontend
npm test
```

Expected: PASS.

- [ ] **Step 2: Run the production build**

Run:

```bash
cd frontend
npm run build
```

Expected: PASS with Vite build output and no TypeScript errors.

- [ ] **Step 3: Start the frontend dev server**

Run:

```bash
cd frontend
npm run dev
```

Expected: Vite starts at `http://127.0.0.1:5173`. Keep this process running for browser verification.

- [ ] **Step 4: Browser-check desktop and mobile layouts**

Open `http://127.0.0.1:5173` and check:

- Desktop Radar: header, status pills, macro summary strip, dense table, selected detail panel.
- Desktop Macro: first viewport shows regime card, metric tiles, cycle progress, and existing deep sections below.
- Mobile width near `390px`: header actions wrap, macro summary stacks, table scrolls without button text overlap, detail panel appears below table.
- Light theme: text remains readable and controls remain visible.

- [ ] **Step 5: Stop the dev server**

Stop the Vite process with `Ctrl+C` in the terminal session that started it.

- [ ] **Step 6: Final commit if verification fixes were needed**

If Step 1 through Step 4 required corrections after Task 7, commit those corrections:

```bash
git add frontend/src
git commit -m "fix: polish frontend console verification issues"
```

If no corrections were needed, do not create an empty commit.

## Self-Review

Spec coverage:

- Componentized console shell is covered by Task 3.
- Radar macro summary is covered by Task 4.
- Dense Radar table and grouped detail panel are covered by Task 5.
- Radar reference palette and responsive rules are covered by Task 6.
- Macro first-viewport card hierarchy is covered by Task 7.
- Tests and build verification are covered by Tasks 1, 2, 6, 7, and 8.

Type consistency:

- `Filter`, `ThemeMode`, and `ViewMode` are defined once in `uiFormat.ts`.
- `MacroSummaryStrip` consumes `BtcMacroSnapshot | null` and does not fetch data.
- `RadarWorkspace` receives filtered symbols and selected symbol from `App`, so `App` remains the state owner.
- `TradingViewModal` keeps the existing `ThemeMode` and `Language` inputs.

Placeholder scan:

- The plan contains explicit paths, commands, expected outcomes, and code snippets for each code-changing task.
