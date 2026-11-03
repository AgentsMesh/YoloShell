import { byId } from "./dom.js";

export class TerminalChrome {
  private readonly status = byId("terminal-input-state");
  private readonly take = byId<HTMLButtonElement>("terminal-take");
  private readonly size = byId("terminal-size");
  private readonly error = byId("terminal-error");

  onTake(handler: () => void): void {
    this.take.onclick = () => handler();
  }

  setStatus(text: string, active = false): void {
    this.status.textContent = text;
    this.status.classList.toggle("is-active", active);
  }

  setSize(text: string): void {
    this.size.textContent = text;
  }

  showTake(visible: boolean): void {
    this.take.hidden = !visible;
  }

  showError(message: string): void {
    this.error.textContent = message;
    this.error.hidden = false;
  }

  clearError(): void {
    this.error.hidden = true;
    this.error.textContent = "";
  }
}
