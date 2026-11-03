import type { TerminalControl } from "./terminalControl.js";
import type { Xterm } from "./terminalXterm.js";

const ESCAPE = 0x1b;
const PASTE_START = [0x1b, 0x5b, 0x32, 0x30, 0x30, 0x7e];
const PASTE_END = [0x1b, 0x5b, 0x32, 0x30, 0x31, 0x7e];

/**
 * xterm answers terminal queries through onData as well. Query replies are
 * forwarded only while the view holds the lease. Printable input while
 * observing is user intent; an observing paste is dropped whole because its
 * bracketed markers cannot be safely split or truncated.
 */
export class TerminalInputBinding {
  private pasteOpen = false;

  constructor(private readonly control: TerminalControl) {}

  attach(term: Xterm): void {
    term.onData((data) => this.send(new TextEncoder().encode(data)));
    term.onBinary((data) =>
      this.send(Uint8Array.from([...data].map((character) => character.charCodeAt(0)))),
    );
  }

  /** Forgets an unfinished observing paste across a reconnect. */
  reset(): void {
    this.pasteOpen = false;
  }

  private send(data: Uint8Array): void {
    if (this.control.controlled) {
      this.pasteOpen = false;
      this.control.input(data);
      return;
    }
    if (this.pasteOpen) {
      if (indexOf(data, PASTE_END) >= 0) this.pasteOpen = false;
      return;
    }
    const start = indexOf(data, PASTE_START);
    if (start >= 0) {
      if (indexOf(data, PASTE_END, start + PASTE_START.length) < 0) this.pasteOpen = true;
      return;
    }
    if (data.length > 0 && data[0] !== ESCAPE) this.control.input(data);
  }
}

/** xterm emits a whole paste as one chunk containing both markers. */
function indexOf(data: Uint8Array, marker: number[], from = 0): number {
  for (let start = from; start + marker.length <= data.length; start += 1) {
    if (marker.every((byte, offset) => data[start + offset] === byte)) return start;
  }
  return -1;
}
