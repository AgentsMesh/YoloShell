import { HubError, TerminalSession, type SessionState } from "../../sdk/src/index.ts";
import { hubEndpoint } from "./hub";
import type { TerminalTarget } from "./target";

/** The actor's single Hub lease/input session; tabs only provide intents. */
export class TerminalLease {
  private session: TerminalSession | null = null;
  private opening: Promise<TerminalSession> | null = null;
  private readonly connect: () => Promise<TerminalSession>;

  constructor(
    target: TerminalTarget,
    onChange: (state: SessionState) => void,
    onError: (error: Error) => void,
    connect?: () => Promise<TerminalSession>,
  ) {
    this.connect =
      connect ??
      (() => TerminalSession.open(hubEndpoint(), target, (state) => onChange(state), onError));
  }

  get state(): SessionState {
    return this.session?.state ?? "observing";
  }

  get connected(): boolean {
    return this.session?.connected ?? false;
  }

  async take(): Promise<boolean> {
    let session = await this.open();
    // A Hub restart closes the SDK's three private sockets without changing
    // the actor or its browser sockets. Rebuild them before reporting failure.
    if (!session.connected) {
      await this.release();
      session = await this.open();
    }
    try {
      return await this.attempt(session);
    } catch (error) {
      if (error instanceof HubError) throw error;
      await this.release();
      session = await this.open();
      return this.attempt(session);
    }
  }

  enqueue(data: Uint8Array): void {
    if (this.session?.state === "controlled") this.session.enqueue(data);
  }

  resize(cols: number, rows: number): Promise<void> {
    return this.session?.resize(cols, rows) ?? Promise.resolve();
  }

  async release(): Promise<void> {
    const session = this.session;
    this.session = null;
    this.opening = null;
    await session?.close();
  }

  /** Drops a session whose Hub sockets failed; the next take opens a fresh one. */
  async drop(): Promise<void> {
    const session = this.session;
    this.session = null;
    this.opening = null;
    await session?.close();
  }

  close(): Promise<void> {
    return this.release();
  }

  private async attempt(session: TerminalSession): Promise<boolean> {
    if (session.state === "controlled") return true;
    const generation = await session.observe();
    if (generation === null) await session.acquire();
    else await session.take(generation);
    return session.state === "controlled";
  }

  private async open(): Promise<TerminalSession> {
    if (this.session) return this.session;
    if (!this.opening) {
      this.opening = this.connect();
    }
    try {
      this.session = await this.opening;
      return this.session;
    } finally {
      this.opening = null;
    }
  }
}
