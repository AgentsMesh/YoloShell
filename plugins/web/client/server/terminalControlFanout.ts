import type { ServerWebSocket } from "bun";
import type { TerminalControlFrame, WireLeaseState } from "../shared/protocol";

type Sink = { ws: ServerWebSocket<unknown>; clientId: string };
export type LeaseState = WireLeaseState;

/** One actor's control plane: one Hub lease, many local tab intents. */
export class TerminalControlFanout {
  private readonly sinks = new Map<string, Sink>();
  private controller: string | null = null;
  private acquiringClient: string | null = null;
  private state: LeaseState = "observing";

  get count(): number {
    return this.sinks.size;
  }

  get controlledBy(): string | null {
    return this.controller;
  }

  /** Whether this tab holds (or is acquiring) the local control slot. */
  owns(clientId: string): boolean {
    return this.controller === clientId || this.acquiringClient === clientId;
  }

  add(ws: ServerWebSocket<unknown>, clientId: string): void {
    const previous = this.sinks.get(clientId);
    if (previous && previous.ws !== ws) this.sinks.delete(clientId);
    this.sinks.set(clientId, { ws, clientId });
    this.send(ws, { type: "ready" });
    this.sendLease(clientId);
    this.sendPresence();
  }

  remove(ws: ServerWebSocket<unknown>, clientId: string): boolean {
    const sink = this.sinks.get(clientId);
    if (!sink || sink.ws !== ws) return false;
    this.sinks.delete(clientId);
    if (this.controller === clientId) {
      this.controller = null;
      this.broadcastLease();
      return true;
    }
    this.broadcastPresence();
    return false;
  }

  assign(clientId: string): void {
    this.controller = clientId;
    this.acquiringClient = null;
    this.state = "controlled";
    this.broadcastLease();
  }

  acquire(clientId: string): void {
    this.controller = null;
    this.acquiringClient = clientId;
    this.state = "acquiring";
    this.broadcastLease();
  }

  deny(clientId: string, message: string): void {
    const sink = this.sinks.get(clientId);
    if (sink) this.send(sink.ws, { type: "error", message });
    this.acquiringClient = null;
    this.sendLease(clientId);
  }

  release(): void {
    this.controller = null;
    this.acquiringClient = null;
    this.state = "released";
    this.broadcastLease();
  }

  lost(): void {
    this.controller = null;
    this.acquiringClient = null;
    this.state = "lost";
    this.broadcastLease();
  }

  observe(): void {
    this.controller = null;
    this.acquiringClient = null;
    this.state = "observing";
    this.broadcastLease();
  }

  error(message: string): void {
    for (const sink of this.sinks.values()) this.send(sink.ws, { type: "error", message });
  }

  private broadcastLease(): void {
    for (const clientId of this.sinks.keys()) this.sendLease(clientId);
    this.broadcastPresence();
  }

  private sendLease(clientId: string): void {
    const sink = this.sinks.get(clientId);
    if (!sink) return;
    const controlled = this.state === "controlled" && this.controller === clientId;
    const state =
      controlled
        ? "controlled"
        : this.state === "acquiring"
          ? this.acquiringClient === clientId
            ? "acquiring"
            : "observing"
          : this.state === "controlled"
            ? "observing"
            : this.state;
    this.send(sink.ws, {
      type: "lease",
      state,
      controller: this.controller === clientId,
      controlled: this.controller !== null,
      viewers: this.sinks.size,
    });
  }

  private sendPresence(): void {
    for (const sink of this.sinks.values()) {
      this.send(sink.ws, {
        type: "presence",
        viewers: this.sinks.size,
        controlled: this.controller !== null,
        mine: this.controller === sink.clientId,
      });
    }
  }

  private broadcastPresence(): void {
    this.sendPresence();
  }

  private send(ws: ServerWebSocket<unknown>, value: TerminalControlFrame): void {
    if (ws.readyState === 1) ws.send(JSON.stringify(value));
  }
}
