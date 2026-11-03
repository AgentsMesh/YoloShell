import { ApiError, forgetToken } from "./api.js";
import { renderStatus } from "./render.js";
import { dispatch } from "./store.js";

export function setBusy(busy: boolean): void {
  dispatch({ type: "request/busy", busy });
}

export function setStatus(text: string, kind = ""): void {
  dispatch({ type: "request/status", text, kind });
}

export function clearStatus(): void {
  setStatus("");
}

export function lock(): void {
  forgetToken();
  location.reload();
}

export function fail(error: unknown): void {
  if (error instanceof ApiError && error.status === 401) {
    lock();
    return;
  }
  setStatus(error instanceof Error ? error.message : "Request failed", "bad");
  renderStatus();
}
