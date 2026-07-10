# AlphaPulse OKX Frontend Mobile Adaptation Design

## Context

The AlphaPulse OKX frontend is a five-page React/Vite trading console with Radar, Macro, Strategy, Paper Trading, and Review views. Its current terminal layout is optimized for desktop: a fixed left task rail, a multi-column top bar, dense tables, and desktop-width detail panels. Existing responsive rules partially stack grids, but later desktop parity styles override several of those rules. At phone widths this leaves navigation, fixed-height scrolling, tables, and detail panels difficult to use.

The mobile layout will use `/Users/c3b6/Downloads/监控系统与策略模块 (1)` as its structural reference. That reference already demonstrates the approved mobile information architecture: a compact status header, a horizontally scrollable account strip, bottom navigation, two-column summary cards, touch-friendly data cards, and a bottom detail sheet. The production app will reuse those patterns without copying the reference's mock data or replacing the production data and state flows.

## Goals

- Fully adapt all five frontend pages for phone screens from 360px wide.
- Keep the current desktop appearance and interactions unchanged above the mobile breakpoint.
- Preserve all current API, WebSocket, notification, strategy-control, paper-order, and chart behavior.
- Prevent document-level horizontal overflow and content being obscured by fixed navigation.
- Make primary mobile controls usable with one hand and touch targets approximately 44px high.
- Preserve access to dense data through mobile summaries, detail sheets, or component-local horizontal scrolling.

## Non-goals

- Redesigning the desktop console.
- Changing backend contracts, trading behavior, scoring, filtering, or persistence.
- Replacing the existing theme system or introducing another component framework.
- Recreating every wide table as a bespoke mobile component when local horizontal scrolling remains usable.
- Supporting viewports narrower than 360px as a formal acceptance target.

## Responsive Architecture

The mobile layout activates at `max-width: 760px`. Desktop markup and styles remain the default. Mobile variants share the same props and source data as their desktop equivalents and are switched through CSS rather than JavaScript viewport detection. This keeps state, event handlers, and real-time updates single-sourced.

The shell becomes a three-part mobile frame:

1. A compact header containing the active page identity, connection state, and the most important live-market alert.
2. A horizontally scrollable quick-status strip for BTC price, positions, signals, and unrealized profit and loss.
3. A fixed five-item bottom navigation for Radar, Macro, Strategy, Paper Trading, and Review. It observes `env(safe-area-inset-bottom)` and exposes the active view with `aria-current="page"`.

The active page owns vertical scrolling. Content receives bottom padding equal to the navigation height plus the safe-area inset. The document itself must not scroll horizontally. Filter and tab rows may scroll horizontally inside their own bounds.

## Shared Mobile Presentation

- Page padding becomes 12px to 16px.
- Summary grids use two equal columns; a primary metric may span both columns.
- Cards use the existing terminal palette with the reference layout's rounded corners, spacing, and hierarchy.
- Inputs use at least 16px text to prevent iOS focus zoom.
- Primary buttons and navigation targets are approximately 44px high.
- Charts use their existing responsive containers and phone-appropriate fixed minimum heights.
- Tables that remain tables live inside an explicit scroll container with `overflow-x: auto` and no effect on document width.
- Empty, loading, and error states keep their existing content and state sources.

## Page Designs

### Radar

The current five-card statistics row becomes a two-column mobile grid. The highest-priority BTC/market metric may span both columns, following the reference app's emphasis pattern. Radar filters form a horizontally scrollable segmented row.

The desktop radar table remains unchanged. Mobile renders a signal-card list from the same `SymbolSnapshot[]`. Each card shows:

- position indicator when the symbol has an open paper position;
- shortened instrument name;
- signal direction;
- current price and one-hour change;
- Trend and Range scores with compact visual bars;
- up to four pool or signal tags.

Selecting a card opens the existing symbol detail content in a mobile bottom sheet. The sheet has a dimmed backdrop, drag-handle decoration, explicit close button, a maximum height around 86vh, internal scrolling, dialog semantics, backdrop close, and Escape close. Existing TradingView and paper-trade actions remain available inside it.

### Macro

Macro summary and analysis grids become one or two columns depending on available phone width. Charts become full width. Permission, regime, cohort, K-line, and event content stacks vertically. Date-range, pagination, and table toolbar controls stack when they cannot fit on one row.

Large historical and comparison tables remain semantic tables inside component-local horizontal scroll regions. Sticky headers may remain where they do not interfere with vertical page scrolling.

### Strategy

The active strategy becomes the primary status card, with compact key metrics arranged in two or three columns where labels remain readable. Version selectors become horizontally scrollable chips. Signal attribution, feature hits, and Shadow positions become stacked cards or existing single-column panels.

Strategy start, stop, and reset actions remain wired to the existing handlers. Version comparison and configuration tables stay in local horizontal scroll containers. Details and controls stack into one column.

### Paper Trading

Account metrics use a two-column summary grid. The desktop positions table remains unchanged; mobile renders position cards from the same `PaperPositionSnapshot[]`. Cards show instrument, side, leverage, entry, mark, profit and loss, and primary risk or signal context. Selecting a card updates the existing selected-position state.

The quick-order form and selected-position detail appear below the position list. Labels, inputs, buy/sell buttons, and close-position action use the full available width where appropriate. Existing busy, disabled, and error behavior remains unchanged. Recent trades use compact list rows or cards without dropping information required to identify the action and result.

### Review

Review summary metrics use a two-column grid. Version selectors and range controls scroll horizontally when needed. The selected version summary and profit curve become full-width cards.

Signal attribution uses summary cards inspired by the reference app, showing sample count, net profit and loss, win rate, profit factor, confidence, and suggestion. Position history and trade history gain mobile summary-card presentations driven by the same filtered data used by desktop tables. More complex strategy-doctor and comparison tables remain horizontally scrollable within their panels.

## Component Boundaries

- `ConsoleShell.tsx` owns the shared mobile header, quick-status strip, bottom navigation, and safe-area shell structure.
- `RadarTable.tsx` owns desktop radar table rendering and mobile radar card rendering from identical props.
- `MonitorPage.tsx` owns selection and mobile bottom-sheet placement for symbol details.
- `TradePage.tsx` owns desktop position table rendering and mobile position cards from identical props.
- `ReviewPage.tsx` owns mobile review/history cards where the desktop table is too dense for phone scanning.
- `MacroPanel.tsx` and `StrategyPage.tsx` retain their existing data and component boundaries; their mobile work is primarily responsive layout and local overflow containment.
- `styles.css` owns the breakpoint, visibility switching, safe-area sizing, touch dimensions, card layouts, bottom sheet, and page-specific responsive rules.

No new global data store or viewport state is introduced.

## Accessibility and Interaction

- The bottom bar uses `nav`; each button preserves its accessible page label and the active button uses `aria-current="page"`.
- Mobile data cards use real buttons when the whole card is interactive. Nested actions do not create invalid nested buttons.
- The symbol detail sheet uses `role="dialog"`, an accessible label, and `aria-modal="true"` in its mobile presentation.
- The sheet closes through its close button, backdrop activation, or Escape.
- Focus-visible styles remain present on navigation, tabs, cards, and actions.
- Reduced-motion preferences continue to disable decorative transitions and pulses where applicable.
- Text and controls must not require hover to expose essential actions.

## Error and State Handling

Responsive presentation does not create new request paths. Fetch failures, macro loading and errors, WebSocket reconnecting or stale states, notification permission, order busy/error states, and strategy operation results continue through the existing React state.

Mobile card and table variants render from the same arrays and callbacks, so filtering, selecting, opening a trade, and closing a position cannot diverge by viewport. Empty states replace both mobile and desktop collections consistently.

## Testing and Acceptance

Automated tests will be added before production changes and will verify:

- all five shell navigation targets remain available and expose active-page semantics;
- the mobile radar card structure contains the approved key fields and invokes symbol selection;
- the mobile symbol detail sheet has dialog semantics and closes through supported controls;
- mobile paper position cards select the existing position and preserve trading actions;
- responsive CSS contains the 760px mobile boundary, bottom safe-area handling, document overflow protection, touch sizing, and explicit table overflow containment.

Existing application tests, style tests, type checks, and production build must remain green.

Manual browser verification covers 360x800, 390x844, 430x932, and 768x1024. At each size:

- the document has no unintended horizontal overflow;
- the bottom navigation does not cover the last content item;
- all five pages are reachable;
- filters, forms, and primary actions fit and remain operable;
- wide tables scroll only inside their panels;
- charts fit their containers;
- the Radar detail sheet opens, scrolls, and closes;
- desktop behavior remains unchanged when returning above 760px.

## Delivery Scope

Implementation is complete when all five pages meet the mobile acceptance checks at 360px and the current desktop application passes its full automated suite and build without behavior changes.
