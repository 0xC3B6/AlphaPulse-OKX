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

  it("uses exponential reconnect backoff capped by max delay", () => {
    const sockets: FakeSocket[] = [];
    const reconnecting = vi.fn();

    const connection = connectWebSocketWithReconnect({
      createSocket: () => {
        const socket = new FakeSocket();
        sockets.push(socket);
        return socket;
      },
      maxRetryDelayMs: 4_000,
      onMessage: vi.fn(),
      onReconnectAttempt: reconnecting,
      retryDelayMs: 1_000,
    });

    sockets[0].emit("close");
    vi.advanceTimersByTime(1_000);
    expect(sockets).toHaveLength(2);
    expect(reconnecting).toHaveBeenLastCalledWith(1_000);

    sockets[1].emit("close");
    vi.advanceTimersByTime(1_999);
    expect(sockets).toHaveLength(2);
    vi.advanceTimersByTime(1);
    expect(sockets).toHaveLength(3);
    expect(reconnecting).toHaveBeenLastCalledWith(2_000);

    sockets[2].emit("close");
    vi.advanceTimersByTime(4_000);
    expect(sockets).toHaveLength(4);
    expect(reconnecting).toHaveBeenLastCalledWith(4_000);

    connection.close();
  });

  it("marks the stream stale when no message arrives before the heartbeat timeout", () => {
    const sockets: FakeSocket[] = [];
    const onStale = vi.fn();

    const connection = connectWebSocketWithReconnect({
      createSocket: () => {
        const socket = new FakeSocket();
        sockets.push(socket);
        return socket;
      },
      onMessage: vi.fn(),
      onStale,
      retryDelayMs: 1_000,
      staleTimeoutMs: 5_000,
    });

    sockets[0].emit("open");
    vi.advanceTimersByTime(4_999);
    expect(onStale).not.toHaveBeenCalled();

    sockets[0].emit("message", { data: "{}" });
    vi.advanceTimersByTime(4_999);
    expect(onStale).not.toHaveBeenCalled();

    vi.advanceTimersByTime(1);
    expect(onStale).toHaveBeenCalledTimes(1);

    connection.close();
  });
});
