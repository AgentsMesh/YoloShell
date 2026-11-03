import type { SessionClient } from "./session.js";

let current: SessionClient | null = null;

export function setSession(value: SessionClient | null): void {
  current = value;
}

export function getSession(): SessionClient | null {
  return current;
}
