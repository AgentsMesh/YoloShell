export type TerminalControlHost = {
  sendTake(): void;
  sendRelease(): void;
  sendInput(data: Uint8Array): void;
  setStdin(enabled: boolean): void;
  setStatus(text: string, active?: boolean): void;
  showTake(visible: boolean): void;
  showError(message: string): void;
  clearError(): void;
  focus(): void;
  fit(): void;
};
