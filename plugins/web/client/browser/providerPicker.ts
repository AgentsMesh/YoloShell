import {
  parseProviderFeatures,
  providerFeatureView,
  type ProviderFeaturesView,
} from "../shared/protocol.js";

type ProviderRow = {
  provider_id?: string;
  display_name?: string;
  available?: boolean;
  features?: unknown;
};
type Hub = { providers?: ProviderRow[] };

export type ProviderFeatures = ProviderFeaturesView;
export type ProviderChoice = {
  id: string;
  label: string;
  available: boolean;
  features?: ProviderFeatures;
};

function features(value: unknown): ProviderFeatures | undefined {
  const parsed = parseProviderFeatures(value);
  return parsed ? providerFeatureView(parsed) : undefined;
}

function featureTitle(value: ProviderFeatures): string {
  const labels = [];
  if (value.conversation) labels.push("Conversation");
  if (value.submitPrompt) labels.push("Prompt");
  if (value.interrupt) labels.push("Interrupt");
  if (value.interaction === "typed") labels.push("Typed approval");
  else if (value.interaction === "terminal") labels.push("Approval in terminal");
  return labels.length ? labels.join(" · ") : "No agent controls advertised";
}

/** An empty list means an older Hub without provider discovery. */
export function providerChoices(hub: Hub | undefined): ProviderChoice[] {
  const rows = hub?.providers;
  if (!Array.isArray(rows) || rows.length === 0) return [];
  const choices: ProviderChoice[] = [];
  for (const row of rows) {
    if (
      typeof row?.provider_id !== "string" ||
      !/^[A-Za-z0-9._-]{1,96}$/.test(row.provider_id) ||
      typeof row.display_name !== "string" ||
      row.display_name.trim().length === 0 ||
      row.display_name.length > 128 ||
      /[\u0000-\u001f\u007f]/.test(row.display_name) ||
      typeof row.available !== "boolean"
    ) {
      continue;
    }
    const parsed = features(row.features);
    choices.push({
      id: row.provider_id,
      label: row.display_name,
      available: row.available,
      ...(parsed ? { features: parsed } : {}),
    });
  }
  return choices;
}

export function providerSupported(hub: Hub | undefined): boolean {
  const choices = providerChoices(hub);
  return choices.length === 0 || choices.some((choice) => choice.available);
}

export function providerAvailable(hub: Hub | undefined, id: string): boolean {
  const choices = providerChoices(hub);
  return choices.length === 0 || choices.some((choice) => choice.id === id && choice.available);
}

/** Replace the static fallback only when the machine advertises a list. */
export function syncProviderSelect(
  select: HTMLSelectElement,
  hub: Hub | undefined,
): void {
  const choices = providerChoices(hub);
  if (choices.length === 0) return;
  const current = select.value;
  select.replaceChildren(
    ...choices.map((choice) => {
      const option = document.createElement("option");
      option.value = choice.id;
      option.textContent = choice.available
        ? choice.label
        : `${choice.label} · unavailable`;
      if (choice.features) option.title = featureTitle(choice.features);
      option.disabled = !choice.available;
      return option;
    }),
  );
  const preferred =
    choices.find((choice) => choice.id === current && choice.available) ??
    choices.find((choice) => choice.available);
  if (preferred) select.value = preferred.id;
}
