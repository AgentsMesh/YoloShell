import { byId } from "./dom.js";
import { loadToken } from "./api.js";
import { TerminalChrome } from "./terminalChrome.js";
import { TerminalControl } from "./terminalControl.js";
import { TerminalInputBinding } from "./terminalInputBinding.js";
import { reconnectDelay } from "./terminalReconnect.js";
import { TERMINAL_OPTIONS } from "./terminalTheme.js";
import { TerminalSockets, type TerminalTarget } from "./terminalSocket.js";
import type { TerminalOutputFrame } from "../shared/protocol.js";
import { FitCtor, TerminalCtor, type Xterm } from "./terminalXterm.js";
import {
  begin,
  finish,
  initial,
  isSeeding,
  seed,
  settleLive,
} from "./terminalStreamState.js";

export class TerminalView {
  private readonly host = byId("terminal-host");
  private readonly chrome = new TerminalChrome();
  private readonly control: TerminalControl;
  private readonly input: TerminalInputBinding;
  private term: Xterm | null = null;
  private fit: { fit(): void } | null = null;
  private sockets: TerminalSockets | null = null;
  private observer: ResizeObserver | null = null;
  private disposed = false;
  private stream = initial();
  private reconnectAttempts = 0;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(
    private readonly target: TerminalTarget,
    autoTake = false,
  ) {
    this.control = new TerminalControl(
      {
        sendTake: () => this.sockets?.take(),
        sendRelease: () => this.sockets?.release(),
        sendInput: (data) => this.sockets?.input(data),
        setStdin: (enabled) => { if (this.term) this.term.options.disableStdin = !enabled; },
        setStatus: (text, active) => this.chrome.setStatus(text, active),
        showTake: (visible) => this.chrome.showTake(visible),
        showError: (message) => this.chrome.showError(message),
        clearError: () => this.chrome.clearError(),
        focus: () => this.term?.focus(),
        fit: () => this.fitNow(),
      },
      autoTake,
    );
    this.input = new TerminalInputBinding(this.control);
  }

  open(): void {
    if (!this.host || !TerminalCtor || !FitCtor) {
      this.fail("Terminal emulator is unavailable");
      return;
    }
    const term = new TerminalCtor({ ...TERMINAL_OPTIONS, cols: 80, rows: 24 }) as Xterm;
    const fit = new FitCtor();
    term.loadAddon(fit);
    term.open(this.host);
    this.term = term;
    this.fit = fit;
    this.input.attach(term);
    term.onResize(({ cols, rows }) => {
      if (!this.control.controlled || isSeeding(this.stream)) return;
      this.chrome.setSize(`${cols}×${rows}`);
      this.sockets?.resize(cols, rows);
    });
    this.host.onclick = () => {
      term.focus();
      this.control.requestControl();
    };
    this.chrome.onTake(() => this.control.requestControl());
    this.observer = new ResizeObserver(() => {
      if (this.control.controlled && !isSeeding(this.stream)) this.fitNow();
    });
    this.observer.observe(this.host);
    this.openSockets();
  }

  takeControl(): void { this.term?.focus(); this.control.requestControl(); }

  /** Follow the owner's PTY size while this view is read-only. */
  reconcileSize(cols?: number, rows?: number): void {
    const term = this.term;
    if (this.disposed || isSeeding(this.stream) || this.control.controlled || !term) return;
    if (!cols || !rows || (term.cols === cols && term.rows === rows)) return;
    term.resize(cols, rows);
    this.chrome.setSize(`${cols}×${rows}`);
  }

  dispose(): void {
    this.disposed = true;
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
    this.reconnectTimer = null;
    this.observer?.disconnect();
    this.observer = null;
    this.sockets?.close();
    this.sockets = null;
    this.control.dispose();
    this.chrome.onTake(() => undefined);
    this.term?.dispose();
    this.term = null;
    if (this.host) this.host.onclick = null;
    this.host?.replaceChildren();
  }

  private openSockets(): void {
    const token = loadToken();
    if (!token) {
      this.fail("Terminal access token is unavailable");
      return;
    }
    this.stream = begin(this.stream);
    this.input.reset();
    this.control.resetConnection();
    this.sockets = new TerminalSockets(this.target, token, {
      outputText: (message) => this.onOutput(message),
      outputBytes: (data) => this.term?.write(data),
      controlText: (message) => this.control.control(message),
      closed: () => this.onClosed(),
    });
  }

  private onOutput(message: TerminalOutputFrame): void {
    if (message?.type === "ready") {
      this.stream = seed(this.stream);
      this.term?.reset();
      if (typeof message.cols === "number" && typeof message.rows === "number") {
        this.term?.resize(message.cols, message.rows);
        this.chrome.setSize(`${message.cols}×${message.rows}`);
      }
      this.control.setStream(false, true);
    } else if (message?.type === "live") {
      const generation = this.stream.generation;
      this.term?.write("", () => this.live(generation));
    } else if (message?.type === "reset") {
      this.stream = seed(this.stream);
      this.term?.reset();
      this.control.setStream(false, true);
    } else if (message?.type === "exit") {
      this.stream = finish(this.stream, "ended");
      this.control.setStream(false, false);
      this.chrome.setStatus("Session exited");
    } else if (message?.type === "unreachable") {
      this.stream = finish(this.stream, "ended");
      this.control.setStream(false, false);
      this.chrome.setStatus("Terminal is unreachable");
    } else if (message?.type === "error") {
      this.fail(message.message || "Terminal stream failed");
    }
  }

  private live(generation: number): void {
    if (this.disposed) return;
    const result = settleLive(this.stream, generation);
    if (!result.applied) return;
    this.stream = result.state;
    this.control.setStream(true, false);
    this.chrome.clearError();
    this.reconnectAttempts = 0;
  }

  private onClosed(): void {
    if (this.disposed) return;
    this.stream = begin(this.stream);
    this.control.resetConnection();
    this.chrome.setStatus("Reconnecting…");
    this.sockets?.close();
    if (this.reconnectTimer) return;
    const delay = reconnectDelay(this.reconnectAttempts++);
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      if (this.disposed) return;
      this.openSockets();
    }, delay);
  }

  private fitNow(): void {
    if (this.disposed || !this.fit) return;
    try {
      this.fit.fit();
    } catch {
      // The pane may be hidden while the view is being disposed.
    }
  }

  private fail(message: string): void {
    this.stream = finish(this.stream, "failed");
    this.control.setStream(false, false);
    this.chrome.showError(message);
    this.chrome.setStatus(message);
  }
}
