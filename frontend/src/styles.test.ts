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
    expect(css).toContain(".macro-summary-strip");
    expect(css).toContain(".radar-workspace");
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
});
