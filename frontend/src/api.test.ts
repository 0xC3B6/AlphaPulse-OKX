import { describe, expect, it } from "vitest";

import { resolveApiUrl, resolveWebSocketUrl } from "./api";

describe("production transport URLs", () => {
  it("uses same-origin API paths when no backend override is configured", () => {
    expect(resolveApiUrl("/api/snapshot", "")).toBe("/api/snapshot");
  });

  it("normalizes an explicit backend URL", () => {
    expect(resolveApiUrl("/api/snapshot", "https://example.test///")).toBe(
      "https://example.test/api/snapshot",
    );
  });

  it("derives same-origin WebSocket URLs from the browser location", () => {
    expect(
      resolveWebSocketUrl("/ws", "", {
        protocol: "https:",
        host: "radar.example.test",
      }),
    ).toBe("wss://radar.example.test/ws");
  });

  it("converts an explicit HTTPS backend URL to WSS", () => {
    expect(
      resolveWebSocketUrl("/ws", "https://api.example.test/", {
        protocol: "http:",
        host: "ignored.example.test",
      }),
    ).toBe("wss://api.example.test/ws");
  });
});
