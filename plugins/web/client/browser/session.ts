import { loadToken } from "./api.js";
import { clientId } from "./clientIdentity.js";
import { reconnectDelay } from "./terminalReconnect.js";
import {
  WEB_PROTOCOL,
  type SessionClientFrame,
  type SessionConversationFrame,
  type SessionServerFrame,
  type WireAgentTarget,
} from "../shared/protocol.js";

type Handlers = {
  catalogue: (inventory: unknown) => void;
  cataloguePatch: (patch: unknown) => void;
  connected: () => void;
  disconnected: () => void;
  unauthorized: () => void;
  conversation: (message: SessionConversationFrame) => void;
};

function url(): string {
  const scheme = location.protocol === "https:" ? "wss" : "ws";
  return `${scheme}://${location.host}/ws/session`;
}

/** One browser tab's catalogue/event subscription; the server owns the poll. */
export class SessionClient {
  private socket: WebSocket | null = null;
  private timer: ReturnType<typeof setTimeout> | null = null;
  private attempts = 0;
  private closed = false;
  private watching: WireAgentTarget | null = null;
  private readonly id = clientId();

  constructor(private readonly handlers: Handlers) {}

  start(): void {
    this.closed = false;
    this.open();
  }

  refresh(): void {
    this.send({ type: "catalogue.refresh" });
  }

  snapshot(): void {
    this.send({ type: "catalogue.snapshot" });
  }

  watchConversation(binding: WireAgentTarget): void {
    this.watching = binding;
    this.send({ type: "conversation.watch", binding });
  }

  unwatchConversation(): void {
    this.watching = null;
    this.send({ type: "conversation.unwatch" });
  }

  close(): void {
    this.closed = true;
    if (this.timer) clearTimeout(this.timer);
    this.timer = null;
    this.socket?.close();
    this.socket = null;
  }

  private open(): void {
    if (this.closed) return;
    const token = loadToken();
    if (!token) {
      this.handlers.unauthorized();
      return;
    }
    const socket = new WebSocket(url(), WEB_PROTOCOL);
    this.socket = socket;
    socket.onopen = () => {
      this.send({ type: "hello", token, client_id: this.id });
    };
    socket.onmessage = (event) => {
      if (this.closed || typeof event.data !== "string") return;
      let message: SessionServerFrame;
      try {
        message = JSON.parse(event.data) as SessionServerFrame;
      } catch {
        return;
      }
      if (message?.type === "ready") {
        this.attempts = 0;
        if (this.watching) this.send({ type: "conversation.watch", binding: this.watching });
        this.handlers.connected();
      } else if (message?.type === "catalogue") {
        this.handlers.catalogue(message.inventory);
      } else if (message?.type === "catalogue.patch") {
        this.handlers.cataloguePatch(message.patch);
      } else if (
        message?.type === "conversation" ||
        message?.type === "conversation.unavailable" ||
        message?.type === "conversation.error"
      ) {
        this.handlers.conversation(message);
      }
    };
    socket.onclose = (event) => {
      if (this.closed) return;
      this.socket = null;
      if (event.code === 1008 && event.reason === "unauthorized") {
        this.closed = true;
        this.handlers.unauthorized();
        return;
      }
      this.handlers.disconnected();
      this.schedule();
    };
  }

  private schedule(): void {
    if (this.closed || this.timer) return;
    const delay = reconnectDelay(this.attempts++);
    this.timer = setTimeout(() => {
      this.timer = null;
      this.open();
    }, delay);
  }

  private send(value: SessionClientFrame): void {
    if (this.socket?.readyState === WebSocket.OPEN) this.socket.send(JSON.stringify(value));
  }
}
