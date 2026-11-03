const MAX_PENDING_BYTES = 64 * 1024;

/** Input typed before the writer lease or terminal stream is ready. */
export class PendingInput {
  private chunks: Uint8Array[] = [];
  private bytes = 0;

  push(data: Uint8Array): void {
    if (this.bytes + data.byteLength > MAX_PENDING_BYTES) return;
    this.chunks.push(Uint8Array.from(data));
    this.bytes += data.byteLength;
  }

  drain(): Uint8Array[] {
    const chunks = this.chunks;
    this.chunks = [];
    this.bytes = 0;
    return chunks;
  }

  clear(): void {
    this.chunks = [];
    this.bytes = 0;
  }
}
