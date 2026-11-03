import { HubClient } from "./hub";
import { HubError } from "./hubError.ts";
import type { LeaseController } from "./leaseController";

const MAX_INPUT_BYTES = 65_536;

export class InputQueue {
  private readonly chunks: Uint8Array[] = [];
  private flushing = false;

  constructor(
    private readonly input: HubClient,
    private readonly control: HubClient,
    private readonly lease: LeaseController,
    private readonly onError: (error: Error) => void,
  ) {}

  enqueue(data: Uint8Array): void {
    for (let offset = 0; offset < data.byteLength; offset += MAX_INPUT_BYTES) {
      const chunk = data.subarray(offset, Math.min(offset + MAX_INPUT_BYTES, data.byteLength));
      if (chunk.byteLength) this.chunks.push(Uint8Array.from(chunk));
    }
    if (!this.flushing) void this.flush();
  }

  async resize(cols: number, rows: number): Promise<void> {
    if (!Number.isInteger(cols) || !Number.isInteger(rows) || cols < 2 || rows < 2 || cols > 1000 || rows > 1000) {
      this.onError(new Error("Terminal size is invalid"));
      return;
    }
    const grant = this.lease.currentGrant;
    if (this.lease.state !== "controlled" || !grant) {
      this.onError(new Error("Terminal control is not held"));
      return;
    }
    try {
      await this.control.request(
        "resize",
        { request: { request_id: crypto.randomUUID(), grant, size: { cols, rows } } },
        this.lease.origin,
      );
    } catch (error) {
      this.onError(error instanceof Error ? error : new Error("Terminal resize failed"));
    }
  }

  private async flush(): Promise<void> {
    if (this.flushing) return;
    this.flushing = true;
    try {
      while (this.chunks.length) {
        if (!(await this.send(this.takeBatch()))) return;
      }
    } finally {
      this.flushing = false;
    }
  }

  /**
   * Collapse every chunk that queued behind one delivery into one request.
   * Fast typing and backspace repeat otherwise become one Hub round trip per
   * key, and the queue only grows while the first key is in flight.
   */
  private takeBatch(): Uint8Array {
    const first = this.chunks.shift()!;
    if (first.byteLength >= MAX_INPUT_BYTES) return first;
    const parts = [first];
    let size = first.byteLength;
    while (this.chunks.length) {
      const next = this.chunks[0];
      if (size + next.byteLength > MAX_INPUT_BYTES) break;
      parts.push(this.chunks.shift()!);
      size += next.byteLength;
    }
    if (parts.length === 1) return first;
    const batch = new Uint8Array(size);
    let offset = 0;
    for (const part of parts) {
      batch.set(part, offset);
      offset += part.byteLength;
    }
    return batch;
  }

  /** A rejected lease is retried once after acquiring; delivery failures are not. */
  private async send(data: Uint8Array): Promise<boolean> {
    const grant = this.lease.currentGrant;
    if (this.lease.state !== "controlled" || !grant) {
      if (!(await this.recover(false))) return this.fail("Terminal control is not held");
    }
    try {
      await this.deliver(data);
      return true;
    } catch (error) {
      if (isLeaseLoss(error) && (await this.recover(true))) {
        try {
          await this.deliver(data);
          return true;
        } catch (retry) {
          return this.fail(retry);
        }
      }
      return this.fail(error);
    }
  }

  private async deliver(data: Uint8Array): Promise<void> {
    const grant = this.lease.currentGrant;
    if (this.lease.state !== "controlled" || !grant) {
      throw new HubError(-32603, "Terminal control is not held", null);
    }
    await this.input.input(
      { request_id: crypto.randomUUID(), grant, data: Array.from(data) },
      this.lease.origin,
    );
  }

  private async recover(force: boolean): Promise<boolean> {
    if (this.lease.state === "released") return false;
    if (!force && this.lease.state === "controlled" && this.lease.currentGrant) return true;
    try {
      await this.lease.acquire();
      return this.lease.state === "controlled" && this.lease.currentGrant !== null;
    } catch {
      return false;
    }
  }

  private fail(error: unknown): boolean {
    this.chunks.length = 0;
    this.onError(error instanceof Error ? error : new Error("Terminal input failed"));
    return false;
  }
}

function isLeaseLoss(error: unknown): boolean {
  if (!(error instanceof HubError)) return false;
  const data = error.data as { kind?: unknown; code?: unknown } | null;
  return data?.kind === "hub" && data.code === "invalid_lease";
}
