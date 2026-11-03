import type { ServerWebSocket } from "bun";
import type { TerminalTarget } from "./target";
import { attachControl, type ControlHandle } from "./terminalRegistry";

/** Edge adapter for one browser control socket; the actor owns the Hub lease. */
export class ControlSession {
  private handle: ControlHandle | null = null;

  constructor(
    private readonly ws: ServerWebSocket<unknown>,
    private readonly target: TerminalTarget,
    private readonly clientId: string,
  ) {}

  start(): void {
    this.handle = attachControl(this.ws, this.target, this.clientId);
  }

  message(message: string | Uint8Array): void {
    this.handle?.message(message);
  }

  close(): Promise<void> {
    this.handle?.close();
    this.handle = null;
    return Promise.resolve();
  }
}
