import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { connectWebSocketWithReconnect, type SocketLike } from "./realtime";

class FakeSocket implements SocketLike {
  private listeners = new Map<string, Set<(event: unknown) => void>>();
  closed = false;

  addEventListener(type: string, listener: (event: unknown) => void): void {
    const listeners = this.listeners.get(type) ?? new Set();
    listeners.add(listener);
    this.listeners.set(type, listeners);
  }

  close(): void {
    this.closed = true;
  }

  emit(type: string, event: unknown = {}): void {
    for (const listener of this.listeners.get(type) ?? []) {
      listener(event);
    }
  }
}

describe("connectWebSocketWithReconnect", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("reconnects after an unexpected close", () => {
    const sockets: FakeSocket[] = [];

    const connection = connectWebSocketWithReconnect({
      createSocket: () => {
        const socket = new FakeSocket();
        sockets.push(socket);
        return socket;
      },
      onMessage: vi.fn(),
      retryDelayMs: 1_000,
    });

    expect(sockets).toHaveLength(1);

    sockets[0].emit("close");
    vi.advanceTimersByTime(999);
    expect(sockets).toHaveLength(1);

    vi.advanceTimersByTime(1);
    expect(sockets).toHaveLength(2);

    connection.close();
  });

  it("does not reconnect after manual close", () => {
    const sockets: FakeSocket[] = [];

    const connection = connectWebSocketWithReconnect({
      createSocket: () => {
        const socket = new FakeSocket();
        sockets.push(socket);
        return socket;
      },
      onMessage: vi.fn(),
      retryDelayMs: 1_000,
    });

    connection.close();
    sockets[0].emit("close");
    vi.advanceTimersByTime(1_000);

    expect(sockets).toHaveLength(1);
    expect(sockets[0].closed).toBe(true);
  });
});
