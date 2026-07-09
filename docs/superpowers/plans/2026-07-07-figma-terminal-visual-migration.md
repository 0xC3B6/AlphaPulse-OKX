# Figma Terminal Visual Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Apply the downloaded Figma "监控系统与策略模块" terminal-style visual system to the existing AlphaPulse frontend while preserving real backend data and the current four top-level pages.

**Architecture:** Treat the Figma code bundle as visual reference only. Keep the existing React/Vite app, API flow, WebSocket reconnect logic, and page boundaries; update `ConsoleShell`, monitor/table presentation, and shared CSS tokens/classes to match the dense dark quantitative terminal style.

**Tech Stack:** React 18, TypeScript, Vite, plain CSS, existing lightweight-charts integration, Vitest/Testing Library.

---

### Task 1: Shell And Navigation Contract

**Files:**
- Modify: `frontend/src/App.test.tsx`
- Modify: `frontend/src/ConsoleShell.tsx`
- Modify: `frontend/src/App.tsx`
- Modify: `frontend/src/styles.css`

- [ ] **Step 1: Write failing shell expectations**

Add expectations to the existing "uses four top-level task rail pages and keeps radar filters inside Monitor" test:

```tsx
expect(screen.getByTestId("terminal-shell")).toBeInTheDocument();
expect(screen.getByTestId("terminal-market-tape")).toHaveTextContent("LAB-USDT-SWAP");
expect(screen.getByTestId("terminal-market-tape")).toHaveTextContent("DOGE-USDT-SWAP");
expect(screen.getByTestId("terminal-live-status")).toHaveTextContent("LIVE");
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- App.test.tsx -t "uses four top-level"`

Expected: FAIL because `terminal-shell`, `terminal-market-tape`, and `terminal-live-status` are missing.

- [ ] **Step 3: Implement shell contract**

Add `data-testid="terminal-shell"` on the top-level shell, render a compact `terminal-market-tape` from live symbols passed by `App`, and render a sidebar footer `terminal-live-status`. Keep four nav buttons only: monitor, trade, review, macro.

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- App.test.tsx -t "uses four top-level"`

Expected: PASS.

### Task 2: Monitor Terminal Layout

**Files:**
- Modify: `frontend/src/App.test.tsx`
- Modify: `frontend/src/MonitorPage.tsx`
- Modify: `frontend/src/RadarTable.tsx`
- Modify: `frontend/src/styles.css`

- [ ] **Step 1: Write failing monitor expectations**

Add expectations to the existing "loads symbols and filters trend opportunities" test:

```tsx
expect(screen.getByTestId("monitor-terminal")).toBeInTheDocument();
expect(screen.getByTestId("radar-terminal-table")).toHaveTextContent("Signal");
expect(screen.getByTestId("monitor-live-count")).toHaveTextContent("symbols");
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- App.test.tsx -t "loads symbols and filters"`

Expected: FAIL because the new test ids and live count are missing.

- [ ] **Step 3: Implement monitor contract**

Add terminal wrapper attributes, live symbol count, compact segmented filters, dense table styling hooks, and terminal-style score/signal cells using existing `SymbolSnapshot` data only.

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- App.test.tsx -t "loads symbols and filters"`

Expected: PASS.

### Task 3: Global Terminal Theme

**Files:**
- Modify: `frontend/src/styles.css`
- Modify: `frontend/src/styles.test.ts`

- [ ] **Step 1: Write failing CSS expectations**

Add CSS regression checks requiring the dark terminal palette:

```ts
expect(css).toContain("--terminal-bg: #070b12");
expect(css).toContain("--terminal-cyan: #22d3ee");
expect(css).toContain(".terminal-market-tape");
```

- [ ] **Step 2: Run CSS test to verify it fails**

Run: `npm test -- styles.test.ts`

Expected: FAIL until the new tokens/classes exist.

- [ ] **Step 3: Implement CSS migration**

Introduce terminal tokens from the Figma bundle, tune dark/light/system modes, restyle shell/sidebar/topbar/market tape/cards/tables/buttons/badges, and keep responsive constraints for desktop and mobile.

- [ ] **Step 4: Run CSS test to verify it passes**

Run: `npm test -- styles.test.ts`

Expected: PASS.

### Task 4: Verification

**Files:**
- No production file changes required.

- [ ] **Step 1: Run full static and unit verification**

Run:

```bash
npm run lint
npm test
npm run build
```

Expected: All commands exit 0. Existing React `act(...)` warnings may remain if tests pass.

- [ ] **Step 2: Browser visual verification**

Open the local dev server, verify the shell has a dark terminal sidebar and market tape, Monitor keeps real symbols and filters, Review keeps strategy/history interactions, and WebSocket state reconnects to connected without refresh.

- [ ] **Step 3: Final status**

Report changed files, verification commands, and local URL. Do not claim completion without the command output from Step 1 and browser observation from Step 2.
