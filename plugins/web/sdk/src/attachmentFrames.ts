export type Frame = { kind: "json"; value: unknown } | { kind: "blob"; data: Uint8Array };

export async function takeBlob(queue: FrameQueue, expected: number): Promise<Uint8Array> {
  const frame = await queue.take();
  if (frame.kind !== "blob" || frame.data.byteLength !== expected) {
    throw new Error("Terminal attachment blob is invalid");
  }
  return frame.data;
}

const BLOB_TAG = 1;
const MAX_BUFFER_BYTES = 16 * 1024 * 1024;
const MAX_ITEMS = 4096;
const INITIAL_CAPACITY = 4096;

export class FrameQueue {
  private buffer = Buffer.alloc(0);
  private start = 0;
  private end = 0;
  private items: Frame[] = [];
  private waiters: Array<{ resolve: (frame: Frame) => void; reject: (error: Error) => void }> = [];
  private failure: Error | null = null;

  push(data: Uint8Array): void {
    if (this.failure) return;
    const incoming = Buffer.from(data);
    const required = this.end - this.start + incoming.length;
    if (required > MAX_BUFFER_BYTES) {
      this.fail(new Error("Hub attachment buffer limit exceeded"));
      return;
    }
    if (this.end + incoming.length > this.buffer.length) {
      let capacity = Math.max(this.buffer.length, INITIAL_CAPACITY);
      while (capacity < required) capacity *= 2;
      const next = Buffer.allocUnsafe(capacity);
      this.buffer.copy(next, 0, this.start, this.end);
      this.buffer = next;
      this.end = required - incoming.length;
      this.start = 0;
    }
    incoming.copy(this.buffer, this.end);
    this.end += incoming.length;
    this.parse();
  }

  close(error?: Error): void {
    this.fail(error ?? new Error("Hub attachment closed"));
  }

  async take(): Promise<Frame> {
    const item = this.items.shift();
    if (item) return item;
    if (this.failure) throw this.failure;
    return new Promise((resolve, reject) => this.waiters.push({ resolve, reject }));
  }

  private parse(): void {
    for (;;) {
      const available = this.end - this.start;
      if (available === 0) return;
      if (this.buffer[this.start] === BLOB_TAG) {
        if (available < 5) return;
        const length = this.buffer.readUInt32BE(this.start + 1);
        if (available < 5 + length) return;
        const data = this.buffer.subarray(this.start + 5, this.start + 5 + length);
        this.start += 5 + length;
        this.enqueue({ kind: "blob", data: Uint8Array.from(data) });
        continue;
      }
      const newline = this.buffer.subarray(this.start, this.end).indexOf(10);
      if (newline < 0) return;
      const line = this.buffer.subarray(this.start, this.start + newline).toString("utf8");
      this.start += newline + 1;
      try {
        this.enqueue({ kind: "json", value: JSON.parse(line) });
      } catch {
        this.fail(new Error("Hub attachment sent invalid JSON"));
        return;
      }
    }
  }

  private enqueue(frame: Frame): void {
    const waiter = this.waiters.shift();
    if (waiter) waiter.resolve(frame);
    else this.items.push(frame);
    if (this.items.length > MAX_ITEMS) this.fail(new Error("Hub attachment queue limit exceeded"));
  }

  private fail(error: Error): void {
    if (this.failure) return;
    this.failure = error;
    this.buffer = Buffer.alloc(0);
    this.start = 0;
    this.end = 0;
    const waiters = this.waiters.splice(0);
    for (const waiter of waiters) waiter.reject(error);
  }
}
