import type { ServerWebSocket } from "bun";
import type { AttachmentEvent, TerminalAttachment } from "../../sdk/src/index.ts";
import type { TerminalTarget } from "./target";
import { TerminalOutput } from "./terminalOutput";
import { openSeed } from "./terminalSeed";

/** The actor's one upstream output attachment and its shared fan-out. */
export class TerminalStream {
  private readonly output = new TerminalOutput();
  private attachment: TerminalAttachment | null = null;
  private generation = 0;
  private queue: Promise<void> = Promise.resolve();
  private closed = false;

  constructor(
    private readonly target: TerminalTarget,
    private readonly onClosed: () => void,
  ) {}

  get count(): number {
    return this.output.count;
  }

  history(limit: number): { data: Uint8Array; truncated: boolean } | null {
    return this.output.history(limit);
  }

  resize(cols: number, rows: number): void {
    this.output.resize(cols, rows);
  }

  attach(ws: ServerWebSocket<unknown>): void {
    this.output.subscribe(ws);
    this.ensure();
  }

  detach(ws: ServerWebSocket<unknown>): void {
    this.output.unsubscribe(ws);
  }

  close(): void {
    if (this.closed) return;
    this.closed = true;
    this.generation += 1;
    this.attachment?.close();
    this.attachment = null;
  }

  private ensure(): void {
    this.queue = this.queue.then(async () => {
      if (this.closed || this.output.count === 0) return;
      if (this.output.needsSeed) await this.seed();
    }).catch((error) => {
      this.output.error(error instanceof Error ? error.message : "Terminal stream failed");
      this.closed = true;
      this.onClosed();
    });
  }

  private async seed(): Promise<void> {
    this.attachment?.close();
    this.attachment = null;
    const { attachment, seed } = await openSeed(this.target);
    if (this.closed) {
      attachment.close();
      return;
    }
    this.attachment = attachment;
    this.output.install(seed);
    const generation = ++this.generation;
    void this.read(attachment, generation);
  }

  private async read(attachment: TerminalAttachment, generation: number): Promise<void> {
    for (;;) {
      if (this.closed || generation !== this.generation) return;
      let event: AttachmentEvent;
      try {
        event = await attachment.nextEvent();
      } catch (error) {
        if (!this.closed && generation === this.generation) {
          this.output.error(error instanceof Error ? error.message : "Terminal stream failed");
          this.closed = true;
          this.onClosed();
        }
        return;
      }
      if (generation !== this.generation) return;
      if (event.event === "output") this.output.publish(event.data);
      else if (event.event === "exited") {
        this.output.ended(event.code, event.signal);
        this.closed = true;
        this.onClosed();
        return;
      } else if (event.event === "resync") {
        this.attachment?.close();
        this.attachment = null;
        this.output.invalidate();
        this.ensure();
        return;
      } else if (event.event === "unreachable") {
        this.output.unreachable();
        this.closed = true;
        this.onClosed();
        return;
      }
    }
  }
}
