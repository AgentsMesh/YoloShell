import { byId, el } from "./dom.js";
import { getState } from "./store.js";
import type { Selected } from "./types.js";

/** Renders the one typed interaction attached to the selected agent. */
export function renderInteraction(selected: Selected | null): void {
  const card = byId("interaction");
  const agent = selected?.kind === "agent" ? selected.agent : null;
  const pending = agent?.interaction;
  const available = agent?.capabilities?.respond_interaction?.support === "available";
  if (!pending?.reference?.id || !available) {
    card.hidden = true;
    return;
  }
  card.hidden = false;
  byId("interaction-prompt").textContent =
    pending.prompt?.trim() || "The agent is waiting for your answer.";
  const choices = byId("interaction-choices");
  choices.replaceChildren(
    ...(pending.choices ?? [])
      .filter((choice) => choice.id)
      .map((choice) => {
        const button = el("button", "interaction-choice", choice.label || choice.id || "");
        button.type = "button";
        button.dataset.choice = choice.id;
        button.dataset.interaction = pending.reference?.id ?? "";
        button.disabled = getState().request.busy;
        return button;
      }),
  );
}
