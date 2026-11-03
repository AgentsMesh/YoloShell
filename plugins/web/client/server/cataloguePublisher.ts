import {
  hubSummary,
  machinePatch,
  type MachinePatch,
} from "./cataloguePatch";
import type { MachineView } from "./machineView";

export type WirePatch = MachinePatch & {
  snapshot: { server: string; sequence: number };
};

export type CatalogueEvent =
  | { kind: "snapshot"; inventory: unknown }
  | { kind: "patch"; patch: WirePatch };

/**
 * Publishes one aggregate catalogue where every machine advances through an
 * epoch and contiguous revisions. A change that cannot be diffed becomes a full
 * snapshot; a revision-only poll publishes nothing.
 */
export class CataloguePublisher {
  private readonly server = crypto.randomUUID();
  private sequence = 0;
  private readonly published = new Map<string, MachineView>();
  private current: unknown = null;

  constructor(private readonly entryMachine: string) {}

  latest(): unknown {
    return this.current;
  }

  publish(views: MachineView[], changed: MachineView): CatalogueEvent | null {
    const previous = this.published.get(changed.machine);
    if (!previous) {
      this.published.set(changed.machine, changed);
      this.addMissing(views);
      return this.snapshotEvent();
    }
    const decision = machinePatch(previous, changed);
    if (decision.kind === "none") return null;
    this.published.set(changed.machine, changed);
    if (decision.kind === "snapshot") {
      this.addMissing(views);
      return this.snapshotEvent();
    }
    this.bump();
    return {
      kind: "patch",
      patch: {
        ...decision.patch,
        snapshot: { server: this.server, sequence: this.sequence },
      },
    };
  }

  /** A gap repair is newer than every frame the requester may have applied. */
  forceSnapshot(): CatalogueEvent | null {
    if (!this.current) return null;
    this.bump();
    return { kind: "snapshot", inventory: this.current };
  }

  reset(): void {
    this.published.clear();
    this.current = null;
  }

  private addMissing(views: MachineView[]): void {
    for (const view of views) {
      if (!this.published.has(view.machine)) this.published.set(view.machine, view);
    }
  }

  private snapshotEvent(): CatalogueEvent {
    this.bump();
    return { kind: "snapshot", inventory: this.current };
  }

  private bump(): void {
    this.sequence += 1;
    this.current = this.inventory();
  }

  private inventory(): unknown {
    const views = [...this.published.values()].sort((left, right) => {
      if (left.entry) return -1;
      if (right.entry) return 1;
      return left.label.localeCompare(right.label);
    });
    return {
      snapshot: { server: this.server, sequence: this.sequence },
      machine: this.entryMachine,
      hubs: { hubs: views.map(hubSummary) },
      terminals: { terminals: views.flatMap((view) => view.terminals) },
      agents: { agents: views.flatMap((view) => view.agents) },
    };
  }
}
