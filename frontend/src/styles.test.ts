// @ts-expect-error Vitest runs this in Node, but the app tsconfig does not load Node types.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const css = readFileSync("src/styles.css", "utf8");

const cssBlock = (selector: string) => {
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = css.match(new RegExp(`${escapedSelector}\\s*\\{(?<body>[^}]*)\\}`));

  if (!match?.groups?.body) {
    throw new Error(`Missing CSS block for ${selector}`);
  }

  return match.groups.body;
};

const lastCssBlock = (selector: string) => {
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const matches = [...css.matchAll(new RegExp(`${escapedSelector}\\s*\\{(?<body>[^}]*)\\}`, "g"))];
  const match = matches[matches.length - 1];

  if (!match?.groups?.body) {
    throw new Error(`Missing CSS block for ${selector}`);
  }

  return match.groups.body;
};

const cssBlockContaining = (selector: string, expected: string) => {
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const matches = [...css.matchAll(new RegExp(`${escapedSelector}\\s*\\{(?<body>[^}]*)\\}`, "g"))];
  const match = matches.find((candidate) => candidate.groups?.body.includes(expected));

  if (!match?.groups?.body) {
    throw new Error(`Missing CSS block for ${selector} containing ${expected}`);
  }

  return match.groups.body;
};

const systemDarkBlock = () => {
  const match = css.match(
    /@media \(prefers-color-scheme: dark\)\s*\{\s*:root:not\(\[data-theme\]\),\s*:root\[data-theme="system"\]\s*\{(?<body>[^}]*)\}/,
  );

  if (!match?.groups?.body) {
    throw new Error("Missing system dark CSS block");
  }

  return match.groups.body;
};

describe("theme transitions", () => {
  it("animates theme color changes and respects reduced motion", () => {
    expect(css).toContain("--theme-transition-duration");
    expect(css).toContain("background-color var(--theme-transition-duration)");
    expect(css).toContain("color var(--theme-transition-duration)");
    expect(css).toContain("border-color var(--theme-transition-duration)");
    expect(css).toContain("@media (prefers-reduced-motion: reduce)");
    expect(css).toContain("--theme-transition-duration: 0ms");
  });
});

describe("console palette", () => {
  it("loads the Tailwind 4 pipeline used by the Figma bundle", () => {
    expect(css).toContain('@import "tailwindcss" source(none);');
    expect(css).toContain('@source "./**/*.{js,ts,jsx,tsx}";');
    expect(css).toContain('@import "tw-animate-css";');
  });

  it("uses the approved radar reference colors and responsive console classes", () => {
    const paletteAnchors = ["--app-bg: #0b0e14", "--surface: #151924", "--border: #2b313f"];
    const explicitDark = cssBlock(':root[data-theme="dark"]');
    const systemDark = systemDarkBlock();

    for (const anchor of paletteAnchors) {
      expect(explicitDark).toContain(anchor);
      expect(systemDark).toContain(anchor);
    }

    expect(css).toContain(".console-topbar");
    expect(css).toContain(".task-rail");
    expect(css).toContain(".task-rail-button");
    expect(css).toContain("--terminal-bg: #070b12");
    expect(css).toContain("--terminal-cyan: #22d3ee");
    expect(css).toContain(".terminal-market-tape");
    expect(css).toContain(".macro-summary-strip");
    expect(css).toContain(".radar-workspace");
    expect(css).toContain(".strategy-page");
    expect(css).toContain(".strategy-workspace");
    expect(css).toContain(".trade-page");
    expect(css).toContain(".review-page");
    expect(css).toContain("@media (max-width: 960px)");
  });

  it("protects the console and macro grids from tablet overflow", () => {
    expect(css).toContain("@media (max-width: 1120px)");
    expect(css).toContain("@media (max-width: 1080px)");
    expect(css).toContain(".console-topbar > *");
    expect(css).toContain("min-width: 0");
    expect(css).toContain(".radar-workspace > .detail-panel");
    expect(css).not.toContain(".radar-table-panel,\n.detail-panel");
  });

  it("keeps theme and language controls visible in the terminal header", () => {
    const headerActions = lastCssBlock(".figma-radar-header .console-actions");

    expect(headerActions).toContain("display: flex");
    expect(headerActions).not.toContain("display: none");
  });

  it("lets the light theme control the terminal palette", () => {
    const light = cssBlock(':root[data-theme="light"]');
    const terminalShell = cssBlockContaining(".terminal-shell", "background: var(--terminal-bg)");

    expect(light).toContain("--terminal-bg: #f5f7fb");
    expect(light).toContain("--terminal-panel: #ffffff");
    expect(terminalShell).toContain("background: var(--terminal-bg)");
    expect(terminalShell).not.toContain("--app-bg:");
  });

  it("keeps detail market metrics inside their cards", () => {
    const metricStrip = cssBlock(".detail-metric-strip");
    const metricCard = cssBlock(".detail-metric-strip div");
    const metricValue = cssBlock(".detail-metric-strip dd");

    expect(metricStrip).toContain("auto-fit");
    expect(metricStrip).toContain("minmax(96px, 1fr)");
    expect(metricCard).toContain("min-width: 0");
    expect(metricCard).toContain("overflow: hidden");
    expect(metricValue).toContain("max-width: 100%");
    expect(metricValue).toContain("overflow-wrap: anywhere");
  });

  it("locks document scroll and delegates overflow to content surfaces", () => {
    expect(css).toMatch(
      /html,\s*body,\s*#root\s*\{[^}]*height:\s*100%;[^}]*min-height:\s*0;[^}]*overflow:\s*hidden;/s,
    );

    const contentSurface = lastCssBlock(".console-main > .page-surface");
    const macroSurface = lastCssBlock(".console-main > .macro-panel");

    expect(contentSurface).toContain("flex: 1 1 auto");
    expect(contentSurface).toContain("min-height: 0");
    expect(contentSurface).toContain("contain: layout paint");
    expect(contentSurface).toContain("overflow: auto");
    expect(contentSurface).toContain("align-content: start");

    expect(macroSurface).toContain("flex: 1 1 auto");
    expect(macroSurface).toContain("min-height: 0");
    expect(macroSurface).toContain("contain: layout paint");
    expect(macroSurface).toContain("overflow: auto");
  });

  it("keeps radar overflow inside the table pane", () => {
    const monitorPage = cssBlockContaining(".monitor-page", "overflow: hidden");
    const radarTablePanel = cssBlockContaining(".monitor-page .radar-table-panel", "overflow: auto");

    expect(monitorPage).toContain("overflow: hidden");
    expect(radarTablePanel).toContain("min-height: 0");
    expect(radarTablePanel).toContain("flex: 1");
    expect(radarTablePanel).toContain("contain: layout paint");
    expect(radarTablePanel).toContain("overflow: auto");
  });
});
