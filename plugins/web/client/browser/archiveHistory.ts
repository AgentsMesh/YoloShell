import { byId } from "./dom.js";
import { TERMINAL_OPTIONS } from "./terminalTheme.js";
import { FitCtor, TerminalCtor, type Xterm } from "./terminalXterm.js";

let term: Xterm | null = null;
let observer: ResizeObserver | null = null;

export function archiveHistoryNote(text: string): void {
  byId("archive-history-note").textContent = text;
}

export function disposeArchiveTerminal(): void {
  observer?.disconnect();
  observer = null;
  term?.dispose();
  term = null;
  byId("archive-terminal").replaceChildren();
}

export function showArchiveTerminal(visible: boolean): void {
  byId("archive-terminal").hidden = !visible;
}

/** Replay one raw terminal stream into a read-only emulator. */
export function showArchiveStream(data: Uint8Array): boolean {
  const host = byId("archive-terminal");
  if (!TerminalCtor || !FitCtor) return false;
  disposeArchiveTerminal();
  showArchiveTerminal(true);
  const terminal = new TerminalCtor({
    ...TERMINAL_OPTIONS,
    cols: 100,
    rows: 30,
    disableStdin: true,
  }) as Xterm;
  const fit = new FitCtor();
  terminal.loadAddon(fit);
  terminal.open(host);
  fit.fit();
  terminal.write("\x1bc");
  terminal.write(data);
  term = terminal;
  observer = new ResizeObserver(() => fit.fit());
  observer.observe(host);
  return true;
}
