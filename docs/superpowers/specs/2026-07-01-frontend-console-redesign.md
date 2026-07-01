# Frontend Console Redesign

Date: 2026-07-01

## Goal

Optimize the AlphaPulse OKX frontend into a unified trading console that is easier to scan during active market monitoring, while keeping the existing Radar and Macro product capabilities.

The redesign should use the attached Radar reference HTML as the primary visual style: a dark trading-terminal palette, compact top bar, status pills, dense market table, and right-side selected-symbol detail panel. The attached Macro reference informs hierarchy for macro cycle metrics, but its darker `#09090b / #18181b` palette should not become the app-wide color direction.

## Approved Direction

Use a componentized console redesign.

This means:

- Keep the current React/Vite stack and existing backend API contracts.
- Refactor frontend UI into clearer component boundaries instead of only layering more CSS onto `App.tsx`.
- Use a unified console shell for Radar and Macro navigation.
- Show a compact macro summary in the Radar workspace.
- Preserve the full Macro page for deep cycle analysis.
- Keep the implementation scoped to frontend layout, styling, and component organization unless a small API-facing adjustment is needed to support existing data display.

## Reference Inputs

Reference files:

- `/Users/c3b6/Downloads/gemini-code-1782875046952.html`
- `/Users/c3b6/Downloads/gemini-code-1782875072869.html`

The Radar reference is the visual baseline:

- Background: `#0b0e14`
- Surface: `#151924`
- Border: `#2b313f`
- Text: slate/zinc style muted labels with white primary values
- Accent: blue selected tabs and active states
- Positive and negative values: emerald and rose
- Layout: compact top bar, status pills, left table, right detail panel

The Macro reference is used for information hierarchy only:

- Large current-regime card
- Key metric tiles
- Cycle progress bar
- Clear refresh action

## Product Scope

In scope:

- Redesign the app shell and navigation into a compact console header.
- Restyle status indicators as pills.
- Add a macro summary strip to the Radar view using the already-loaded macro snapshot.
- Redesign the Radar workspace around a dense table and selected-symbol detail panel.
- Redesign the Macro page first viewport using card hierarchy while keeping the Radar reference color palette.
- Reorganize selected-symbol details into clear groups.
- Preserve existing theme, language, notification, TradingView modal, chart, paper-trading, and macro data behavior.
- Add or update focused frontend tests for the changed rendering and interactions.

Out of scope:

- Backend data model changes.
- New trading logic, sorting controls, or automatic order actions.
- Replacing the existing chart library.
- A full design system migration.
- Introducing a new UI component package.

## Component Architecture

The frontend should be split into small components with stable responsibilities.

`ConsoleShell` owns the page frame:

- Product title and subtitle.
- Radar/Macro view switcher.
- Backend, stream, notification, last scan, and symbol count status pills.
- Theme, language, and notification actions.
- Layout slots for optional macro summary and active view content.

`MacroSummaryStrip` renders a compact macro summary:

- Current regime.
- BTC price.
- Confidence.
- Drawdown.
- Cycle progress.
- Weekly MA200 when available.

It should consume the same `BtcMacroSnapshot | null` already loaded by `App`. It should not fetch macro data itself.

`RadarWorkspace` owns the Radar view:

- Opportunity filters.
- Sorted and filtered symbol table.
- Empty state.
- Selected-symbol detail panel.
- TradingView modal trigger wiring.

`RadarTable` owns table rendering:

- Symbol and pool tags.
- Price.
- 5m, 15m, and 1h changes.
- Trend and range scores.
- Direction label plus trigger reason.
- Selected row state.
- TV button that does not trigger row selection.

`SymbolDetailPanel` owns selected-symbol details:

- Symbol header and trigger reason.
- Current price and short-horizon metric strip.
- Existing `ChartPanel`.
- Structure group for FVG, support/resistance, funding, and update time.
- Paper trading group for account metrics, quick order controls, position, and recent trades.
- Recent signals group when useful data exists.

`MacroPanel` remains the full macro analysis page:

- First viewport is restyled into a regime card, metric cards, and cycle progress card.
- Existing AHR999 chart, analog comparisons, events, valuation metrics, analogs, trading bias, and external metric statuses stay available below.

## Layout Design

Desktop Radar layout:

- Top console header.
- Macro summary strip below the header.
- Main area split into a flexible table column and a fixed detail column around `360px` to `400px`.
- The market table should be the largest visual area.
- The detail panel should remain visible while scanning table rows on typical desktop widths.

Desktop Macro layout:

- Same console header.
- Macro page starts with the card hierarchy inspired by the Macro reference.
- Deep analysis sections follow below with existing chart/table functionality.

Tablet and small desktop:

- Keep table readable with horizontal overflow where necessary.
- Allow the detail panel to move below the table once the two-column layout becomes cramped.

Mobile:

- Single-column order: header, macro summary, filters, table, selected-symbol details.
- Buttons and pills wrap without overlapping.
- Table may scroll horizontally, but text and buttons must not overflow their own controls.

## Visual Design

The redesign uses the Radar reference palette across both Radar and Macro:

- App background: `#0b0e14`
- Surface: `#151924`
- Border: `#2b313f`
- Subtle border: a lower-opacity variant of `#2b313f`
- Primary text: near-white slate
- Muted text: slate gray
- Accent: blue
- Positive: emerald
- Negative: rose

Dark mode is the primary experience. Light and system theme modes must remain functional, but the highest visual fidelity target is the dark trading-terminal style.

Cards and panels should stay utilitarian:

- Border radius around `8px` for operational panels.
- No decorative gradient-orb backgrounds.
- No marketing-style hero layouts.
- Use compact typography and tabular numeric formatting for prices, percentages, and scores.

## Interaction Design

View switching:

- Radar and Macro remain the two primary views.
- Switching away from Radar closes the TradingView modal as it does today.

Table behavior:

- Clicking a table row selects the symbol.
- Clicking the TV button opens the TradingView modal and stops row-click propagation.
- Selected row uses a clear active marker, such as a blue left border and subtle active background.
- Direction labels use `LONG`, `SHORT`, and `NEUTRAL` as the first visual anchor in the signal column.
- Trigger reason remains visible next to the direction label.

Detail panel behavior:

- Detail groups are expanded by default for this iteration.
- No new collapse state is required in the first implementation.
- Existing paper-trading controls remain available.
- Existing chart timeframe and FVG controls remain available through `ChartPanel`.

Macro behavior:

- Initial macro loading, error, refresh, and stale data behavior should remain compatible with the current `MacroPanel` contract.
- The Radar macro summary should display a compact loading, error, or unavailable state instead of hiding the entire workspace.

## Data Flow

`App` continues to own:

- Dashboard snapshot state.
- Macro snapshot state.
- Backend and stream status.
- Theme and language state.
- Selected symbol state.
- TradingView modal state.
- Paper order state.
- Notification permission state.

Derived data should stay memoized where it already is:

- Sorted symbols.
- Filtered symbols.
- Selected symbol.

The macro snapshot loaded by `App` should be passed to both:

- `MacroSummaryStrip` in the Radar workspace.
- `MacroPanel` in the Macro view.

No component should duplicate macro fetching.

## Error Handling

Dashboard fetch failures:

- Keep current backend disconnected status behavior.
- The Radar workspace should still show an empty or disconnected state without layout breakage.

Macro fetch failures:

- Full Macro view shows the existing error and refresh action.
- Radar macro summary shows a compact unavailable state and preserves the rest of the Radar workspace.

TradingView symbol resolution:

- Keep the existing fallback behavior for unsupported symbol formats.
- Continue showing an unavailable state instead of throwing.

Paper trading actions:

- Preserve current busy and error handling.
- Keep errors visible near the paper-trading controls.

## Accessibility

- View switcher and filters remain button groups with labels.
- Status area remains exposed as connection status content.
- TradingView modal keeps `role="dialog"` and `aria-modal="true"`.
- TV buttons keep symbol-specific accessible labels.
- Color is not the only signal: direction labels and score text remain visible alongside color.
- Focus states should remain visible against the dark palette.

## Testing Plan

Frontend tests should cover:

- Radar and Macro view switching.
- Macro summary rendering with available data.
- Macro summary loading/unavailable behavior.
- Symbol row selection.
- TV button opening the modal without accidental row propagation.
- Macro loading and error states.
- Existing notification and paper-trading behavior that is affected by component movement.

Verification commands:

```bash
cd frontend
npm test
npm run build
```

Manual visual verification:

- Desktop Radar at a typical wide viewport.
- Desktop Macro at a typical wide viewport.
- Narrow/mobile viewport with wrapped controls.
- Dark theme as the primary target.
- Light/system modes for basic readability and absence of broken colors.

## Implementation Notes

Work should preserve unrelated existing changes in the current dirty worktree.

Suggested implementation order:

1. Extract shell/status/filter/table/detail components without changing behavior.
2. Add `MacroSummaryStrip` using existing macro snapshot state.
3. Apply Radar-reference CSS variables and console layout styles.
4. Restyle Radar table and detail panel.
5. Restyle Macro first viewport while preserving deep sections.
6. Update tests and run verification.

## Risks

- `App.tsx`, `MacroPanel.tsx`, and `styles.css` are already large; broad edits can create regressions if not split carefully.
- The current worktree has uncommitted changes. Implementation must avoid reverting or accidentally committing unrelated work.
- Macro chart and AHR999 sections are complex. Restyling should focus on layout containers and first viewport before touching chart internals.
- Light mode may need targeted polish after the dark trading-console style is applied.
