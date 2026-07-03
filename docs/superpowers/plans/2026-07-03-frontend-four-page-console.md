# Frontend Four-Page Console Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the approved four-page AlphaPulse OKX console with top-level Monitor, Trade, Review, and Macro work areas.

**Architecture:** Keep `App` as the owner of API state and trading actions, then split UI into task-level pages. `ConsoleShell` becomes the frame with a left task rail and top status bar. Monitor keeps radar filters and selected-symbol context; Trade owns all current paper positions and quick order; Review owns account performance and trade history; Macro reuses the existing macro panel.

**Tech Stack:** React 18, TypeScript, Vite, Vitest, Testing Library, CSS variables, existing `lightweight-charts`.

---

## Scope Check

This plan implements one subsystem: the React frontend. It does not add backend APIs. Review uses the existing `PaperAccountSnapshot` fields (`initial_balance`, `realized_pnl`, `unrealized_pnl`, `equity`, `used_margin`, `available_balance`, `positions`, `trades`). Versioned strategy comparisons and full historical positions render as empty/unavailable states until backend data exists.

## File Structure

- Modify `frontend/src/uiFormat.ts`: extend `ViewMode` to `monitor | trade | review | macro`; add paper review metric helpers.
- Modify `frontend/src/i18n.ts`: add labels for four pages, task rail, Trade, and Review.
- Modify `frontend/src/ConsoleShell.tsx`: render left task rail, page title/status bar, existing theme/language/notification controls.
- Create `frontend/src/MonitorPage.tsx`: Monitor page with local radar filters, optional macro summary, radar table, and monitor-only symbol detail.
- Modify `frontend/src/SymbolDetailPanel.tsx`: remove paper-trading responsibilities so it becomes monitor context only, with a route-to-Trade action.
- Create `frontend/src/TradePage.tsx`: account summary, all positions table, quick order panel, selected position detail, and recent trades.
- Create `frontend/src/ReviewPage.tsx`: paper performance metrics, realized PnL curve, trade records, and empty states for unavailable version/history datasets.
- Modify `frontend/src/App.tsx`: wire new `ViewMode`, page routing, selected symbol/position behavior, and existing trading actions.
- Modify `frontend/src/styles.css`: add task rail, page shell, Monitor/Trade/Review layouts, and responsive behavior.
- Modify `frontend/src/App.test.tsx`: test top-level nav, page-local filters, Trade, Review, Macro, and preserved paper order flow.
- Modify `frontend/src/styles.test.ts`: assert task rail and four-page responsive selectors.

## Task 1: Add Failing Four-Page Console Tests

**Files:**
- Modify: `frontend/src/App.test.tsx`

- [ ] **Step 1: Add test data with current positions and closed trades**

Inside `frontend/src/App.test.tsx`, after the `paper` constant, add:

```tsx
const activePaper: PaperAccountSnapshot = {
  ...paper,
  realized_pnl: 207.58,
  unrealized_pnl: 42.86,
  equity: 10250.44,
  used_margin: 459.44,
  available_balance: 9791,
  positions: [
    {
      inst_id: "LAB-USDT-SWAP",
      side: "long",
      qty: 58.1395348837,
      entry_price: 17.2,
      mark_price: 17.9,
      margin: 100,
      leverage: 10,
      notional: 1000,
      unrealized_pnl: 40.7,
      pnl_pct: 0.407,
      opened_at_ms: 1782400000000,
    },
    {
      inst_id: "DOGE-USDT-SWAP",
      side: "short",
      qty: 1000,
      entry_price: 0.18,
      mark_price: 0.17784,
      margin: 180,
      leverage: 1,
      notional: 180,
      unrealized_pnl: 2.16,
      pnl_pct: 0.012,
      opened_at_ms: 1782400000000,
    },
  ],
  trades: [
    {
      id: 3,
      inst_id: "BREV-USDT-SWAP",
      side: "short",
      action: "close",
      price: 0.083787,
      qty: 2000,
      margin: 100,
      notional: 167.57,
      realized_pnl: 236.94,
      ts_ms: 1782400000000,
    },
    {
      id: 2,
      inst_id: "NES-USDT-SWAP",
      side: "short",
      action: "close",
      price: 0.218644,
      qty: 1000,
      margin: 100,
      notional: 218.64,
      realized_pnl: -58.22,
      ts_ms: 1782396400000,
    },
    {
      id: 1,
      inst_id: "LAB-USDT-SWAP",
      side: "long",
      action: "open",
      price: 17.2,
      qty: 58.1395348837,
      margin: 100,
      notional: 1000,
      realized_pnl: 0,
      ts_ms: 1782392800000,
    },
  ],
};
```

- [ ] **Step 2: Add navigation and page-local filter tests**

Append these tests inside the existing `describe("App", () => { })` block:

```tsx
  it("uses four top-level task rail pages and keeps radar filters inside Monitor", async () => {
    mockSnapshot({ ...snapshot, paper: activePaper });

    render(<App />);
    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);

    const taskRail = screen.getByRole("navigation", { name: "主导航" });
    expect(taskRail).toHaveTextContent("监控");
    expect(taskRail).toHaveTextContent("交易");
    expect(taskRail).toHaveTextContent("复盘");
    expect(taskRail).toHaveTextContent("宏观");
    expect(screen.getByRole("button", { name: "监控" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("button", { name: "趋势" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "交易" }));

    expect(screen.getByRole("heading", { name: "交易" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "趋势" })).not.toBeInTheDocument();
    expect(screen.getByText("当前持仓")).toBeInTheDocument();
  });
```

- [ ] **Step 3: Add Trade page behavior test**

Append:

```tsx
  it("shows all current positions on the Trade page and preloads the selected Monitor symbol", async () => {
    mockSnapshot({ ...snapshot, paper: activePaper });

    render(<App />);
    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("row", { name: /DOGE-USDT-SWAP/ }));
    fireEvent.click(screen.getByRole("button", { name: "去交易" }));

    expect(screen.getByRole("heading", { name: "交易" })).toBeInTheDocument();
    expect(screen.getByTestId("trade-page")).toHaveTextContent("LAB-USDT-SWAP");
    expect(screen.getByTestId("trade-page")).toHaveTextContent("DOGE-USDT-SWAP");
    expect(screen.getByLabelText("交易合约")).toHaveValue("DOGE-USDT-SWAP");
    expect(screen.getByText("全部当前持仓")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "模拟卖出 / 开空" })).toBeInTheDocument();
  });
```

- [ ] **Step 4: Add Review page behavior test**

Append:

```tsx
  it("shows Review performance and trade records without Monitor filters", async () => {
    mockSnapshot({ ...snapshot, paper: activePaper });

    render(<App />);
    expect((await screen.findAllByText("LAB-USDT-SWAP")).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: "复盘" }));

    expect(screen.getByRole("heading", { name: "复盘" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "趋势" })).not.toBeInTheDocument();
    expect(screen.getByTestId("review-page")).toHaveTextContent("已实现盈亏");
    expect(screen.getByTestId("review-page")).toHaveTextContent("+207.58 USDT");
    expect(screen.getByTestId("review-page")).toHaveTextContent("胜率");
    expect(screen.getByTestId("review-page")).toHaveTextContent("50.00%");
    expect(screen.getByText("策略版本数据暂不可用")).toBeInTheDocument();
    expect(screen.getByText(/BREV-USDT-SWAP/)).toBeInTheDocument();
  });
```

- [ ] **Step 5: Update Macro navigation assertions**

In existing tests, replace clicks that use `screen.getByRole("button", { name: "大周期" })` with `screen.getByRole("button", { name: "宏观" })`.

- [ ] **Step 6: Run App tests and verify the new tests fail**

Run:

```bash
cd frontend
npm test -- App.test.tsx
```

Expected: FAIL because `主导航`, `交易`, `复盘`, `去交易`, `trade-page`, and `review-page` are not implemented yet.

## Task 2: Implement Four-Page Types, Copy, and Shell

**Files:**
- Modify: `frontend/src/uiFormat.ts`
- Modify: `frontend/src/i18n.ts`
- Modify: `frontend/src/ConsoleShell.tsx`
- Modify: `frontend/src/styles.css`

- [ ] **Step 1: Extend view mode**

Change `frontend/src/uiFormat.ts`:

```ts
export type ViewMode = "monitor" | "trade" | "review" | "macro";
```

- [ ] **Step 2: Add page copy**

Add these keys to both `zh` and `en` in `frontend/src/i18n.ts`.

Chinese:

```ts
views: {
  monitor: "监控",
  trade: "交易",
  review: "复盘",
  macro: "宏观",
},
pageDescriptions: {
  monitor: "实时雷达、机会筛选和选中合约上下文",
  trade: "全部当前持仓、快速下单和风险占用",
  review: "模拟盘表现、收益曲线和成交记录",
  macro: "BTC 周期、估值和市场许可",
},
```

English:

```ts
views: {
  monitor: "Monitor",
  trade: "Trade",
  review: "Review",
  macro: "Macro",
},
pageDescriptions: {
  monitor: "Live radar, opportunity filters, and selected-symbol context",
  trade: "All current positions, quick order, and margin usage",
  review: "Paper performance, PnL curve, and trade records",
  macro: "BTC cycle, valuation, and market permission",
},
```

- [ ] **Step 3: Refactor `ConsoleShell`**

Replace the top view switcher in `frontend/src/ConsoleShell.tsx` with a left task rail and top status bar. Keep existing theme, language, and notification controls. The four rail buttons should call `onViewModeChange` with `monitor`, `trade`, `review`, and `macro`; the active one must set `aria-current="page"`.

- [ ] **Step 4: Add shell CSS**

Add CSS for:

```css
.task-console-shell
.task-rail
.task-rail-button
.console-main
.console-topbar
.console-page-title
```

Expected layout: desktop uses a narrow left rail and a flexible main area; below `960px`, the rail becomes a horizontal bar.

- [ ] **Step 5: Run focused tests**

Run:

```bash
cd frontend
npm test -- App.test.tsx
```

Expected: still FAIL because pages are not wired, but failures related to missing `主导航` should be resolved.

## Task 3: Implement Monitor Page

**Files:**
- Create: `frontend/src/MonitorPage.tsx`
- Modify: `frontend/src/SymbolDetailPanel.tsx`
- Modify: `frontend/src/App.tsx`
- Modify: `frontend/src/styles.css`

- [ ] **Step 1: Create `MonitorPage`**

Create a page component that renders:

- `MacroSummaryStrip`
- local radar filter bar
- `RadarTable`
- monitor-only `SymbolDetailPanel`

Props should include the existing filtered symbols, filter state, selected symbol, macro summary state, theme mode, and callbacks for selecting symbols, opening TradingView, and switching selected symbol to Trade.

- [ ] **Step 2: Make `SymbolDetailPanel` monitor-only**

Remove account-wide paper metrics, quick order, position, and trade history JSX from `SymbolDetailPanel`. Add a `onTradeSymbol` prop and render a `去交易` / `Trade` button near the TradingView action.

- [ ] **Step 3: Wire Monitor in `App`**

Set initial `viewMode` to `"monitor"` and render `MonitorPage` when active. Update TradingView modal rendering to use `viewMode === "monitor"`.

- [ ] **Step 4: Run focused tests**

Run:

```bash
cd frontend
npm test -- App.test.tsx
```

Expected: Monitor tests pass. Trade and Review tests still fail until their pages exist.

## Task 4: Implement Trade Page

**Files:**
- Create: `frontend/src/TradePage.tsx`
- Modify: `frontend/src/App.tsx`
- Modify: `frontend/src/i18n.ts`
- Modify: `frontend/src/styles.css`

- [ ] **Step 1: Add Trade copy**

Add labels for account summary, all current positions, quick order, selected position, trading contract, and no-position states.

- [ ] **Step 2: Create `TradePage`**

Render:

- account summary cards
- all current positions table
- quick order form using `orderMargin` and `orderLeverage`
- selected position detail
- close-position action for the selected position
- recent trades list

The quick order contract input should be accessible with `aria-label={copy.trade.orderInstrument}` and use the selected Monitor symbol when navigating from Monitor to Trade.

- [ ] **Step 3: Update trading callbacks in `App`**

Change paper order and close helpers to accept an optional instrument id:

```ts
async function submitPaperOrder(side: PaperSide, instId = selected?.inst_id) {
  const target =
    snapshot.symbols.find((symbol) => symbol.inst_id === instId) ??
    selected ??
    null;
  if (!target) {
    return;
  }

  const margin = Number(orderMargin);
  const leverage = Number(orderLeverage);
  setTradeBusy(true);
  setTradeError(null);
  try {
    const paper = await openPaperOrder({
      inst_id: target.inst_id,
      side,
      margin,
      leverage,
    });
    setSnapshot((current) => ({ ...current, paper }));
  } catch (error) {
    setTradeError(error instanceof Error ? error.message : String(error));
  } finally {
    setTradeBusy(false);
  }
}

async function submitPaperClose(instId = selected?.inst_id) {
  if (!instId) {
    return;
  }

  setTradeBusy(true);
  setTradeError(null);
  try {
    const paper = await closePaperPosition(instId);
    setSnapshot((current) => ({ ...current, paper }));
  } catch (error) {
    setTradeError(error instanceof Error ? error.message : String(error));
  } finally {
    setTradeBusy(false);
  }
}
```

Use the provided `instId` when opening or closing from Trade.

- [ ] **Step 4: Run focused tests**

Run:

```bash
cd frontend
npm test -- App.test.tsx
```

Expected: Trade tests and existing paper order test pass. Review tests still fail until Review exists.

## Task 5: Implement Review Page

**Files:**
- Create: `frontend/src/ReviewPage.tsx`
- Modify: `frontend/src/App.tsx`
- Modify: `frontend/src/uiFormat.ts`
- Modify: `frontend/src/i18n.ts`
- Modify: `frontend/src/styles.css`

- [ ] **Step 1: Add review metric helper**

Add a helper that computes closed-trade count, win rate, average win, average loss, max win, max loss, and profit factor from `PaperAccountSnapshot`.

- [ ] **Step 2: Add Review copy**

Add labels for performance, realized PnL curve, win rate, average win, average loss, max win, max loss, profit factor, trade records, and unavailable strategy/history data.

- [ ] **Step 3: Create `ReviewPage`**

Render:

- account performance cards
- a simple SVG realized PnL curve from close trades
- recent trade record list
- empty state for strategy/history data not currently exposed by the backend

- [ ] **Step 4: Run focused tests**

Run:

```bash
cd frontend
npm test -- App.test.tsx
```

Expected: App tests pass.

## Task 6: Polish CSS, Style Tests, and Full Verification

**Files:**
- Modify: `frontend/src/styles.css`
- Modify: `frontend/src/styles.test.ts`

- [ ] **Step 1: Update style tests**

Update `frontend/src/styles.test.ts` to assert these selectors exist:

```ts
expect(css).toContain(".task-rail");
expect(css).toContain(".task-rail-button");
expect(css).toContain(".trade-page");
expect(css).toContain(".review-page");
```

- [ ] **Step 2: Run frontend verification**

Run:

```bash
cd frontend
npm run lint
npm test
npm run build
```

Expected: all commands pass.

- [ ] **Step 3: Run repository status check**

Run:

```bash
git status --short
```

Expected: only intended frontend files and the plan are modified.
