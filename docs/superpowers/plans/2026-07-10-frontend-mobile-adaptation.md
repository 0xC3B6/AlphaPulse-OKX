# Frontend Mobile Adaptation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Adapt all five AlphaPulse OKX frontend pages for touch-friendly phone use from 360px while leaving the existing desktop console behavior unchanged.

**Architecture:** Keep the current React state, API, WebSocket, page components, and desktop markup as the source of truth. Add semantic mobile hooks to the shared shell, render mobile card alternatives beside the densest desktop tables from identical props, and activate them with one final `@media (max-width: 760px)` layer so later terminal-parity CSS cannot override the phone layout.

**Tech Stack:** React 18, TypeScript, Vite 5, Vitest, Testing Library, CSS media queries, Recharts, lightweight-charts

---

## File Structure

- Modify `frontend/src/ConsoleShell.tsx`: expose semantic/test hooks for the compact phone header, quick account strip, five-item bottom navigation, and scrollable page area.
- Modify `frontend/src/RadarTable.tsx`: render a mobile signal-card list from the same terminal rows used by the desktop radar table.
- Modify `frontend/src/MonitorPage.tsx`: turn the selected-symbol detail container into the phone bottom sheet, add its backdrop, and support Escape close.
- Modify `frontend/src/TradePage.tsx`: render phone position cards from the same `PaperPositionSnapshot[]` and expose the selected-position detail region.
- Modify `frontend/src/ReviewPage.tsx`: render phone strategy, attribution, and trade-record cards next to existing desktop tables; reuse the existing position-history card list.
- Modify `frontend/src/styles.css`: hide mobile-only alternatives by default and append the authoritative 760px phone layer for the shell and all five pages.
- Modify `frontend/src/App.test.tsx`: specify mobile structure and shared interaction behavior before implementation.
- Modify `frontend/src/styles.test.ts`: specify breakpoint, safe-area, overflow, touch-size, and visibility contracts before CSS implementation.
- No backend, API, data-model, or persistence files change.

### Task 1: Add the semantic mobile shell contract

**Files:**
- Modify: `frontend/src/App.test.tsx`
- Modify: `frontend/src/ConsoleShell.tsx`

- [ ] **Step 1: Write the failing shell test**

Add this test after `"uses Figma task rail pages and keeps radar filters inside Monitor"`:

```tsx
it("exposes the five-page mobile shell and active navigation semantics", async () => {
  vi.stubGlobal("fetch", vi.fn().mockImplementation((input: RequestInfo | URL) => {
    if (String(input).includes("/api/macro/btc")) {
      return Promise.resolve({ json: async () => macroSnapshot, ok: true });
    }
    return Promise.resolve({ json: async () => snapshot, ok: true });
  }));

  render(<App />);

  expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);
  const navigation = screen.getByTestId("figma-sidebar");
  expect(navigation).toHaveClass("mobile-bottom-navigation");
  const pageButtons = ["监控", "宏观", "策略", "交易", "复盘"].map((name) =>
    within(navigation).getByRole("button", { name }),
  );
  expect(pageButtons).toHaveLength(5);
  expect(within(navigation).getByRole("button", { name: "监控" })).toHaveAttribute(
    "aria-current",
    "page",
  );
  expect(screen.getByTestId("figma-radar-header")).toHaveClass("mobile-console-header");
  expect(screen.getByTestId("terminal-quick-stats")).toHaveClass("mobile-account-strip");
  expect(screen.getByTestId("mobile-page-content")).toContainElement(
    screen.getByTestId("monitor-terminal"),
  );

  fireEvent.click(within(navigation).getByRole("button", { name: "宏观" }));
  expect(within(navigation).getByRole("button", { name: "宏观" })).toHaveAttribute(
    "aria-current",
    "page",
  );
});
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```bash
cd frontend && npm test -- --run src/App.test.tsx -t "exposes the five-page mobile shell"
```

Expected: FAIL because the three mobile classes and `mobile-page-content` hook do not exist.

- [ ] **Step 3: Add the smallest shell hooks**

In `ConsoleShell.tsx`, update only the existing elements:

```tsx
<nav
  className="task-rail figma-sidebar mobile-bottom-navigation"
  data-testid="figma-sidebar"
  aria-label={copy.aria.taskNavigation}
>
```

```tsx
<section className="console-main mobile-page-content" data-testid="mobile-page-content">
  <header
    className="console-topbar figma-radar-header mobile-console-header"
    data-testid="figma-radar-header"
  >
```

```tsx
<div
  className="terminal-quick-stats mobile-account-strip"
  data-testid="terminal-quick-stats"
>
```

Keep the existing `aria-current={viewMode === value ? "page" : undefined}` on every task button.

- [ ] **Step 4: Run the focused test and verify GREEN**

Run:

```bash
cd frontend && npm test -- --run src/App.test.tsx -t "exposes the five-page mobile shell"
```

Expected: PASS.

- [ ] **Step 5: Run the full app test file**

Run:

```bash
cd frontend && npm test -- --run src/App.test.tsx
```

Expected: all App tests PASS.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/App.test.tsx frontend/src/ConsoleShell.tsx
git commit -m "test: define mobile console shell"
```

### Task 2: Add mobile Radar cards and selected-symbol sheet behavior

**Files:**
- Modify: `frontend/src/App.test.tsx`
- Modify: `frontend/src/RadarTable.tsx`
- Modify: `frontend/src/MonitorPage.tsx`
- Modify: `frontend/src/styles.css`

- [ ] **Step 1: Write failing Radar card and sheet tests**

Add these tests near the existing dense-radar selection tests:

```tsx
it("renders touch-friendly mobile radar cards from the live symbol rows", async () => {
  vi.stubGlobal("fetch", vi.fn().mockImplementation((input: RequestInfo | URL) => {
    if (String(input).includes("/api/macro/btc")) {
      return Promise.resolve({ json: async () => macroSnapshot, ok: true });
    }
    return Promise.resolve({ json: async () => snapshot, ok: true });
  }));

  render(<App />);

  expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);
  const cards = screen.getByTestId("mobile-radar-cards");
  expect(cards).toHaveTextContent("LAB");
  expect(cards).toHaveTextContent("SHORT");
  expect(cards).toHaveTextContent("Trend");
  expect(cards).toHaveTextContent("84");
  expect(cards).toHaveTextContent("Range");
  expect(cards).toHaveTextContent("dynamic");
  expect(
    within(cards).getByRole("button", { hidden: true, name: /查看 LAB-USDT-SWAP 详情/ }),
  ).toBeInTheDocument();
});

it("opens and closes the mobile symbol detail sheet", async () => {
  vi.stubGlobal("fetch", vi.fn().mockImplementation((input: RequestInfo | URL) => {
    if (String(input).includes("/api/macro/btc")) {
      return Promise.resolve({ json: async () => macroSnapshot, ok: true });
    }
    return Promise.resolve({ json: async () => snapshot, ok: true });
  }));

  render(<App />);

  expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);
  fireEvent.click(
    within(screen.getByTestId("mobile-radar-cards")).getByRole("button", {
      hidden: true,
      name: /查看 LAB-USDT-SWAP 详情/,
    }),
  );

  const sheet = screen.getByRole("dialog", { name: "LAB-USDT-SWAP 详情" });
  expect(sheet).toHaveAttribute("aria-modal", "true");
  expect(sheet).toHaveTextContent("LAB-USDT-SWAP");

  fireEvent.keyDown(window, { key: "Escape" });
  expect(screen.queryByRole("dialog", { name: "LAB-USDT-SWAP 详情" })).not.toBeInTheDocument();
});
```

- [ ] **Step 2: Run both tests and verify RED**

Run:

```bash
cd frontend && npm test -- --run src/App.test.tsx -t "mobile radar|mobile symbol detail"
```

Expected: FAIL because the card list and dialog do not exist.

- [ ] **Step 3: Render mobile cards from the existing terminal rows**

In `RadarTable.tsx`, add the card list immediately after the existing `</table>` and before the current `radar-table-panel` closing `</div>`:

```tsx
<div className="mobile-radar-cards" data-testid="mobile-radar-cards">
  {rows.map((row) => {
    const symbol = row.source;
    return (
      <button
        aria-label={`查看 ${symbol.inst_id} 详情`}
        className={`mobile-radar-card ${
          symbol.inst_id === selectedId ? "selected" : ""
        }`}
        key={symbol.inst_id}
        onClick={() =>
          onSelectSymbol(symbol.inst_id === selectedId ? null : symbol.inst_id)
        }
        type="button"
      >
        <span className="mobile-radar-card-heading">
          <strong>
            {row.hasPosition ? <i className="position-dot" aria-label="open position" /> : null}
            {row.base}
            <small>/USDT</small>
          </strong>
          <SignalBadge value={row.signal} />
        </span>
        <span className="mobile-radar-card-body">
          <span>
            <b>{formatPrice(symbol.price)}</b>
            <em className={row.chg < 0 ? "negative" : "positive"}>
              {formatPct(row.chg)}
            </em>
          </span>
          <span className="mobile-radar-scores">
            <span><small>Trend</small><ScoreBar value={row.trend} tone="trend" /></span>
            <span><small>Range</small><ScoreBar value={row.range} tone="range" /></span>
          </span>
        </span>
        <span className="terminal-tag-list">
          {row.tags.slice(0, 4).map((tag) => (
            <span className="terminal-tag-pill" key={tag}>{tag}</span>
          ))}
        </span>
      </button>
    );
  })}
</div>
```

Do not copy mock reference data. `rows` must remain the only source for both presentations.

- [ ] **Step 4: Add sheet semantics, backdrop, and Escape close**

Change the Monitor import:

```tsx
import { useEffect } from "react";
```

After `selectedVisible` is calculated, add:

```tsx
useEffect(() => {
  if (!selectedVisible) {
    return;
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      onSelectSymbol(null);
    }
  }

  window.addEventListener("keydown", handleKeyDown);
  return () => window.removeEventListener("keydown", handleKeyDown);
}, [onSelectSymbol, selectedVisible]);
```

Replace the selected detail block with:

```tsx
{selectedVisible ? (
  <>
    <button
      aria-label="关闭详情"
      className="mobile-sheet-backdrop"
      onClick={() => onSelectSymbol(null)}
      type="button"
    />
    <aside
      aria-label={`${selectedVisible.inst_id} 详情`}
      aria-modal="true"
      className="detail-panel figma-detail-column mobile-symbol-sheet"
      data-testid="figma-detail-column"
      role="dialog"
    >
      <span className="mobile-sheet-handle" aria-hidden="true" />
      <SymbolDetailPanel
        copy={copy}
        onClose={() => onSelectSymbol(null)}
        onOpenTradingView={onOpenTradingView}
        onTradeSymbol={onTradeSymbol}
        symbol={selectedVisible}
        themeMode={themeMode}
      />
    </aside>
  </>
) : null}
```

- [ ] **Step 5: Hide mobile-only alternatives by default**

Append before the final responsive layer in `styles.css`:

```css
.mobile-radar-cards,
.mobile-sheet-backdrop,
.mobile-sheet-handle {
  display: none;
}
```

- [ ] **Step 6: Run the focused tests and full app suite**

Run:

```bash
cd frontend && npm test -- --run src/App.test.tsx -t "mobile radar|mobile symbol detail"
cd frontend && npm test -- --run src/App.test.tsx
```

Expected: both focused tests PASS, then all App tests PASS.

- [ ] **Step 7: Commit**

```bash
git add frontend/src/App.test.tsx frontend/src/RadarTable.tsx frontend/src/MonitorPage.tsx frontend/src/styles.css
git commit -m "feat: add mobile radar cards and detail sheet"
```

### Task 3: Add mobile paper-position cards

**Files:**
- Modify: `frontend/src/App.test.tsx`
- Modify: `frontend/src/TradePage.tsx`
- Modify: `frontend/src/styles.css`

- [ ] **Step 1: Write the failing position-card test**

Add this after the existing Trade page test:

```tsx
it("selects live paper positions from mobile position cards", async () => {
  vi.stubGlobal("fetch", vi.fn().mockImplementation((input: RequestInfo | URL) => {
    if (String(input).includes("/api/macro/btc")) {
      return Promise.resolve({ json: async () => macroSnapshot, ok: true });
    }
    return Promise.resolve({ json: async () => ({ ...snapshot, paper: activePaper }), ok: true });
  }));

  render(<App />);

  expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);
  fireEvent.click(screen.getByRole("button", { name: "交易" }));
  const cards = screen.getByTestId("mobile-position-cards");
  expect(cards).toHaveTextContent("LAB-USDT-SWAP");
  expect(cards).toHaveTextContent("DOGE-USDT-SWAP");

  fireEvent.click(
    within(cards).getByRole("button", {
      hidden: true,
      name: "查看 DOGE-USDT-SWAP 持仓",
    }),
  );

  expect(screen.getByTestId("trade-selected-position")).toHaveTextContent("0.17784");
  expect(screen.getByLabelText("交易合约")).toHaveValue("DOGE-USDT-SWAP");
});
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```bash
cd frontend && npm test -- --run src/App.test.tsx -t "mobile position cards"
```

Expected: FAIL because `mobile-position-cards` and `trade-selected-position` do not exist.

- [ ] **Step 3: Render cards from `paper.positions`**

Immediately after the entire existing position empty/table conditional in `TradePage.tsx`, add this independent mobile conditional:

```tsx
{paper.positions.length > 0 ? (
  <div className="mobile-position-cards" data-testid="mobile-position-cards">
    {paper.positions.map((position) => (
      <button
        aria-label={`查看 ${position.inst_id} 持仓`}
        className={`mobile-position-card ${
          position.inst_id === selectedPosition?.inst_id ? "selected" : ""
        }`}
        key={position.inst_id}
        onClick={() => onSelectPosition(position.inst_id)}
        type="button"
      >
        <span className="mobile-position-heading">
          <strong>{position.inst_id}</strong>
          <span>{copy.directions[position.side]} · {position.leverage.toFixed(1)}x</span>
          <em className={pnlClass(position.unrealized_pnl)}>
            {formatSignedUsdt(position.unrealized_pnl)}
          </em>
        </span>
        <dl>
          <div><dt>{copy.paper.entry}</dt><dd>{formatPrice(position.entry_price)}</dd></div>
          <div><dt>{copy.paper.mark}</dt><dd>{formatPrice(position.mark_price)}</dd></div>
          <div><dt>{copy.paper.margin}</dt><dd>{formatUsdt(position.margin)}</dd></div>
          <div><dt>{copy.paper.pnl}</dt><dd>{formatPct(position.pnl_pct)}</dd></div>
        </dl>
        <small>{position.primary_signal ?? position.reason ?? "-"}</small>
      </button>
    ))}
  </div>
) : null}
```

Add `data-testid="trade-selected-position"` to the detail section headed by `copy.trade.selectedPosition`:

```tsx
<section className="detail-section" data-testid="trade-selected-position">
```

- [ ] **Step 4: Hide the mobile position cards by default**

Extend the default mobile-only selector in `styles.css`:

```css
.mobile-radar-cards,
.mobile-position-cards,
.mobile-sheet-backdrop,
.mobile-sheet-handle {
  display: none;
}
```

- [ ] **Step 5: Run focused and full tests**

Run:

```bash
cd frontend && npm test -- --run src/App.test.tsx -t "mobile position cards"
cd frontend && npm test -- --run src/App.test.tsx
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/App.test.tsx frontend/src/TradePage.tsx frontend/src/styles.css
git commit -m "feat: add mobile paper position cards"
```

### Task 4: Add mobile Review strategy, attribution, and trade cards

**Files:**
- Modify: `frontend/src/App.test.tsx`
- Modify: `frontend/src/ReviewPage.tsx`
- Modify: `frontend/src/styles.css`

- [ ] **Step 1: Write the failing Review mobile-card test**

Add:

```tsx
it("keeps Review strategy and trade data scannable in mobile cards", async () => {
  vi.stubGlobal("fetch", vi.fn().mockImplementation((input: RequestInfo | URL) => {
    if (String(input).includes("/api/macro/btc")) {
      return Promise.resolve({ json: async () => macroSnapshot, ok: true });
    }
    return Promise.resolve({
      json: async () => ({ ...snapshot, paper: activePaper, strategy_center: strategyCenter }),
      ok: true,
    });
  }));

  render(<App />);

  expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);
  fireEvent.click(screen.getByRole("button", { name: "复盘" }));
  fireEvent.click(screen.getByRole("button", { name: "策略版本对比" }));

  expect(screen.getByTestId("mobile-review-strategies")).toHaveTextContent("v0.1.3");
  expect(screen.getByTestId("mobile-review-strategies")).toHaveTextContent("50.00%");
  expect(screen.getByTestId("mobile-strategy-attribution")).toHaveTextContent(
    "Multiday Reversal",
  );

  fireEvent.click(screen.getByRole("button", { name: "成交记录" }));
  expect(screen.getByTestId("mobile-review-trades")).toHaveTextContent("BREV-USDT-SWAP");
  expect(screen.getByTestId("mobile-review-trades")).toHaveTextContent("+236.94 USDT");
});
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```bash
cd frontend && npm test -- --run src/App.test.tsx -t "Review strategy and trade data"
```

Expected: FAIL because the three mobile lists do not exist.

- [ ] **Step 3: Add mobile strategy cards beside the existing strategy table**

Inside `StrategyStatsTable`, after the desktop table and before the wrapping div closes, add:

```tsx
<div className="mobile-review-strategies" data-testid="mobile-review-strategies">
  {stats.map((item) => {
    const returnRate = item.return_pct ?? strategyReturnRate(item.realized_pnl, initialBalance);
    return (
      <button
        aria-label={`查看 ${item.strategy_version}`}
        className={item.strategy_version === selectedVersion ? "active" : ""}
        key={`mobile-${item.strategy_name}-${item.strategy_version}`}
        onClick={() => onSelectVersion(item.strategy_version)}
        type="button"
      >
        <span>
          <strong>{item.strategy_version}</strong>
          <small>{item.strategy_name}</small>
        </span>
        <dl>
          <Metric label={copy.paper.strategyTrades} value={String(item.total_trades)} />
          <Metric label={copy.paper.winRate} value={formatNullablePct(item.win_rate)} />
          <Metric label={copy.paper.realized} value={formatSignedUsdt(item.realized_pnl)} />
          <Metric label={copy.paper.returnRate} value={formatNullablePct(returnRate)} />
        </dl>
      </button>
    );
  })}
</div>
```

- [ ] **Step 4: Add mobile attribution cards**

Inside `paper-strategy-doctor-table-wrap`, add this immediately after the existing `</table>`:

```tsx
<div className="mobile-strategy-attribution" data-testid="mobile-strategy-attribution">
  {rows.map((row) => (
    <article key={`mobile-${row.signal}`}>
      <header>
        <strong>{row.signal}</strong>
        <span className={`confidence-pill ${row.confidence}`}>{row.confidence}</span>
      </header>
      <dl>
        <Metric label={copy.paper.sampleCount} value={String(row.sampleCount)} />
        <Metric label={copy.paper.netPnl} value={formatSignedUsdt(row.netPnl)} />
        <Metric label={copy.paper.winRate} value={formatNullablePct(row.winRate)} />
        <Metric label={copy.paper.profitFactor} value={formatNullableRatio(row.profitFactor)} />
      </dl>
      <p>{copy.paper[row.recommendationKey]}</p>
    </article>
  ))}
</div>
```

Render this only in the existing non-empty branch so the current empty state remains the sole empty state.

- [ ] **Step 5: Add mobile trade cards**

After the entire existing empty/table conditional in `TradeRecordsSection`, add this independent mobile conditional:

```tsx
{trades.length > 0 ? (
  <div className="mobile-review-trades" data-testid="mobile-review-trades">
    {trades.map((trade) => (
      <article key={`mobile-${trade.id}`}>
        <header>
          <strong>{trade.inst_id}</strong>
          <span>{copy.paper.tradeActions[trade.action]} · {copy.directions[trade.side]}</span>
          <em className={pnlClass(trade.realized_pnl)}>
            {formatSignedUsdt(trade.realized_pnl)}
          </em>
        </header>
        <dl>
          <Metric label={copy.table.price} value={formatPrice(trade.price)} />
          <Metric label={copy.paper.strategyVersion} value={tradeStrategyLabel(trade)} />
          <Metric label={copy.status.lastScan} value={formatTimestamp(trade.ts_ms)} />
        </dl>
      </article>
    ))}
  </div>
) : null}
```

- [ ] **Step 6: Hide all Review mobile alternatives by default**

Extend the default selector:

```css
.mobile-radar-cards,
.mobile-position-cards,
.mobile-review-strategies,
.mobile-strategy-attribution,
.mobile-review-trades,
.mobile-sheet-backdrop,
.mobile-sheet-handle {
  display: none;
}
```

- [ ] **Step 7: Run focused and full tests**

Run:

```bash
cd frontend && npm test -- --run src/App.test.tsx -t "Review strategy and trade data"
cd frontend && npm test -- --run src/App.test.tsx
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add frontend/src/App.test.tsx frontend/src/ReviewPage.tsx frontend/src/styles.css
git commit -m "feat: add mobile review data cards"
```

### Task 5: Implement the authoritative 360px mobile responsive layer

**Files:**
- Modify: `frontend/src/styles.test.ts`
- Modify: `frontend/src/styles.css`

- [ ] **Step 1: Write failing CSS contract tests**

Append to `styles.test.ts`:

```ts
describe("mobile console adaptation", () => {
  it("defines the authoritative phone breakpoint and safe-area bottom navigation", () => {
    const mobileLayer = css.match(
      /\/\* Mobile console adaptation \*\/\s*@media \(max-width: 760px\)\s*\{(?<body>[\s\S]*)\}\s*$/u,
    )?.groups?.body;

    expect(mobileLayer).toBeTruthy();
    expect(mobileLayer).toContain(".mobile-bottom-navigation");
    expect(mobileLayer).toContain("position: fixed");
    expect(mobileLayer).toContain("grid-template-columns: repeat(5, minmax(0, 1fr))");
    expect(mobileLayer).toContain("env(safe-area-inset-bottom)");
    expect(mobileLayer).toContain(".mobile-page-content");
    expect(mobileLayer).toContain("overflow-x: hidden");
  });

  it("switches dense desktop data to mobile cards without losing local table overflow", () => {
    expect(css).toContain(".mobile-radar-cards");
    expect(css).toContain(".mobile-position-cards");
    expect(css).toContain(".mobile-review-strategies");
    expect(css).toContain(".mobile-strategy-attribution");
    expect(css).toContain(".mobile-review-trades");
    expect(css).toMatch(
      /@media \(max-width: 760px\)[\s\S]*\.monitor-page \.radar-table[\s\S]*display:\s*none/u,
    );
    expect(css).toMatch(
      /@media \(max-width: 760px\)[\s\S]*\.mobile-radar-cards[\s\S]*display:\s*grid/u,
    );
    expect(css).toMatch(
      /\.macro-table-wrap,[\s\S]*\.strategy-version-table-wrap[\s\S]*overflow-x:\s*auto/u,
    );
  });

  it("uses phone-safe touch and form sizing", () => {
    expect(css).toMatch(
      /@media \(max-width: 760px\)[\s\S]*\.task-rail-button[\s\S]*min-height:\s*60px/u,
    );
    expect(css).toMatch(
      /@media \(max-width: 760px\)[\s\S]*input,[\s\S]*select[\s\S]*font-size:\s*16px/u,
    );
    expect(css).toMatch(
      /@media \(max-width: 760px\)[\s\S]*\.paper-actions button[\s\S]*min-height:\s*44px/u,
    );
  });
});
```

- [ ] **Step 2: Run style tests and verify RED**

Run:

```bash
cd frontend && npm test -- --run src/styles.test.ts
```

Expected: FAIL because the authoritative final mobile layer does not exist.

- [ ] **Step 3: Append the complete final mobile layer**

Append this after the current `@media (max-width: 980px)` parity rule so it wins the cascade:

```css
/* Mobile console adaptation */
@media (max-width: 760px) {
  html,
  body,
  #root {
    width: 100%;
    min-width: 0;
    overflow: hidden;
  }

  .terminal-shell {
    width: 100%;
    height: 100dvh;
    grid-template-columns: minmax(0, 1fr);
    overflow: hidden;
  }

  .mobile-page-content {
    min-width: 0;
    overflow-x: hidden;
    padding-bottom: calc(64px + env(safe-area-inset-bottom));
  }

  .mobile-bottom-navigation {
    position: fixed;
    inset: auto 0 0;
    z-index: 40;
    display: block;
    width: 100%;
    min-height: 0;
    border: 0;
    border-top: 1px solid rgba(148, 174, 196, 0.12);
    background: rgba(8, 14, 24, 0.98);
    padding: 0 0 env(safe-area-inset-bottom);
  }

  .mobile-bottom-navigation .task-rail-brand,
  .mobile-bottom-navigation .task-rail-footer {
    display: none;
  }

  .mobile-bottom-navigation .task-rail-items {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 0;
    margin: 0;
  }

  .mobile-bottom-navigation .task-rail-button {
    display: flex;
    min-width: 0;
    min-height: 60px;
    flex-direction: column;
    gap: 4px;
    justify-content: center;
    border: 0;
    border-radius: 0;
    background: transparent;
    box-shadow: none;
    padding: 7px 2px 6px;
  }

  .mobile-bottom-navigation .task-rail-button.active {
    background: rgba(34, 211, 238, 0.08);
    box-shadow: inset 0 2px 0 var(--terminal-cyan);
  }

  .mobile-bottom-navigation .task-rail-button-icon {
    width: 22px;
    height: 22px;
    background: transparent;
  }

  .mobile-bottom-navigation .task-rail-button-copy {
    display: block;
    min-width: 0;
  }

  .mobile-bottom-navigation .task-rail-button-copy strong {
    display: block;
    overflow: hidden;
    font-size: 9px;
    line-height: 1;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mobile-bottom-navigation .task-rail-button-copy small {
    display: none;
  }

  .mobile-console-header {
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 8px;
    align-items: center;
    min-height: 52px;
    padding: 9px 12px 8px;
  }

  .mobile-console-header .console-page-title h1 {
    font-size: 14px;
    line-height: 1.2;
  }

  .mobile-console-header .console-page-title p {
    display: none;
  }

  .mobile-console-header .terminal-market-tape {
    max-width: 190px;
    justify-self: end;
    overflow-x: auto;
  }

  .mobile-console-header .terminal-ws-pill,
  .mobile-console-header .terminal-hot-pill {
    min-height: 28px;
    flex: 0 0 auto;
    font-size: 9px;
    white-space: nowrap;
  }

  .mobile-account-strip {
    grid-column: 1 / -1;
    display: flex;
    width: calc(100% + 24px);
    margin: 0 -12px -8px;
    gap: 18px;
    justify-content: flex-start;
    overflow-x: auto;
    border-top: 1px solid rgba(148, 174, 196, 0.06);
    padding: 7px 12px;
    scrollbar-width: none;
  }

  .mobile-account-strip span {
    flex: 0 0 auto;
    font-size: 10px;
    white-space: nowrap;
  }

  .console-main > .page-surface,
  .console-main > .macro-panel,
  .monitor-terminal {
    min-width: 0;
    padding: 12px 12px calc(20px + env(safe-area-inset-bottom));
    overflow-x: hidden;
    overflow-y: auto;
    contain: layout paint;
  }

  .figma-statbar,
  .page-metric-grid,
  .macro-summary-strip,
  .macro-overview-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }

  .figma-statbar .figma-stat-card:first-child,
  .figma-statbar .figma-stat-card:last-child {
    grid-column: 1 / -1;
  }

  .figma-stat-card,
  .metric-card {
    min-height: 72px;
    border-radius: 10px;
    padding: 10px;
  }

  .figma-radar-tabs,
  .page-local-tabs,
  .review-history-filters {
    display: flex;
    width: 100%;
    flex-wrap: nowrap;
    justify-content: flex-start;
    overflow-x: auto;
    scrollbar-width: none;
  }

  .figma-radar-tabs .toolbar-group {
    display: flex;
    min-width: max-content;
  }

  .figma-radar-tabs button,
  .page-local-tabs button,
  .page-local-tabs span {
    min-height: 38px;
    flex: 0 0 auto;
    white-space: nowrap;
  }

  .figma-radar-tabs .monitor-live-count {
    flex: 0 0 auto;
  }

  .monitor-page {
    display: block;
    min-height: auto;
    overflow: visible;
  }

  .monitor-page .radar-list-column {
    display: block;
  }

  .monitor-page .radar-table {
    display: none;
  }

  .monitor-page .radar-table-panel {
    overflow: visible;
    border: 0;
    background: transparent;
  }

  .mobile-radar-cards {
    display: grid;
    gap: 8px;
  }

  .mobile-radar-card {
    display: grid;
    gap: 10px;
    width: 100%;
    min-width: 0;
    min-height: 44px;
    border: 1px solid rgba(148, 174, 196, 0.1);
    border-radius: 12px;
    background: #0c1520;
    color: var(--text-primary);
    padding: 12px;
    text-align: left;
  }

  .mobile-radar-card.selected {
    border-color: rgba(34, 211, 238, 0.5);
    background: rgba(34, 211, 238, 0.08);
  }

  .mobile-radar-card-heading,
  .mobile-radar-card-body {
    display: flex;
    min-width: 0;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .mobile-radar-card-heading strong {
    display: flex;
    min-width: 0;
    align-items: center;
    font-size: 14px;
  }

  .mobile-radar-card-heading small {
    color: var(--terminal-muted);
    font-size: 10px;
  }

  .mobile-radar-card-body > span:first-child {
    display: grid;
    gap: 3px;
  }

  .mobile-radar-card-body b {
    font-size: 14px;
  }

  .mobile-radar-card-body em {
    font-size: 11px;
    font-style: normal;
  }

  .mobile-radar-scores {
    display: grid;
    gap: 6px;
  }

  .mobile-radar-scores > span {
    display: grid;
    grid-template-columns: 36px auto;
    align-items: center;
    gap: 5px;
  }

  .mobile-radar-scores small {
    color: var(--terminal-muted);
    font-size: 9px;
  }

  .mobile-radar-card .terminal-tag-list {
    display: flex;
    flex-wrap: wrap;
  }

  .mobile-sheet-backdrop {
    position: fixed;
    inset: 0;
    z-index: 48;
    display: block;
    width: 100%;
    height: 100%;
    border: 0;
    border-radius: 0;
    background: rgba(0, 0, 0, 0.66);
  }

  .mobile-symbol-sheet {
    position: fixed;
    inset: auto 0 0;
    z-index: 49;
    display: block;
    width: 100%;
    max-height: 86dvh;
    overflow-y: auto;
    border: 0;
    border-top: 1px solid rgba(148, 174, 196, 0.16);
    border-radius: 18px 18px 0 0;
    background: #0d1929;
    padding-bottom: env(safe-area-inset-bottom);
  }

  .mobile-sheet-handle {
    display: block;
    width: 36px;
    height: 4px;
    margin: 10px auto 2px;
    border-radius: 999px;
    background: rgba(148, 174, 196, 0.24);
  }

  .figma-symbol-detail .detail-header {
    position: sticky;
    top: 0;
    z-index: 2;
    background: #0d1929;
  }

  .figma-symbol-detail .detail-header-actions button {
    min-height: 44px;
  }

  .trade-grid,
  .review-grid,
  .strategy-workspace,
  .strategy-version-detail-grid,
  .strategy-version-panels {
    grid-template-columns: minmax(0, 1fr);
  }

  .trade-positions-panel .trade-table {
    display: none;
  }

  .mobile-position-cards {
    display: grid;
    gap: 8px;
    padding: 10px;
  }

  .mobile-position-card {
    display: grid;
    gap: 10px;
    width: 100%;
    min-height: 44px;
    border: 1px solid rgba(148, 174, 196, 0.1);
    border-radius: 12px;
    background: rgba(7, 11, 18, 0.72);
    color: var(--text-primary);
    padding: 12px;
    text-align: left;
  }

  .mobile-position-card.selected {
    border-color: rgba(34, 211, 238, 0.5);
  }

  .mobile-position-heading {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 3px 10px;
  }

  .mobile-position-heading strong {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .mobile-position-heading span {
    color: var(--terminal-muted);
    font-size: 10px;
  }

  .mobile-position-heading em {
    grid-column: 2;
    grid-row: 1 / span 2;
    align-self: center;
    font-style: normal;
  }

  .mobile-position-card dl,
  .mobile-review-strategies dl,
  .mobile-strategy-attribution dl,
  .mobile-review-trades dl {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
    margin: 0;
  }

  .mobile-position-card dt,
  .mobile-review-strategies dt,
  .mobile-strategy-attribution dt,
  .mobile-review-trades dt {
    color: var(--terminal-muted);
    font-size: 9px;
  }

  .mobile-position-card dd,
  .mobile-review-strategies dd,
  .mobile-strategy-attribution dd,
  .mobile-review-trades dd {
    margin: 2px 0 0;
    overflow-wrap: anywhere;
    font-size: 11px;
  }

  .trade-order,
  .paper-actions {
    grid-template-columns: minmax(0, 1fr);
  }

  .paper-actions button,
  .close-button,
  .review-history-search-actions button {
    min-height: 44px;
    width: 100%;
  }

  input,
  select,
  textarea {
    max-width: 100%;
    font-size: 16px;
  }

  .review-strategy-table,
  .paper-strategy-doctor-table,
  .review-trade-table {
    display: none;
  }

  .mobile-review-strategies,
  .mobile-strategy-attribution,
  .mobile-review-trades {
    display: grid;
    gap: 8px;
  }

  .mobile-review-strategies button,
  .mobile-strategy-attribution article,
  .mobile-review-trades article {
    display: grid;
    gap: 10px;
    min-height: 44px;
    border: 1px solid rgba(148, 174, 196, 0.1);
    border-radius: 12px;
    background: rgba(7, 11, 18, 0.72);
    color: var(--text-primary);
    padding: 12px;
    text-align: left;
  }

  .mobile-review-strategies button.active {
    border-color: rgba(167, 139, 250, 0.56);
    background: rgba(139, 92, 246, 0.1);
  }

  .mobile-review-strategies button > span,
  .mobile-strategy-attribution header,
  .mobile-review-trades header {
    display: flex;
    min-width: 0;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .mobile-review-strategies small {
    overflow: hidden;
    color: var(--terminal-muted);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mobile-strategy-attribution p {
    margin: 0;
    color: var(--terminal-muted);
    font-size: 11px;
  }

  .mobile-review-trades header {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
  }

  .mobile-review-trades header span {
    color: var(--terminal-muted);
    font-size: 10px;
  }

  .mobile-review-trades header em {
    grid-column: 2;
    grid-row: 1 / span 2;
    font-style: normal;
  }

  .review-history-filter-grid,
  .paper-strategy-curve-metrics,
  .macro-overview-grid,
  .macro-columns,
  .macro-chart-grid,
  .macro-band-grid,
  .macro-permission-grid,
  .macro-permission-detail-grid,
  .macro-cohort-grid,
  .macro-kline-grid,
  .macro-analog-list {
    grid-template-columns: minmax(0, 1fr);
  }

  .review-history-search-row,
  .review-history-search-actions,
  .macro-section-header,
  .macro-table-toolbar,
  .macro-ahr-range-toolbar,
  .macro-ahr-range-inputs {
    display: flex;
    flex-direction: column;
    align-items: stretch;
  }

  .review-history-list li {
    border-radius: 12px;
    padding: 12px;
  }

  .review-history-list dl {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .macro-table-wrap,
  .strategy-version-table-wrap,
  .strategy-version-detail,
  .review-strategy-table-wrap,
  .paper-strategy-doctor-table-wrap {
    max-width: 100%;
    overflow-x: auto;
  }

  .macro-data-table,
  .strategy-version-table,
  .strategy-version-detail table {
    min-width: 720px;
  }

  .pagination-controls {
    flex-wrap: wrap;
  }

  .pagination-controls button {
    min-height: 40px;
  }
}
```

- [ ] **Step 4: Run style tests and verify GREEN**

Run:

```bash
cd frontend && npm test -- --run src/styles.test.ts
```

Expected: all style tests PASS.

- [ ] **Step 5: Run all frontend tests and type checks**

Run:

```bash
cd frontend && npm test
cd frontend && npm run lint
```

Expected: all tests PASS and both TypeScript checks exit 0.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/styles.test.ts frontend/src/styles.css
git commit -m "feat: adapt console layout for phone screens"
```

### Task 6: Verify real phone viewports and desktop regression

**Files:**
- Modify only if a failing visual check is first captured by an automated style or component test.

- [ ] **Step 1: Start the production frontend against the local backend**

Run:

```bash
cd frontend && npm run dev
```

Expected: Vite reports `http://127.0.0.1:5173`. If the backend is not running, the existing empty and disconnected states are still valid for layout checks; use the test fixture path only if the repository already provides one.

- [ ] **Step 2: Verify 360x800**

Open `http://127.0.0.1:5173` at 360x800 and verify:

- the fixed bottom bar shows exactly five items and does not cover the last card;
- Radar summary is two columns with full-width priority cards;
- Radar filters scroll inside their row;
- symbol cards fit without document-level horizontal scroll;
- selecting a symbol opens a scrollable bottom sheet;
- close button, backdrop, and Escape close the sheet;
- TradingView and paper-trade actions remain reachable.

Expected: `document.documentElement.scrollWidth === document.documentElement.clientWidth`.

- [ ] **Step 3: Verify 390x844 and 430x932**

At each viewport, navigate through Radar, Macro, Strategy, Paper Trading, and Review. Verify:

- charts remain within their cards;
- Macro and Strategy dense tables scroll only inside their panel;
- paper order inputs do not trigger layout zoom and action buttons remain 44px or taller;
- position cards select the existing position detail;
- Review version, attribution, history, and trade cards remain readable;
- loading, empty, disconnected, and error text does not overflow.

Expected: all five pages usable with no document-level horizontal overflow.

- [ ] **Step 4: Verify 768x1024 and desktop**

At 768x1024 and 1440x900, verify the desktop table presentations are visible, mobile-only cards are hidden, the existing left/task navigation behavior is unchanged above 760px, and all current desktop actions still work.

Expected: no visual or interaction regression from the current desktop console.

- [ ] **Step 5: If a visual check fails, return to RED**

Add the smallest failing assertion to `App.test.tsx` or `styles.test.ts` that reproduces the issue, run it to confirm RED, make the minimal CSS or markup fix, rerun the focused test, and repeat the affected viewport check. Do not apply untested visual fixes.

- [ ] **Step 6: Stop the dev server**

Terminate the Vite process cleanly after verification.

### Task 7: Final verification and delivery

**Files:**
- Verify all files changed by Tasks 1–6.

- [ ] **Step 1: Run the full automated suite**

Run:

```bash
cd frontend && npm test
```

Expected: all Vitest suites PASS with no unhandled errors.

- [ ] **Step 2: Run type checks and production build**

Run:

```bash
cd frontend && npm run lint
cd frontend && npm run build
```

Expected: TypeScript checks exit 0 and Vite creates `frontend/dist` successfully.

- [ ] **Step 3: Review the exact diff**

Run:

```bash
git diff --check
git status --short
git diff --stat HEAD~5..HEAD
```

Expected: no whitespace errors; only the planned frontend, test, spec, and plan files are changed or committed. The visual-companion `.superpowers/` session is not staged.

- [ ] **Step 4: Record final evidence**

Report:

- test, lint, and build command results;
- phone viewports checked;
- desktop viewport checked;
- changed component list;
- any remaining non-blocking limitations.
