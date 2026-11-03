import { parseProviderFeatures } from "../shared/protocol";
import type { ProviderOption } from "./machineView";

const MAX_PROVIDERS = 64;
const MAX_ID = 96;
const MAX_NAME = 128;

function text(value: unknown, maximum: number): string | null {
  if (
    typeof value !== "string" ||
    value.trim().length === 0 ||
    value.length > maximum ||
    /[\u0000-\u001f\u007f]/.test(value)
  ) {
    return null;
  }
  return value;
}

/** Validate the Hub's provider list; malformed rows are omitted, not guessed. */
export function providerOptions(value: unknown): ProviderOption[] {
  if (!Array.isArray(value)) return [];
  const options: ProviderOption[] = [];
  const seen = new Set<string>();
  for (const row of value.slice(0, MAX_PROVIDERS)) {
    if (typeof row !== "object" || row === null) continue;
    const item = row as Record<string, unknown>;
    const id = typeof item.provider_id === "string" && /^[A-Za-z0-9._-]+$/.test(item.provider_id)
      ? text(item.provider_id, MAX_ID)
      : null;
    const name = text(item.display_name, MAX_NAME);
    if (!id || !name || seen.has(id) || typeof item.available !== "boolean") continue;
    seen.add(id);
    const parsed = parseProviderFeatures(item.features);
    options.push({
      provider_id: id,
      display_name: name,
      available: item.available,
      ...(parsed ? { features: parsed } : {}),
    });
  }
  options.sort((left, right) => left.provider_id.localeCompare(right.provider_id));
  return options;
}
