import type {
  BackendEvent,
  BtcMacroSnapshot,
  ChartSnapshot,
  DashboardSnapshot,
  PaperAccountSnapshot,
  PaperOrderRequest,
} from "./types";
import { connectWebSocketWithReconnect, type RealtimeConnection } from "./realtime";

const backendBaseUrl = "http://127.0.0.1:8787";

export async function fetchSnapshot(): Promise<DashboardSnapshot> {
  return requestJson<DashboardSnapshot>("/api/snapshot");
}

export async function fetchBtcMacro(): Promise<BtcMacroSnapshot> {
  return requestJson<BtcMacroSnapshot>("/api/macro/btc");
}

export async function fetchPaperAccount(): Promise<PaperAccountSnapshot> {
  return requestJson<PaperAccountSnapshot>("/api/paper");
}

export async function fetchSymbolChart(
  instId: string,
  timeframe: ChartSnapshot["timeframe"],
  filled: boolean,
): Promise<ChartSnapshot> {
  const params = new URLSearchParams({
    timeframe,
    limit: "180",
    filled: String(filled),
  });
  return requestJson<ChartSnapshot>(
    `/api/symbols/${encodeURIComponent(instId)}/chart?${params.toString()}`,
  );
}

export async function openPaperOrder(
  order: PaperOrderRequest,
): Promise<PaperAccountSnapshot> {
  return requestJson<PaperAccountSnapshot>("/api/paper/orders", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(order),
  });
}

export async function closePaperPosition(
  instId: string,
): Promise<PaperAccountSnapshot> {
  return requestJson<PaperAccountSnapshot>(
    `/api/paper/positions/${encodeURIComponent(instId)}/close`,
    { method: "POST" },
  );
}

export function connectEvents(
  onEvent: (event: BackendEvent) => void,
  lifecycle: {
    onClose?: () => void;
    onError?: () => void;
    onOpen?: () => void;
  } = {},
): RealtimeConnection {
  return connectWebSocketWithReconnect({
    createSocket: () => new WebSocket("ws://127.0.0.1:8787/ws"),
    onClose: lifecycle.onClose,
    onError: lifecycle.onError,
    onMessage: (message) => {
      onEvent(JSON.parse(String((message as MessageEvent).data)) as BackendEvent);
    },
    onOpen: lifecycle.onOpen,
    retryDelayMs: 1_500,
  });
}

async function requestJson<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${backendBaseUrl}${path}`, init);
  if (!response.ok) {
    throw new Error(await readErrorMessage(response));
  }
  return response.json();
}

async function readErrorMessage(response: Response): Promise<string> {
  try {
    const body = (await response.json()) as { message?: unknown };
    if (typeof body.message === "string" && body.message.length > 0) {
      return body.message;
    }
  } catch {
    // Fall back to the HTTP status when the backend returns a non-JSON error.
  }
  return `request failed: ${response.status}`;
}
