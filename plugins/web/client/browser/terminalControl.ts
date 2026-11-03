import type { TerminalControlHost } from "./terminalControlHost.js";
import {
  controlError,
  input as inputEvent,
  lease as leaseEvent,
  ready as readyEvent,
  release as releaseEvent,
  requestControl as requestEvent,
} from "./terminalControlLease.js";
import {
  dispose as disposeMachine,
  initial,
  presentation,
  reset as resetMachine,
  setStream as streamEvent,
  type Effect,
  type Machine,
} from "./terminalControlState.js";
import type { ControlState } from "./terminalControlStatus.js";
import type { TerminalControlFrame } from "../shared/protocol.js";

export type TerminalControlState = ControlState;

/**
 * Lease/input intent for one terminal view. Selection is read-only; typing
 * while no other tab controls takes the lease and buffers the first bytes.
 * Another controller requires an explicit takeover.
 */
export class TerminalControl {
  private machine: Machine;
  private readonly host: TerminalControlHost;

  constructor(host: TerminalControlHost, autoTake: boolean) {
    this.host = host;
    this.machine = initial(autoTake);
  }

  get controlled(): boolean {
    return this.machine.lease === "controlled";
  }

  resetConnection(): void {
    this.run(resetMachine(this.machine));
    this.host.setStatus("Connecting…");
    this.host.showTake(false);
  }

  setStream(live: boolean, seeding: boolean): void {
    this.run(streamEvent(this.machine, live, seeding));
  }

  control(message: TerminalControlFrame): void {
    if (this.machine.disposed) return;
    if (message?.type === "ready") {
      this.run(readyEvent(this.machine));
    } else if (message?.type === "lease") {
      this.run(leaseEvent(this.machine, message));
    } else if (message?.type === "error") {
      this.run(
        controlError(
          this.machine,
          typeof message.message === "string"
            ? message.message
            : "Terminal control failed",
        ),
      );
    }
  }

  input(data: Uint8Array): void {
    this.run(inputEvent(this.machine, data));
  }

  release(): void {
    this.run(releaseEvent(this.machine));
  }

  requestControl(): void {
    this.run(requestEvent(this.machine));
  }

  dispose(): void {
    this.machine = disposeMachine(this.machine);
  }

  private run(result: { machine: Machine; effects: Effect[] }): void {
    this.machine = result.machine;
    for (const effect of result.effects) this.apply(effect);
  }

  private apply(effect: Effect): void {
    switch (effect.kind) {
      case "take":
        this.host.sendTake();
        return;
      case "release":
        this.host.sendRelease();
        return;
      case "input":
        this.host.sendInput(effect.data);
        return;
      case "stdin":
        this.host.setStdin(effect.enabled);
        return;
      case "focus":
        this.host.focus();
        return;
      case "fit":
        this.host.fit();
        return;
      case "clearError":
        this.host.clearError();
        return;
      case "showError":
        this.host.showError(effect.message);
        return;
      case "render":
        this.render();
        return;
    }
  }

  private render(): void {
    const view = presentation(this.machine);
    this.host.showTake(view.showTake);
    this.host.setStatus(view.status, view.active);
  }
}

