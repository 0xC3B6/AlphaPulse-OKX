import { describe, expect, it } from "vitest";
import { resolveApiUrl, resolveWebSocketUrl } from "./api";

describe("API endpoint resolution", () => {
  it("uses same-origin HTTP paths when no backend base URL is configured", () => {
    expect(resolveApiUrl("/api/snapshot", "")).toBe("/api/snapshot");
  });

  it("removes trailing slashes from a configured backend base URL", () => {
    expect(resolveApiUrl("/api/snapshot", "https://example.com/backend/")).toBe(
      "https://example.com/backend/api/snapshot",
    );
  });

  it("uses the current origin for same-origin WebSocket paths", () => {
    expect(
      resolveWebSocketUrl("/ws", "", {
        protocol: "https:",
        host: "alpha.example.com",
      }),
    ).toBe("wss://alpha.example.com/ws");
  });

  it("converts configured HTTP backend URLs to WebSocket URLs", () => {
    expect(resolveWebSocketUrl("/ws", "http://127.0.0.1:8787")).toBe(
      "ws://127.0.0.1:8787/ws",
    );
  });
});
