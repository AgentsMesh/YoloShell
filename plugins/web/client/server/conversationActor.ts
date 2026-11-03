import { HubError } from "../../sdk/src/index.ts";
import { read, watch } from "./hub";
import type { TerminalTarget } from "./target";
import type { SessionConversationFrame } from "../shared/protocol";

export type Binding = {
  agent: { address: { hub_id: string; agent_id: string }; execution_id: string };
  runtime: TerminalTarget;
};
type Subscriber = { send(frame: SessionConversationFrame): void };

const MAX_ENTRIES = 64;
const MAX_BYTES = 262_144;
const MAX_RETRIES = 6;
const RETRYABLE = new Set(["evidence_unavailable", "reader_busy"]);

function reason(error: unknown): string | null {
  if (!(error instanceof HubError) || error.code !== -32001) return null;
  const data = error.data as { error?: unknown } | null;
  return typeof data?.error === "string" ? data.error : "conversation_unavailable";
}

function wait(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/** Keep a live watch alive after the short retry window, with a slow probe. */
export function watchRetry(retries: number): { wait: number; announce: boolean } {
  return retries > MAX_RETRIES
    ? { wait: 5000, announce: true }
    : { wait: 500 + retries * 200, announce: false };
}

/** One watch per agent execution; every tab subscribes to the same pages. */
export class ConversationActor {
  private readonly subscribers = new Set<Subscriber>();
  private address: unknown = null;
  private since: { generation?: number[]; revision?: number[] } | null = null;
  private lastPage: unknown = null;
  private reported = false;
  private closed = false;

  constructor(
    readonly key: string,
    private readonly binding: Binding,
    private readonly onClose: () => void,
  ) {}

  add(subscriber: Subscriber): void {
    if (this.closed) return;
    this.subscribers.add(subscriber);
    if (this.lastPage) {
      subscriber.send({
        type: "conversation",
        key: this.key,
        reading: { status: "page", page: this.lastPage },
      });
    }
    if (this.subscribers.size === 1) void this.start();
  }

  remove(subscriber: Subscriber): void {
    this.subscribers.delete(subscriber);
    if (this.subscribers.size === 0) this.close();
  }

  close(): void {
    if (this.closed) return;
    this.closed = true;
    this.subscribers.clear();
    this.onClose();
  }

  private send(frame: SessionConversationFrame): void {
    for (const subscriber of this.subscribers) subscriber.send(frame);
  }

  private async start(): Promise<void> {
    const destination = this.binding.agent.address.hub_id;
    let retries = 0;
    while (!this.closed && this.subscribers.size > 0) {
      try {
        const reply = (await read(destination, (hub) =>
          hub.resolveConversation(this.binding, destination),
        )) as { conversation?: { address?: unknown } };
        this.address = reply.conversation?.address;
        if (!this.address) throw new HubError(-32000, "Conversation descriptor is missing", null);
        break;
      } catch (error) {
        const missing = reason(error) ?? "conversation_unavailable";
        if (RETRYABLE.has(missing) && retries < MAX_RETRIES) {
          retries += 1;
          await wait(500 + retries * 250);
          continue;
        }
        this.unavailable(missing);
        return;
      }
    }
    if (!this.address) return;
    while (!this.closed && this.subscribers.size > 0) {
      const request = {
        address: this.address,
        binding: this.binding,
        cursor: null,
        limits: { entries: MAX_ENTRIES, bytes: MAX_BYTES },
        since_revision: this.since?.revision ?? null,
        since_generation: this.since?.generation ?? null,
      };
      try {
        const reply = (await watch(destination, (hub) =>
          hub.watchConversation(request, 1000, destination),
        )) as { reading?: { status?: string; page?: any } };
        retries = 0;
        const reading = reply.reading;
        if (!reading) throw new HubError(-32000, "Conversation read is missing", null);
        if (reading.status === "page" && reading.page) {
          this.since = {
            generation: reading.page.history_generation,
            revision: reading.page.revision,
          };
          this.lastPage = reading.page;
          this.send({ type: "conversation", key: this.key, reading });
        }
      } catch (error) {
        const missing = reason(error);
        if (missing) {
          if (missing === "history_changed" || missing === "binding_changed") {
            this.since = null;
            await wait(250);
            continue;
          }
          // A binding update at a turn boundary is briefly invisible to the
          // live witness; retry instead of ending the watch for the session.
          if (RETRYABLE.has(missing)) {
            retries += 1;
            // A long provider turn can hide the live witness for more than
            // the initial retry window. Keep the subscription alive and
            // surface one transient note instead of ending the watch.
            const policy = watchRetry(retries);
            if (policy.announce && !this.reported) {
              this.reported = true;
              this.send({
                type: "conversation.error",
                key: this.key,
                message: "Conversation is temporarily unavailable; retrying",
              });
            }
            await wait(policy.wait);
            continue;
          }
          this.unavailable(missing);
          return;
        }
        this.send({ type: "conversation.error", key: this.key, message: "Conversation read failed" });
        await wait(1500);
      }
    }
  }

  private unavailable(value: string): void {
    this.send({ type: "conversation.unavailable", key: this.key, reason: value });
    this.close();
  }
}
