import type { BackendEvent, DashboardSnapshot } from "./types";

export async function fetchSnapshot(): Promise<DashboardSnapshot> {
  const response = await fetch("http://127.0.0.1:8787/api/snapshot");
  if (!response.ok) {
    throw new Error(`snapshot request failed: ${response.status}`);
  }
  return response.json();
}

export function connectEvents(onEvent: (event: BackendEvent) => void): WebSocket {
  const socket = new WebSocket("ws://127.0.0.1:8787/ws");
  socket.addEventListener("message", (message) => {
    onEvent(JSON.parse(String(message.data)) as BackendEvent);
  });
  return socket;
}
