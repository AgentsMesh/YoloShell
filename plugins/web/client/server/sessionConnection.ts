import type { ServerWebSocket } from "bun";
import { getCatalogue } from "./catalogueService";
import { watchConversation } from "./conversationRegistry";
import { sanitize } from "./sanitize";
import { agentTarget } from "./validate";
import type { SessionServerFrame } from "../shared/protocol";

type Connection = { role: string };

/** One browser tab's catalogue/session subscription. */
export class SessionConnection {
  private readonly catalogue = getCatalogue();
  private unsubscribe: (() => void) | null = null;
  private conversation: { stop(): void } | null = null;
  private closed = false;

  constructor(
    private readonly ws: ServerWebSocket<Connection>,
    readonly clientId: string,
  ) {}

  start(): void {
    if (this.closed) return;
    this.unsubscribe = this.catalogue.subscribe((event) => {
      if (event.kind === "snapshot") {
        this.send({ type: "catalogue", inventory: event.inventory });
      } else {
        this.send({ type: "catalogue.patch", patch: event.patch });
      }
    });
    this.send({ type: "ready", client_id: this.clientId });
  }

  message(message: string | Uint8Array): void {
    if (this.closed || typeof message !== "string") return;
    let value: { type?: unknown };
    try {
      value = JSON.parse(message) as { type?: unknown };
    } catch {
      return;
    }
    if (value.type === "catalogue.refresh") {
      this.catalogue.invalidate();
      return;
    }
    if (value.type === "catalogue.snapshot") {
      const event = this.catalogue.resync();
      if (event?.kind === "snapshot") {
        this.send({ type: "catalogue", inventory: event.inventory });
      } else {
        this.catalogue.invalidate();
      }
      return;
    }
    if (value.type === "conversation.watch") {
      this.conversation?.stop();
      this.conversation = null;
      try {
        const binding = agentTarget((value as { binding?: unknown }).binding);
        this.conversation = watchConversation(binding, {
          send: (frame) => this.send(sanitize(frame)),
        });
      } catch (error) {
        this.send({
          type: "conversation.error",
          message: error instanceof Error ? error.message : "Invalid conversation target",
        });
      }
      return;
    }
    if (value.type === "conversation.unwatch") {
      this.conversation?.stop();
      this.conversation = null;
    }
  }

  close(): void {
    this.closed = true;
    this.unsubscribe?.();
    this.unsubscribe = null;
    this.conversation?.stop();
    this.conversation = null;
  }

  private send(value: SessionServerFrame): void {
    if (this.closed) return;
    const amount = (this.ws as unknown as { getBufferedAmount?: () => number }).getBufferedAmount?.() ?? 0;
    if (amount > 8 * 1024 * 1024) {
      this.ws.close(1013, "subscriber is too slow");
      return;
    }
    this.ws.send(JSON.stringify(value));
  }
}
