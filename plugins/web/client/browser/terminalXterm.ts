export type Xterm = {
  cols: number;
  rows: number;
  open(host: HTMLElement): void;
  dispose(): void;
  focus(): void;
  reset(): void;
  resize(cols: number, rows: number): void;
  write(data: string | Uint8Array, callback?: () => void): void;
  loadAddon(addon: unknown): void;
  onData(callback: (data: string) => void): void;
  onBinary(callback: (data: string) => void): void;
  onResize(callback: (size: { cols: number; rows: number }) => void): void;
  options: { disableStdin?: boolean };
};

export const TerminalCtor = (globalThis as any).Terminal;
export const FitCtor = (globalThis as any).FitAddon?.FitAddon;
