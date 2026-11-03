import { api } from "./api.js";
import { answerResult } from "./answer.js";
import { clientId } from "./clientIdentity.js";
import { loadOlder } from "./conversationController.js";
import { byId } from "./dom.js";
import { selectionPayload } from "./payload.js";
import { submitPrompt } from "./promptSend.js";
import { renderPane, renderStatus } from "./render.js";
import { fail, setBusy, setStatus } from "./requestController.js";
import { selected } from "./appSelectors.js";
import { dispatch, getState } from "./store.js";

export { closeDrawer } from "./drawer.js";

async function respond(interactionId: string, choice: string): Promise<void> {
  const state = getState();
  const target = selected(state);
  if (!target || target.kind !== "agent" || state.request.busy) return;
  setBusy(true);
  setStatus("Answering…");
  renderPane();
  try {
    const result = (await api.respond({
      ...selectionPayload(target),
      interaction_id: interactionId,
      choice,
      client_id: clientId(),
    })) as { receipt?: { outcome?: unknown } };
    const answer = answerResult(result.receipt?.outcome);
    if (answer.clear) dispatch({ type: "selection/interactionCleared" });
    setStatus(answer.text, answer.ok ? "ok" : "bad");
  } catch (error) {
    fail(error);
  } finally {
    setBusy(false);
    renderPane();
    renderStatus();
  }
}

export function bindActions(): void {
  byId<HTMLFormElement>("composer").addEventListener("submit", (event) => {
    event.preventDefault();
    void submitPrompt();
  });
  const field = byId<HTMLTextAreaElement>("prompt");
  field.addEventListener("keydown", (event) => {
    if (event.key === "Enter" && !event.shiftKey && !event.isComposing) {
      event.preventDefault();
      void submitPrompt();
    }
  });
  byId<HTMLButtonElement>("older").addEventListener("click", () => void loadOlder());
  byId("interaction-choices").addEventListener("click", (event) => {
    const button = (event.target as HTMLElement).closest<HTMLButtonElement>(
      "button[data-choice]",
    );
    if (!button) return;
    void respond(button.dataset.interaction ?? "", button.dataset.choice ?? "");
  });
}
