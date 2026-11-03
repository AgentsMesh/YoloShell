import { forgetToken } from "./api.js";
import {
  applyInventory,
  applyPatch,
  connected,
  disconnected,
} from "./catalogueController.js";
import { frame } from "./conversationController.js";
import { showGate } from "./render.js";
import { SessionClient } from "./session.js";
import { setSession } from "./sessionLink.js";

let session: SessionClient | null = null;

function unauthorized(): void {
  forgetToken();
  showGate("Token rejected.", "bad");
}

export function startSession(): void {
  session?.close();
  session = new SessionClient({
    catalogue: applyInventory,
    cataloguePatch: applyPatch,
    connected,
    disconnected,
    unauthorized,
    conversation: frame,
  });
  setSession(session);
  session.start();
}

export function refresh(): Promise<void> {
  session?.refresh();
  return Promise.resolve();
}
