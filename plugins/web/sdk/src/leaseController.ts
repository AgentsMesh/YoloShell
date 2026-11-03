import { HubClient } from "./hub";
import { HubError } from "./hubError.ts";

export type LeaseTarget = {
  address: { hub_id: string; terminal_id: string };
  runtime_epoch: string;
};
export type LeaseGrant = {
  generation?: number;
  expires_at_ms?: number;
  ttl_ms?: number;
  renew_after_ms?: number;
};
export type LeaseState = "observing" | "acquiring" | "controlled" | "lost" | "released";
export type LeaseListener = (state: LeaseState, grant: LeaseGrant | null) => void;

const RENEW_MIN_MS = 500;
const RENEW_MAX_MS = 5_000;

export class LeaseController {
  private grant: LeaseGrant | null = null;
  private renewTimer: ReturnType<typeof setTimeout> | null = null;
  private renewSerial = 0;
  private revision = 0;
  private current: LeaseState = "observing";
  private readonly principal = {
    client_id: crypto.randomUUID(),
    controller_instance_id: crypto.randomUUID(),
  };

  constructor(
    private readonly hub: HubClient,
    private readonly target: LeaseTarget,
    private readonly listener: LeaseListener,
  ) {}

  get state(): LeaseState {
    return this.current;
  }

  get currentGrant(): LeaseGrant | null {
    return this.grant;
  }

  /** The owning Hub is the routing destination for every lease request. */
  get origin(): string {
    return this.target.address.hub_id;
  }

  async observe(): Promise<number | null> {
    const reply = (await this.hub.observeLease(this.target, this.origin)) as {
      lease?: { generation?: unknown } | null;
    };
    const generation = reply.lease?.generation;
    return typeof generation === "number" ? generation : null;
  }

  async acquire(): Promise<void> {
    await this.lease({ mode: "acquire" });
  }

  async take(observedGeneration: number): Promise<void> {
    await this.lease({ mode: "take_control", observed_generation: observedGeneration });
  }

  async release(): Promise<void> {
    this.revision += 1;
    this.stopRenew();
    const grant = this.grant;
    this.grant = null;
    this.transition("released");
    if (!grant) return;
    try {
      await this.hub.release(grant, this.origin);
    } catch {
      // The grant expires on its own.
    }
  }

  /** Drops a lease this client no longer owns without waiting for expiry. */
  abandon(): void {
    this.revision += 1;
    this.stopRenew();
    this.grant = null;
    this.transition("lost");
  }

  private async lease(mode: Record<string, unknown>): Promise<void> {
    const revision = ++this.revision;
    this.stopRenew();
    this.transition("acquiring");
    try {
      const reply = (await this.hub.lease({
        request_id: crypto.randomUUID(),
        target: this.target,
        principal: this.principal,
        mode,
      }, this.origin)) as { grant?: LeaseGrant };
      if (revision !== this.revision) {
        if (reply.grant && this.grant === null) {
          try {
            await this.hub.release(reply.grant, this.origin);
          } catch {
            // The superseded grant expires on its own.
          }
        }
        throw new HubError(-32603, "Lease request was superseded", null);
      }
      if (!reply.grant) throw new HubError(-32603, "Lease grant is missing", null);
      this.grant = reply.grant;
      this.transition("controlled");
      this.scheduleRenew();
    } catch (error) {
      if (revision === this.revision) {
        this.grant = null;
        this.transition("lost");
      }
      throw error;
    }
  }

  private scheduleRenew(): void {
    this.stopRenew();
    const grant = this.grant;
    if (!grant) return;
    const delay = Math.max(
      RENEW_MIN_MS,
      Math.min(Number(grant.renew_after_ms ?? 3_000), RENEW_MAX_MS),
    );
    const serial = this.renewSerial;
    this.renewTimer = setTimeout(() => void this.renew(serial), delay);
  }

  private async renew(serial: number): Promise<void> {
    this.renewTimer = null;
    if (serial !== this.renewSerial) return;
    const grant = this.grant;
    const revision = this.revision;
    if (!grant) return;
    try {
      const reply = (await this.hub.renew(grant, this.origin)) as { grant?: LeaseGrant };
      if (revision !== this.revision || serial !== this.renewSerial) return;
      if (!reply.grant) throw new HubError(-32603, "Lease renewal is missing", null);
      this.grant = reply.grant;
      this.scheduleRenew();
    } catch {
      if (revision !== this.revision || serial !== this.renewSerial) return;
      this.grant = null;
      this.transition("lost");
    }
  }

  private stopRenew(): void {
    this.renewSerial += 1;
    if (this.renewTimer) clearTimeout(this.renewTimer);
    this.renewTimer = null;
  }

  private transition(state: LeaseState): void {
    this.current = state;
    this.listener(state, this.grant);
  }
}
