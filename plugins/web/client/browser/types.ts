import type { ProviderFeatures } from "../shared/protocol.js";

export type Json = Record<string, any>;
export type TerminalState = {
  status?: string;
  output_sequence?: number;
  size?: { cols?: number; rows?: number };
  target?: { runtime_epoch?: string };
};
export type Terminal = {
  address?: { machine?: string; terminal_id?: string };
  agent?: unknown;
  archived?: boolean;
  cwd?: string;
  name?: string;
  presentation?: { title?: string | null };
  runtime?: TerminalState;
};
export type Agent = {
  cursor?: {
    target?: { address?: { machine?: string; agent_id?: string }; execution_id?: string };
    sequence?: number;
  };
  terminal?: { address?: { machine?: string; terminal_id?: string }; runtime_epoch?: string };
  provider?: { display_name?: string; kind?: string };
  lifecycle?: { state?: string; exit_code?: number | null };
  activity?: string;
  reachability?: string;
  freshness?: string;
  working_directory?: string | null;
  observed_unix_ms?: number;
  schema_version?: number;
  capabilities?: {
    submit_prompt?: { support?: string };
    respond_interaction?: { support?: string };
  };
  interaction?: {
    reference?: { target?: unknown; id?: string };
    kind?: string;
    choices?: { id?: string; label?: string }[];
    prompt?: string;
  } | null;
};
export type Snapshot = { server?: string; sequence?: number };
export type Inventory = {
  machine?: string;
  snapshot?: Snapshot;
  terminals?: { terminals?: Terminal[] };
  agents?: { agents?: Agent[] };
  hubs?: {
    hubs?: {
      machine?: string;
      reachable?: boolean;
      label?: string;
      entry?: boolean;
      freshness?: string;
      epoch?: string;
      revision?: number;
      capabilities?: {
        create?: boolean;
        agent_start?: boolean;
        terminal_lifecycle?: boolean;
      };
      providers?: {
        provider_id?: string;
        display_name?: string;
        available?: boolean;
        features?: ProviderFeatures;
      }[];
    }[];
  };
};
export type Selected = { kind: "agent"; agent: Agent } | { kind: "terminal"; terminal: Terminal };
export type SelectionId = {
  kind: "agent" | "terminal";
  id: string;
  machine?: string;
};
export type SelectionIntent = SelectionId & { take?: boolean };
export type Lens = "chat" | "terminal";
export type Part = {
  kind?: string;
  text?: string;
  summary?: string;
  call_id?: string;
  name?: string;
  arguments?: string;
  output?: string;
  is_error?: boolean;
  native_kind?: string;
  media_type?: string | null;
};
export type Entry = { id?: number[]; role?: string; timestamp?: string | null; parts?: Part[] };
export type Page = {
  entries?: Entry[];
  next_cursor?: unknown;
  history_generation?: number[];
  revision?: number[];
  availability?: string;
  partial?: string[];
};
export type Reading = {
  status?: string;
  page?: Page;
  history_generation?: number[];
  revision?: number[];
};
export type ConversationResult = { available?: boolean; reason?: string; reading?: Reading };
