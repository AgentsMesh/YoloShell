import { HubClient, type HubEndpoint } from "./hub";
import { InputQueue } from "./inputQueue.ts";
import {
  LeaseController,
  type LeaseGrant,
  type LeaseListener,
  type LeaseState,
  type LeaseTarget,
} from "./leaseController.ts";

export type SessionTarget = LeaseTarget;
export type SessionState = LeaseState;
export type SessionGrant = LeaseGrant;
export type SessionListener = LeaseListener;

export class TerminalSession {
  private constructor(
    private readonly lease: LeaseController,
    private readonly input: InputQueue,
    private readonly hubs: HubClient[],
  ) {}

  static async open(
    endpoint: HubEndpoint,
    target: SessionTarget,
    onChange: SessionListener,
    onError: (error: Error) => void,
  ): Promise<TerminalSession> {
    const results = await Promise.allSettled([
      HubClient.connect(endpoint),
      HubClient.connect(endpoint),
      HubClient.connect(endpoint),
    ]);
    const hubs: HubClient[] = [];
    for (const result of results) {
      if (result.status === "fulfilled") hubs.push(result.value);
    }
    const failed = results.find((result) => result.status === "rejected");
    if (failed?.status === "rejected") {
      for (const hub of hubs) hub.close();
      throw failed.reason;
    }
    const [leaseHub, inputHub, controlHub] = hubs as [HubClient, HubClient, HubClient];
    const lease = new LeaseController(leaseHub, target, onChange);
    const input = new InputQueue(inputHub, controlHub, lease, onError);
    return new TerminalSession(lease, input, hubs);
  }

  get state(): SessionState {
    return this.lease.state;
  }

  /** Every long-lived Hub socket must be usable before a lease action retries. */
  get connected(): boolean {
    return this.hubs.every((hub) => hub.connected);
  }

  get grant(): SessionGrant | null {
    return this.lease.currentGrant;
  }

  observe(): Promise<number | null> {
    return this.lease.observe();
  }

  acquire(): Promise<void> {
    return this.lease.acquire();
  }

  take(generation: number): Promise<void> {
    return this.lease.take(generation);
  }

  abandon(): void {
    this.lease.abandon();
  }

  release(): Promise<void> {
    return this.lease.release();
  }

  enqueue(data: Uint8Array): void {
    this.input.enqueue(data);
  }

  resize(cols: number, rows: number): Promise<void> {
    return this.input.resize(cols, rows);
  }

  async close(): Promise<void> {
    await this.lease.release();
    for (const hub of this.hubs) hub.close();
  }
}
