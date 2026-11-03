import { clientId } from "./clientIdentity.js";
import {
  WEB_PROTOCOL,
  type TerminalClientFrame,
  type TerminalControlFrame,
  type TerminalOutputFrame,
  type TerminalServerFrame,
  type WireTerminalTarget,
} from "../shared/protocol.js";
import {
  HEARTBEAT_PING,
  HEARTBEAT_PING_MS,
  HEARTBEAT_PROBE_MS,
  heartbeatStale,
} from "./heartbeat.js";

export type TerminalTarget = WireTerminalTarget;
export type TerminalRole = "output" | "control";

type Callbacks = {
  outputText: (message: TerminalOutputFrame) => void;
  outputBytes: (data: Uint8Array) => void;
  controlText: (message: TerminalControlFrame) => void;
  closed: (role: TerminalRole, event: CloseEvent) => void;
};

const INPUT_CHUNK = 64 * 1024;
const ROLES: TerminalRole[] = ["output", "control"];

function urlFor(path: string): string {
  const scheme = location.protocol === "https:" ? "wss" : "ws";
  return `${scheme}://${location.host}${path}`;
}

export class TerminalSockets {
  private readonly output: WebSocket;
  private readonly control: WebSocket;
  private readonly lastSeen: Record<TerminalRole, number> = {
    output: Date.now(),
    control: Date.now(),
  };
  private disposed = false;
  private reported = false;
  private heartbeat: ReturnType<typeof setInterval> | null = null;
  private probeTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(
    private readonly target: TerminalTarget,
    private readonly token: string,
    private readonly callbacks: Callbacks,
  ) {
    this.output = this.open("/ws/terminal", "output");
    this.control = this.open("/ws/control", "control");
    this.heartbeat = setInterval(() => this.check(), HEARTBEAT_PING_MS);
    document.addEventListener("visibilitychange", this.onVisible);
    window.addEventListener("online", this.onOnline);
  }

  input(data: Uint8Array): void {
    for (let offset = 0; offset < data.byteLength; offset += INPUT_CHUNK) {
      this.send(
        this.control,
        data.subarray(offset, Math.min(offset + INPUT_CHUNK, data.byteLength)),
      );
    }
  }

  resize(cols: number, rows: number): void {
    this.send(this.control, JSON.stringify({ type: "resize", cols, rows }));
  }

  take(): void {
    this.send(this.control, JSON.stringify({ type: "take" }));
  }

  release(): void {
    this.send(this.control, JSON.stringify({ type: "release" }));
  }

  close(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.stopTimers();
    this.output.close();
    this.control.close();
  }

  private open(path: string, role: TerminalRole): WebSocket {
    const socket = new WebSocket(urlFor(path), WEB_PROTOCOL);
    socket.binaryType = "arraybuffer";
    socket.onopen = () => {
      this.lastSeen[role] = Date.now();
      const hello: TerminalClientFrame = {
        type: "hello",
        token: this.token,
        client_id: clientId(),
        target: this.target,
      };
      this.send(socket, JSON.stringify(hello));
    };
    socket.onmessage = (event) => {
      if (this.disposed) return;
      this.lastSeen[role] = Date.now();
      if (typeof event.data === "string") {
        let message: TerminalServerFrame;
        try {
          message = JSON.parse(event.data) as TerminalServerFrame;
        } catch {
          return;
        }
        if (message.type === "pong") return;
        if (role === "output") {
          this.callbacks.outputText(message as TerminalOutputFrame);
        } else {
          this.callbacks.controlText(message as TerminalControlFrame);
        }
      } else if (role === "output") {
        this.callbacks.outputBytes(new Uint8Array(event.data as ArrayBuffer));
      }
    };
    socket.onerror = () => this.reportClosed(role);
    socket.onclose = (event) => {
      if (!this.disposed && !this.reported) this.callbacks.closed(role, event);
    };
    return socket;
  }

  /**
   * A terminal can be idle for hours. Without a frame both ways, a dropped
   * LAN/NAT flow or a frozen background tab looks open forever and silently
   * eats input; the app-level ping makes that state recoverable.
   */
  private check(): void {
    if (this.disposed || this.reported) return;
    const now = Date.now();
    for (const role of ROLES) {
      const socket = role === "output" ? this.output : this.control;
      if (socket.readyState === WebSocket.CONNECTING) continue;
      if (socket.readyState !== WebSocket.OPEN || heartbeatStale(this.lastSeen[role], now)) {
        this.reportClosed(role);
        return;
      }
      this.send(socket, HEARTBEAT_PING);
    }
  }

  /** Hidden tabs freeze timers; probe once on return instead of waiting 45s. */
  private probe(): void {
    if (this.disposed || this.reported || this.probeTimer) return;
    const before = { ...this.lastSeen };
    for (const role of ROLES) {
      const socket = role === "output" ? this.output : this.control;
      if (socket.readyState === WebSocket.CONNECTING) continue;
      if (socket.readyState !== WebSocket.OPEN) {
        this.reportClosed(role);
        return;
      }
      this.send(socket, HEARTBEAT_PING);
    }
    this.probeTimer = setTimeout(() => {
      this.probeTimer = null;
      if (this.disposed || this.reported) return;
      if (this.lastSeen.output === before.output || this.lastSeen.control === before.control) {
        this.reportClosed("control");
      }
    }, HEARTBEAT_PROBE_MS);
  }

  private readonly onVisible = (): void => {
    if (document.visibilityState === "visible") this.probe();
  };

  private readonly onOnline = (): void => this.probe();

  private reportClosed(role: TerminalRole): void {
    if (this.disposed || this.reported) return;
    this.reported = true;
    this.stopTimers();
    this.callbacks.closed(role, {
      code: 4000,
      reason: "heartbeat",
      wasClean: false,
    } as CloseEvent);
  }

  private stopTimers(): void {
    if (this.heartbeat) clearInterval(this.heartbeat);
    this.heartbeat = null;
    if (this.probeTimer) clearTimeout(this.probeTimer);
    this.probeTimer = null;
    document.removeEventListener("visibilitychange", this.onVisible);
    window.removeEventListener("online", this.onOnline);
  }

  private send(socket: WebSocket, data: string | Uint8Array): void {
    if (this.disposed || socket.readyState !== WebSocket.OPEN) return;
    socket.send(data);
  }
}
