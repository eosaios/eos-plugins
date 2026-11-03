/** Isolated plugin-side SDK. No EOS account credentials or native bridges. */
export interface PluginClient {
  request<T = unknown>(method: string, params?: unknown): Promise<T>;
  close(): void;
}

export function connectPlugin(): Promise<PluginClient> {
  const nonce = (window as unknown as { __EOS_PLUGIN_NONCE__?: string }).__EOS_PLUGIN_NONCE__;
  if (!nonce || window.parent === window) return Promise.reject(new Error("PLUGIN_HOST_REQUIRED"));
  return new Promise((resolve, reject) => {
    const timer = window.setTimeout(() => { window.removeEventListener("message", receive); reject(new Error("PLUGIN_HANDSHAKE_TIMEOUT")); }, 10000);
    const receive = (event: MessageEvent) => {
      if (event.source !== window.parent || event.data?.type !== "eos-plugin:init" || event.data.nonce !== nonce || event.ports.length !== 1) return;
      window.clearTimeout(timer); window.removeEventListener("message", receive);
      const port = event.ports[0];
      const pending = new Map<string, { resolve: (value: unknown) => void; reject: (error: Error) => void; timer: number }>();
      let sequence = 0, closed = false;
      port.onmessage = event => {
        const response = event.data;
        if (!response || typeof response.id !== "string") return;
        const request = pending.get(response.id); if (!request) return;
        pending.delete(response.id); window.clearTimeout(request.timer);
        if (typeof response.error === "string") request.reject(new Error(response.error)); else request.resolve(response.result);
      };
      resolve({
        request<T>(method: string, params: unknown = {}): Promise<T> {
          if (closed) return Promise.reject(new Error("PLUGIN_CHANNEL_CLOSED"));
          if (pending.size >= 10) return Promise.reject(new Error("PLUGIN_REQUEST_LIMIT"));
          const id = String(++sequence), message = { id, method, params };
          if (JSON.stringify(message).length > 1024 * 1024) return Promise.reject(new Error("PLUGIN_REQUEST_TOO_LARGE"));
          return new Promise<T>((resolve, reject) => {
            const timer = window.setTimeout(() => { pending.delete(id); reject(new Error("PLUGIN_REQUEST_TIMEOUT")); }, 30000);
            pending.set(id, { resolve: value => resolve(value as T), reject, timer });
            port.postMessage(message);
          });
        },
        close() {
          closed = true; port.close();
          for (const request of pending.values()) { window.clearTimeout(request.timer); request.reject(new Error("PLUGIN_CHANNEL_CLOSED")); }
          pending.clear();
        },
      });
    };
    window.addEventListener("message", receive);
  });
}
