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
  retryDelayMs,
}: {
  createSocket: () => SocketLike;
  onClose?: () => void;
  onError?: () => void;
  onMessage: (event: unknown) => void;
  onOpen?: () => void;
  retryDelayMs: number;
}): RealtimeConnection {
  let closed = false;
  let socket: SocketLike | null = null;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

  function clearReconnectTimer() {
    if (reconnectTimer !== null) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
  }

  function scheduleReconnect() {
    if (closed || reconnectTimer !== null) {
      return;
    }
    reconnectTimer = setTimeout(() => {
      reconnectTimer = null;
      connect();
    }, retryDelayMs);
  }

  function connect() {
    if (closed) {
      return;
    }
    socket = createSocket();
    socket.addEventListener("open", () => onOpen?.());
    socket.addEventListener("message", onMessage);
    socket.addEventListener("close", () => {
      onClose?.();
      scheduleReconnect();
    });
    socket.addEventListener("error", () => {
      onError?.();
      scheduleReconnect();
    });
  }

  connect();

  return {
    close() {
      closed = true;
      clearReconnectTimer();
      socket?.close();
    },
  };
}
