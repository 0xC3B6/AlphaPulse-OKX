export interface SocketLike {
  addEventListener(type: string, listener: (event: unknown) => void): void;
  close(): void;
}

export interface RealtimeConnection {
  close(): void;
}

export function connectWebSocketWithReconnect({
  createSocket,
  onClose,
  onError,
  onMessage,
  onOpen,
  onReconnectAttempt,
  onStale,
  maxRetryDelayMs,
  retryDelayMs,
  staleTimeoutMs,
}: {
  createSocket: () => SocketLike;
  maxRetryDelayMs?: number;
  onClose?: () => void;
  onError?: () => void;
  onMessage: (event: unknown) => void;
  onOpen?: () => void;
  onReconnectAttempt?: (delayMs: number) => void;
  onStale?: () => void;
  retryDelayMs: number;
  staleTimeoutMs?: number;
}): RealtimeConnection {
  let closed = false;
  let socket: SocketLike | null = null;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  let staleTimer: ReturnType<typeof setTimeout> | null = null;
  let nextRetryDelayMs = retryDelayMs;
  const retryCapMs = maxRetryDelayMs ?? retryDelayMs;

  function clearReconnectTimer() {
    if (reconnectTimer !== null) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
  }

  function clearStaleTimer() {
    if (staleTimer !== null) {
      clearTimeout(staleTimer);
      staleTimer = null;
    }
  }

  function refreshStaleTimer() {
    if (staleTimeoutMs === undefined || closed) {
      return;
    }
    clearStaleTimer();
    staleTimer = setTimeout(() => {
      staleTimer = null;
      onStale?.();
    }, staleTimeoutMs);
  }

  function scheduleReconnect() {
    if (closed || reconnectTimer !== null) {
      return;
    }
    const delayMs = nextRetryDelayMs;
    onReconnectAttempt?.(delayMs);
    reconnectTimer = setTimeout(() => {
      reconnectTimer = null;
      connect();
    }, delayMs);
    nextRetryDelayMs = Math.min(nextRetryDelayMs * 2, retryCapMs);
  }

  function connect() {
    if (closed) {
      return;
    }
    socket = createSocket();
    socket.addEventListener("open", () => {
      nextRetryDelayMs = retryDelayMs;
      refreshStaleTimer();
      onOpen?.();
    });
    socket.addEventListener("message", (event) => {
      refreshStaleTimer();
      onMessage(event);
    });
    socket.addEventListener("close", () => {
      clearStaleTimer();
      onClose?.();
      scheduleReconnect();
    });
    socket.addEventListener("error", () => {
      clearStaleTimer();
      onError?.();
      scheduleReconnect();
    });
  }

  connect();

  return {
    close() {
      closed = true;
      clearReconnectTimer();
      clearStaleTimer();
      socket?.close();
    },
  };
}
