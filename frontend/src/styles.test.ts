// @ts-expect-error Vitest runs this in Node, but the app tsconfig does not load Node types.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const css = readFileSync("src/styles.css", "utf8");

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
