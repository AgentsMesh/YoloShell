import { HubError } from "./hubError.ts";

export type HubEndpoint = { socket: string; hubId: string };
type Pending = {
  resolve: (value: unknown) => void;
  reject: (error: Error) => void;
  timer: ReturnType<typeof setTimeout>;
};

const CALL_TIMEOUT_MS = 10_000;

/** A timed-out call has an unknown server-side fate and must not be retried. */
export class HubTimeoutError extends Error {}

/** One JSON-RPC socket: framing, request correlation and failure fan-out. */
export class HubWire {
  private socket: import("bun").Socket | null = null;
  private pending = new Map<number, Pending>();
  private sequence = 0;
  private buffer = "";
  private decoder = new TextDecoder();

  static async connect(endpoint: HubEndpoint): Promise<HubWire> {
    const wire = new HubWire();
    const lost = (error: Error): void => { wire.socket = null; wire.fail(error); };
    wire.socket = await Bun.connect({
      unix: endpoint.socket,
      socket: {
        data: (_socket, data) => wire.receive(data),
        close: () => lost(new Error("Hub connection closed")),
        error: (_socket, error) => lost(error),
      },
    });
    return wire;
  }

  get connected(): boolean { return this.socket !== null; }

  call(method: string, params: unknown, timeoutMs = CALL_TIMEOUT_MS): Promise<unknown> {
    const id = ++this.sequence;
    const socket = this.socket;
    if (!socket) return Promise.reject(new Error("Hub is not connected"));
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new HubTimeoutError("Hub request timed out"));
      }, timeoutMs);
      this.pending.set(id, { resolve, reject, timer });
      try {
        socket.write(`${JSON.stringify({ jsonrpc: "2.0", id, method, params })}\n`);
      } catch (error) {
        this.pending.delete(id);
        clearTimeout(timer);
        reject(error instanceof Error ? error : new Error(String(error)));
      }
    });
  }

  close(): void {
    this.fail(new Error("Hub connection closed"));
    this.socket?.end();
    this.socket = null;
  }

  private receive(data: Uint8Array): void {
    this.buffer += this.decoder.decode(data, { stream: true });
    let newline = this.buffer.indexOf("\n");
    while (newline >= 0) {
      const line = this.buffer.slice(0, newline);
      this.buffer = this.buffer.slice(newline + 1);
      this.frame(line);
      newline = this.buffer.indexOf("\n");
    }
  }

  private frame(line: string): void {
    if (!line.trim()) return;
    let message: {
      id?: number;
      result?: unknown;
      error?: { code?: number; message?: string; data?: unknown };
    };
    try {
      message = JSON.parse(line);
    } catch {
      return;
    }
    if (typeof message.id !== "number") return;
    const pending = this.pending.get(message.id);
    if (!pending) return;
    this.pending.delete(message.id);
    clearTimeout(pending.timer);
    if (message.error) {
      pending.reject(
        new HubError(
          message.error.code ?? -32603,
          message.error.message ?? "Hub error",
          message.error.data,
        ),
      );
    } else {
      pending.resolve(message.result);
    }
  }

  private fail(error: Error): void {
    for (const pending of this.pending.values()) {
      clearTimeout(pending.timer);
      pending.reject(error);
    }
    this.pending.clear();
  }
}
