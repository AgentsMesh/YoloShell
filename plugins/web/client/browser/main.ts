import { hasToken, loadToken } from "./api.js";
import { bind } from "./events.js";
import { showApp, showGate } from "./render.js";
import { refresh, startSession } from "./sessionController.js";

let booting = false;

async function boot(): Promise<void> {
  if (booting) return;
  booting = true;
  try {
    showApp();
    startSession();
  } finally {
    booting = false;
  }
}

bind({ start: boot, refresh });
loadToken();
if (hasToken())
  boot().catch((error) => {
    console.error("web boot failed", error);
    showGate(error instanceof Error ? error.message : "Could not connect.", "bad");
  });
else showGate();
