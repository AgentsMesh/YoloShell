import type { PendingAgent } from "./pendingAgents.js";
import type {
  Entry,
  Inventory,
  Json,
  Lens,
  SelectionIntent,
  Snapshot,
} from "./types.js";

export type Intent =
  | { type: "catalogue/observe"; emptyStreak: number; at: number }
  | {
      type: "catalogue/replace";
      inventory: Inventory;
      snapshot: Snapshot | undefined;
      emptyStreak: number;
      at: number;
    }
  | { type: "catalogue/patch"; inventory: Inventory; at: number }
  | { type: "catalogue/connected" }
  | { type: "catalogue/disconnected" }
  | { type: "catalogue/pendingAgents"; pendingAgents: PendingAgent[] }
  | { type: "ui/scope"; scope: string | null }
  | { type: "ui/filter"; filter: string }
  | { type: "selection/choose"; intent: SelectionIntent }
  | { type: "selection/created"; intent: SelectionIntent }
  | { type: "selection/adopt"; intent: SelectionIntent }
  | { type: "selection/lens"; lens: Lens }
  | { type: "selection/interactionCleared" }
  | {
      type: "conversation/latest";
      entries: Entry[];
      nextCursor: Json | null;
      partial: string[];
    }
  | { type: "conversation/unchanged" }
  | { type: "conversation/unavailable"; reason: string }
  | { type: "conversation/error"; message: string }
  | { type: "conversation/olderStart" }
  | {
      type: "conversation/olderAppend";
      entries: Entry[];
      nextCursor: Json | null;
      partial: string[];
    }
  | { type: "conversation/olderEnd" }
  | { type: "request/busy"; busy: boolean }
  | { type: "request/status"; text: string; kind?: string }
  | { type: "attention/notify"; value: boolean }
  | { type: "attention/quiet"; value: boolean };
