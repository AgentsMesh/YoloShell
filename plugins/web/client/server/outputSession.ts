import type { ServerWebSocket } from "bun";
import type { TerminalTarget } from "./target";
import { attachOutput, type OutputHandle } from "./terminalRegistry";

/** Edge adapter for one browser output socket; the shared actor owns the Hub stream. */
export class OutputSession {
  private handle: OutputHandle | null = null;

  constructor(
    private readonly ws: ServerWebSocket<unknown>,
    private readonly target: TerminalTarget,
  ) {}

  start(): void {
    this.handle = attachOutput(this.ws, this.target);
  }

  message(): void {
    this.ws.close(1008, "output socket is read-only");
  }

  close(): void {
    this.handle?.close();
    this.handle = null;
  }
}
