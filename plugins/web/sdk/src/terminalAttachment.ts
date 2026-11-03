import { FrameQueue, takeBlob } from "./attachmentFrames.ts";
import { HubError } from "./hubError.ts";

export type AttachmentTarget = {
  address: { hub_id: string; terminal_id: string };
  runtime_epoch: string;
};
export type AttachmentCursor = { target: AttachmentTarget; after_sequence: number };
export type AttachmentRequest = {
  target: AttachmentTarget;
  cursor: AttachmentCursor | null;
  seed?: AttachmentSeed;
};
export type AttachmentSnapshot = {
  data: Uint8Array;
  cols: number;
  rows: number;
  sequence: number;
};
export type AttachmentHistory = {
  data: Uint8Array;
  cols: number;
  rows: number;
  barrier_sequence: number;
};
export type AttachmentSeed = "snapshot" | "history";
export type AttachmentInitial = {
  target: AttachmentTarget;
  snapshot: AttachmentSnapshot | null;
  chunks: Uint8Array[];
  history: AttachmentHistory | null;
  next_sequence: number;
  resynchronized: boolean;
};
export type AttachmentEvent =
  | { event: "output"; data: Uint8Array }
  | { event: "heartbeat" }
  | { event: "exited"; code: number | null; signal: number | null }
  | { event: "resync" }
  | { event: "unreachable" };

const STREAM = "yolo.terminal.binary.v1";
const TIMEOUT_MS = 10_000;

export class TerminalAttachment {
  private seed: AttachmentInitial | null;

  private constructor(
    initial: AttachmentInitial,
    private readonly queue: FrameQueue,
    private readonly socket: import("bun").Socket,
  ) {
    this.seed = initial;
  }

  /** Hands the one-shot seed to the caller and releases its bytes. */
  takeInitial(): AttachmentInitial | null {
    const seed = this.seed;
    this.seed = null;
    return seed;
  }

  static async open(
    endpoint: { socket: string; hubId: string },
    request: AttachmentRequest,
  ): Promise<TerminalAttachment> {
    const queue = new FrameQueue();
    const socket = await Bun.connect({
      unix: endpoint.socket,
      socket: {
        data: (_socket, data) => queue.push(data),
        close: () => queue.close(),
        error: (_socket, error) => queue.close(error instanceof Error ? error : new Error(String(error))),
      },
    });
    try {
      return await TerminalAttachment.openOn(endpoint, request, queue, socket);
    } catch (error) {
      queue.close(error instanceof Error ? error : new Error(String(error)));
      socket.end();
      throw error;
    }
  }

  private static async openOn(
    endpoint: { socket: string; hubId: string },
    request: AttachmentRequest,
    queue: FrameQueue,
    socket: import("bun").Socket,
  ): Promise<TerminalAttachment> {
    let nextId = 0;
    const call = async (method: string, params: unknown): Promise<any> => {
      const id = ++nextId;
      socket.write(`${JSON.stringify({ jsonrpc: "2.0", id, method, params })}\n`);
      const deadline = setTimeout(() => queue.close(new Error("Hub attachment request timed out")), TIMEOUT_MS);
      try {
        for (;;) {
          const frame = await queue.take();
          if (frame.kind !== "json") throw new Error("unexpected binary attachment frame");
          const message = frame.value as {
            id?: number;
            result?: unknown;
            error?: { code?: number; message?: string; data?: unknown };
          };
          if (typeof message.id !== "number") throw new Error("unexpected attachment notification");
          if (message.id !== id) throw new Error("unexpected attachment reply");
          if (message.error) {
            throw new HubError(message.error.code ?? -32603, message.error.message ?? "Hub error", message.error.data);
          }
          return message.result;
        }
      } finally {
        clearTimeout(deadline);
      }
    };
    const hello = (await call("hello", { destination: endpoint.hubId })) as {
      hello?: { hub_id?: string };
    };
    if (hello.hello?.hub_id !== endpoint.hubId) throw new Error("Hub identity mismatch");
    const attach = { destination: request.target.address.hub_id, request };
    const result = (await call("attach", attach)) as {
      stream?: unknown;
      attachment?: any;
    };
    const meta = result.attachment;
    if (result.stream !== STREAM || !meta || meta.target?.runtime_epoch !== request.target.runtime_epoch) {
      throw new Error("Terminal attachment is invalid");
    }
    const snapshot = meta.snapshot
      ? {
          data: await takeBlob(queue, meta.snapshot.byte_length),
          cols: meta.snapshot.size?.cols ?? 0,
          rows: meta.snapshot.size?.rows ?? 0,
          sequence: meta.snapshot.barrier_sequence,
        }
      : null;
    const chunks: Uint8Array[] = [];
    for (const chunk of meta.chunks ?? []) chunks.push(await takeBlob(queue, chunk.byte_length));
    const history = meta.history
      ? {
          data: await takeBlob(queue, meta.history.byte_length),
          cols: meta.history.size?.cols ?? 0,
          rows: meta.history.size?.rows ?? 0,
          barrier_sequence: meta.history.barrier_sequence,
        }
      : null;
    if (
      history &&
      (meta.snapshot ||
        (meta.chunks ?? []).length > 0 ||
        meta.resynchronized === true ||
        meta.history.target?.runtime_epoch !== request.target.runtime_epoch)
    ) {
      throw new Error("Terminal history attachment is invalid");
    }
    return new TerminalAttachment(
      {
        target: meta.target,
        snapshot,
        chunks,
        history,
        next_sequence: meta.next_sequence,
        resynchronized: meta.resynchronized === true,
      },
      queue,
      socket,
    );
  }

  async nextEvent(): Promise<AttachmentEvent> {
    for (;;) {
      const frame = await this.queue.take();
      if (frame.kind !== "json") throw new Error("unexpected binary attachment frame");
      const message = frame.value as { method?: string; params?: any };
      if (message.method !== "terminal_event" || !message.params) continue;
      const params = message.params;
      switch (params.event) {
        case "output":
          return { event: "output", data: await takeBlob(this.queue, params.chunk.byte_length) };
        case "heartbeat":
          return { event: "heartbeat" };
        case "exited":
          return { event: "exited", code: params.code ?? null, signal: params.signal ?? null };
        case "resync_required":
          return { event: "resync" };
        case "unreachable":
          return { event: "unreachable" };
        default:
          throw new Error("unknown terminal event");
      }
    }
  }

  close(): void {
    this.queue.close();
    this.socket.end();
  }
}
