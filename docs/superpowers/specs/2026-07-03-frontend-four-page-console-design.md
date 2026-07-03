# Frontend Four-Page Console Design

Date: 2026-07-03

## Goal

Reduce visual clutter in the AlphaPulse OKX frontend by turning the current mixed radar, paper-trading, review, and macro content into four clear top-level work areas:

- Monitor
- Trade
- Review
- Macro

The current page already exposes the needed data, but too many controls and datasets share the same hierarchy. The redesign should make the top-level navigation explain the user's intent first, then show page-specific controls only inside the relevant page.

## Approved Direction

Use a complete four-page console with a persistent left task rail.

Top-level navigation contains only:

- `监控` / `Monitor`
- `交易` / `Trade`
- `复盘` / `Review`
- `宏观` / `Macro`

The existing radar filter buttons:

- `全部`
- `趋势`
- `震荡`
- `热门`
- `固定`

belong only inside the Monitor page. They should no longer appear as global-level navigation.

The user selected the Trade page direction where position and order management is the primary job: Trade is centered on all current positions and account/order management, not only the currently selected monitor symbol.

## Reference Inputs

Inputs used for this design:

- User screenshots of the current local frontend at `127.0.0.1:5173`.
- Visual brainstorming companion mockups:
  - `C. 任务分栏`
  - `C1. 左侧常驻窄栏`
  - Complete four-page preview.
- Existing frontend code and prior design docs:
  - `frontend/src/ConsoleShell.tsx`
  - `frontend/src/RadarWorkspace.tsx`
  - `frontend/src/RadarTable.tsx`
  - `frontend/src/SymbolDetailPanel.tsx`
  - `frontend/src/MacroPanel.tsx`
  - `docs/superpowers/specs/2026-07-01-frontend-console-redesign.md`

No Figma node URL was provided in this session, so this spec is based on local code, screenshots, and the approved browser companion mockups. If a Figma frame is provided later, use the Figma MCP flow to validate or adjust visual details against that frame.

## Product Scope

In scope:

- Replace the current top header view switcher with a persistent task rail.
- Promote `监控 / 交易 / 复盘 / 宏观` to the only top-level navigation choices.
- Move current radar filters into the Monitor page as local controls.
- Split existing paper-trading content into dedicated Trade and Review pages.
- Keep Monitor focused on scanning opportunities and inspecting the selected symbol.
- Keep Macro as a top-level page using the existing macro panel content.
- Reuse existing backend API contracts and frontend state where possible.
- Use the existing paper snapshot for the first implementation; richer historical-position or strategy-version datasets require a separate backend scope if they are not already present.
- Preserve current language, theme, notification, TradingView, chart, and paper-trading behavior.
- Add focused frontend tests for navigation, page-specific controls, and preserved trading interactions.

Out of scope:

- New backend APIs.
- Real OKX account trading.
- New trading logic, risk engine, or order types.
- A new UI component library.
- Pixel-perfect Figma implementation unless a Figma URL is provided later.

## Information Architecture

The application has two navigation levels.

Top level:

- Monitor: live radar scanning, opportunity filters, selected-symbol context, TradingView entry.
- Trade: all current paper positions, account metrics, quick order entry, close-position actions.
- Review: paper account performance, equity/PnL summary, trade history, and any available strategy/version history.
- Macro: BTC macro cycle, market permission, valuation metrics, historical analogs, AHR999, and macro charts.

Local page controls:

- Monitor owns radar filters: all, trend, range, hot, fixed.
- Trade can have local tabs or sections for current positions, quick order, and recent trades.
- Review can have local sections for overview, trade records, and available history or strategy versions.
- Macro can keep its existing internal sections and chart controls.

This prevents local filters from competing with global navigation.

## Layout Design

Desktop shell:

- A narrow left rail contains the product mark and four top-level task buttons.
- The top status bar remains inside the main content area and shows backend, stream, notification, last scan, symbol count, theme, language, and notification actions.
- The main content area changes by selected page.
- The dark trading-terminal palette remains the primary design target.

Mobile and tablet:

- The left rail collapses into a compact top or bottom task bar.
- Page-local controls wrap below the status bar.
- Tables keep horizontal overflow when needed.
- Detail panels stack below primary content.

## Page Designs

### Monitor

Purpose: identify which contract deserves attention now.

Content:

- Local radar filter bar: all, trend, range, hot, fixed.
- Dense symbol table as the primary area.
- Selected-symbol context panel with:
  - contract header and trigger reason
  - short-horizon metrics
  - chart and FVG controls
  - structure details
  - TradingView action
  - a clear route to Trade for the selected symbol

Monitor should not show account-wide review statistics, historical positions, or full trade history.

### Trade

Purpose: manage all current paper positions and orders.

Content:

- Account summary: equity, available balance, used margin, unrealized PnL.
- Current positions table as the primary area.
- Quick order panel:
  - selected or manually chosen contract
  - margin
  - leverage
  - open long
  - open short
- Selected-position detail:
  - side
  - entry price
  - mark price
  - quantity
  - notional
  - unrealized PnL and percentage
  - close position action
- Recent trades can appear as a secondary section, not the main focus.

When the user opens Trade from Monitor, the quick order panel should preload the selected Monitor symbol. Trade still remains account-centered and should show all current positions.

### Review

Purpose: evaluate strategy and paper-trading results.

Content:

- Summary metrics:
  - initial balance
  - equity
  - realized PnL
  - unrealized PnL
  - win rate
  - average win
  - average loss
  - maximum win
  - maximum loss
  - profit factor
  - cumulative fees
- Realized PnL curve.
- Strategy version comparison table when versioned result rows are available.
- Historical positions with filters for symbol, date range, and strategy version when those fields are available.
- Trade record list.
- Reset paper account action, visually separated from routine controls.

Review is where paper-trading performance, history, and trade records should move. In the first implementation, Review should use the existing paper account snapshot and show clear empty or unavailable states for data the backend does not currently expose.

### Macro

Purpose: inspect BTC macro context and market permission.

Content:

- Existing `MacroPanel` content remains available.
- Macro is a top-level task rail item, not a secondary view switcher button.
- Macro may have local sections for cycle, valuation, analogs, and AHR999, but those are inside the Macro page only.

## Component Architecture

Refactor toward these boundaries:

- `ConsoleShell`
  - owns the application frame, left task rail, status bar, theme/language/notification actions, and content slot.
- `TaskRail`
  - renders the four top-level navigation items.
- `StatusBar`
  - renders connection, stream, notification, scan, symbol count, theme, language, and notification controls.
- `MonitorPage`
  - owns radar filters, symbol table, selected-symbol monitor detail, and Monitor-specific actions.
- `TradePage`
  - owns account summary, all current positions, quick order, selected-position detail, and close-position workflow.
- `ReviewPage`
  - owns account performance summaries, realized PnL presentation, trade records, and any available history or strategy-version comparisons.
- `MacroPage` or existing `MacroPanel`
  - owns full macro analysis.
- Shared presentational components:
  - metric cards
  - local page tabs/filter bars
  - paper position table rows
  - trade history rows
  - empty states

`App` continues to own application state and API interactions unless a small extraction improves clarity without changing behavior.

## Data Flow

`App` continues to own:

- dashboard snapshot
- macro snapshot
- backend and stream status
- active top-level page
- Monitor filter
- selected symbol
- TradingView modal state
- paper order form state
- trade busy/error state
- theme, language, and notification permission

Derived data stays memoized:

- sorted symbols
- filtered Monitor symbols
- selected Monitor symbol
- current positions
- selected position
- review metrics derived from paper positions and trades

No component should duplicate macro or dashboard fetching.

## Interaction Design

Top-level navigation:

- Clicking a task rail item switches the main page.
- The active task rail item has a clear blue active state.
- The highest-level navigation never shows radar filters.

Monitor:

- Clicking a radar row selects that symbol.
- Clicking TradingView opens the modal without changing selection.
- Clicking a Trade action switches to Trade and preloads the selected symbol in quick order.

Trade:

- Position rows select a position for detail.
- Quick order can use the preloaded Monitor symbol or the selected Trade symbol.
- Close-position action applies to the selected position.
- Order errors remain near order controls.

Review:

- Historical filters affect only Review content.
- Reset paper account is visually separated and should not be confused with normal page filters.

Macro:

- Existing refresh, loading, error, and chart interactions are preserved.

## Visual Design

Use the existing dark terminal palette:

- app background: `#0b0e14`
- primary surface: `#151924`
- border: `#2b313f`
- accent: blue
- positive: emerald
- negative: rose

Visual rules:

- The left task rail is compact and utilitarian.
- Page titles and local controls should make the active task obvious.
- Cards use restrained borders and 8px radius.
- Avoid nested cards where a simple section or table is clearer.
- Tables should carry dense information; cards should summarize state or isolate actions.
- Do not make radar filters look like top-level tabs.

## Error Handling

Dashboard fetch failure:

- Preserve disconnected status behavior.
- Monitor, Trade, and Review show empty or unavailable states without layout breakage.

Macro fetch failure:

- Macro shows existing error and refresh action.
- Other pages remain usable.

Paper order failure:

- Trade shows the order error near quick order controls.
- Busy state disables relevant order and close buttons.

Missing selected symbol or position:

- Monitor shows an empty selected-symbol state when no symbols exist.
- Trade shows no-position guidance when no current positions exist.
- Review still shows account-level metrics and empty history states.

## Accessibility

- Task rail uses a labeled navigation region.
- Active page is represented with `aria-current="page"`.
- Local filters remain button groups with accessible labels.
- Tables retain semantic table markup where possible.
- TradingView modal keeps `role="dialog"` and `aria-modal="true"`.
- Color is not the only status signal: direction labels, PnL signs, and text remain visible.
- Focus states must remain visible against the dark palette.

## Testing Plan

Frontend tests should cover:

- Only four top-level navigation items are present in the task rail.
- Monitor page displays radar filters and the symbol table.
- Radar filters are not visible on Trade, Review, or Macro.
- Switching from Monitor to Trade can preload the selected symbol for quick order.
- Trade page shows all current positions and account summary.
- Opening and closing paper positions still calls the existing APIs.
- Review page shows strategy comparison, realized PnL summary, historical positions, and trade records when data exists.
- Macro page still loads, refreshes, and displays macro content.
- TradingView modal behavior is preserved.
- Mobile/responsive CSS keeps the task rail and local controls from overlapping.

## Implementation Notes

The implementation should be staged:

1. Introduce the four-page shell and task rail while keeping existing data flow intact.
2. Move Monitor-specific filters and selected-symbol monitor detail into `MonitorPage`.
3. Extract Trade page from current `SymbolDetailPanel` paper-trading controls and paper account data.
4. Extract Review page from account summaries, realized PnL, current trade records, and any history/version data already available in the snapshot.
5. Rewire Macro into the task rail as the Macro page.
6. Polish responsive layout and update tests.

This is larger than a visual-only CSS cleanup. The main risk is accidentally changing paper-trading behavior while moving UI. Keep API calls and state transitions in `App` until tests prove the new page boundaries are stable.
