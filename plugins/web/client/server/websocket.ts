import type { ServerWebSocket } from "bun";
import { hostAllowed, originAllowed } from "./access";
import { sameToken } from "./auth";
import { ControlSession } from "./controlSession";
import { SessionConnection } from "./sessionConnection";
import { OutputSession } from "./outputSession";
import { terminalTarget } from "./target";
import { closeTerminals } from "./terminalRegistry";
import { WEB_PROTOCOL } from "../shared/protocol";

const HELLO_TIMEOUT_MS = 10_000;
const MAX_SESSIONS = 256;
const debug = process.env.YOLO_WEB_DEBUG === "1";
type Role = "output" | "control" | "session";

type Connection = {
  role: Role;
  deadline?: ReturnType<typeof setTimeout>;
  output?: OutputSession;
  control?: ControlSession;
  session?: SessionConnection;
};

export class TerminalHub {
  private readonly sessions = new Set<ServerWebSocket<Connection>>();

  constructor(
    private readonly token: string,
    private readonly port: number,
    private readonly allowedHosts: Set<string>,
  ) {}

  readonly websocket = {
    open: (ws: ServerWebSocket<Connection>) => this.open(ws),
    message: (ws: ServerWebSocket<Connection>, message: string | Uint8Array) =>
      this.message(ws, message),
    close: (ws: ServerWebSocket<Connection>) => this.close(ws),
    maxPayloadLength: 64 * 1024,
    idleTimeout: 120,
  };

  upgrade(request: Request, server: import("bun").Server): Response | undefined | null {
    const url = new URL(request.url);
    const role =
      url.pathname === "/ws/terminal"
        ? "output"
        : url.pathname === "/ws/control"
          ? "control"
          : url.pathname === "/ws/session"
            ? "session"
            : null;
    if (role === null) return null;
    const host = request.headers.get("host");
    const origin = request.headers.get("origin");
    const protocols = (request.headers.get("sec-websocket-protocol") ?? "")
      .split(",")
      .map((value) => value.trim());
    if (debug) {
      console.error("ws upgrade", url.pathname, { host, origin, protocols });
    }
    if (!hostAllowed(host, this.port, this.allowedHosts)) return forbidden();
    if (origin === null || !originAllowed(origin, host)) return forbidden();
    if (!protocols.includes(WEB_PROTOCOL)) return new Response("subprotocol required", { status: 400 });
    if (!server.upgrade(request, { data: { role } })) return new Response("upgrade failed", { status: 400 });
    return undefined;
  }

  private open(ws: ServerWebSocket<Connection>): void {
    if (debug) console.error("ws open", ws.data?.role);
    ws.data.deadline = setTimeout(() => ws.close(1008, "hello timeout"), HELLO_TIMEOUT_MS);
  }

  private message(ws: ServerWebSocket<Connection>, message: string | Uint8Array): void {
    if (typeof message === "string" && authenticated(ws) && heartbeatPing(message)) {
      ws.send('{"type":"pong"}');
      return;
    }
    if (ws.data.output) return ws.data.output.message();
    if (ws.data.control) {
      ws.data.control.message(message);
      return;
    }
    if (ws.data.session) {
      ws.data.session.message(message);
      return;
    }
    if (debug) {
      console.error("ws first message", ws.data.role, typeof message, String(message).slice(0, 160));
    }
    if (typeof message !== "string") {
      ws.close(1008, "hello required");
      return;
    }
    clearTimeout(ws.data.deadline);
    ws.data.deadline = undefined;
    let value: Record<string, unknown>;
    try {
      value = JSON.parse(message);
    } catch {
      ws.close(1008, "invalid hello");
      return;
    }
    if (value.type !== "hello" || typeof value.token !== "string" || !sameToken(value.token, this.token)) {
      ws.close(1008, "unauthorized");
      return;
    }
    if (this.sessions.size >= MAX_SESSIONS) {
      ws.close(1013, "terminal capacity reached");
      return;
    }
    const clientId =
      typeof value.client_id === "string" && value.client_id.length <= 128
        ? value.client_id
        : crypto.randomUUID();
    if (ws.data.role === "session") {
      const session = new SessionConnection(ws, clientId);
      ws.data.session = session;
      this.sessions.add(ws);
      session.start();
      return;
    }
    let target;
    try {
      target = terminalTarget(value.target);
    } catch (error) {
      if (debug) console.error("ws invalid target", error);
      ws.close(1008, "invalid terminal target");
      return;
    }
    try {
      if (ws.data.role === "control") {
        const session = new ControlSession(ws, target, clientId);
        ws.data.control = session;
        this.sessions.add(ws);
        void session.start();
      } else {
        const session = new OutputSession(ws, target);
        ws.data.output = session;
        this.sessions.add(ws);
        void session.start();
      }
    } catch {
      ws.close(1011, "terminal startup failed");
    }
  }

  private close(ws: ServerWebSocket<Connection>): void {
    if (debug) console.error("ws close", ws.data.role);
    clearTimeout(ws.data.deadline);
    this.sessions.delete(ws);
    ws.data.output?.close();
    void ws.data.control?.close();
    ws.data.session?.close();
  }

  /** Closes every browser session and releases its lease before the process exits. */
  async shutdown(): Promise<void> {
    const pending: Promise<void>[] = [];
    for (const ws of this.sessions) {
      clearTimeout(ws.data.deadline);
      ws.data.output?.close();
      ws.data.session?.close();
      pending.push(ws.data.control?.close() ?? Promise.resolve());
      ws.close(1001, "server shutting down");
    }
    this.sessions.clear();
    closeTerminals();
    const timeout = new Promise<void>((resolve) => setTimeout(resolve, 2000));
    await Promise.race([Promise.allSettled(pending), timeout]);
  }
}

function authenticated(ws: ServerWebSocket<Connection>): boolean {
  return Boolean(ws.data.output || ws.data.control || ws.data.session);
}

/** Transport keepalive only; never routed to a terminal or session actor. */
function heartbeatPing(message: string): boolean {
  if (message.length > 64 || !message.includes('"ping"')) return false;
  try {
    const value = JSON.parse(message) as { type?: unknown } | null;
    return value?.type === "ping";
  } catch {
    return false;
  }
}

function forbidden(): Response {
  return new Response("forbidden", { status: 403 });
}
