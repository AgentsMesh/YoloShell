/**
 * Deterministic Anthropic Messages stream used by the conversation acceptance
 * test. It is not a model: the sequence is one Bash approval, a configurable
 * run of Read calls that builds a long transcript, one Bash denial, and a
 * final text turn.
 */
import { existsSync } from "node:fs";

type Options = { reads?: number; dir?: string; allow?: string; deny?: string; hostname?: string; port?: number };

const event = (name: string, data: unknown): string =>
  `event: ${name}\ndata: ${JSON.stringify(data)}\n\n`;

function start(step: number, model: string): string {
  return event("message_start", {
    type: "message_start",
    message: {
      id: `msg_conv_${step}`,
      type: "message",
      role: "assistant",
      model,
      content: [],
      stop_reason: null,
      stop_sequence: null,
      usage: { input_tokens: 1, output_tokens: 0 },
    },
  });
}

function tool(step: number, name: string, input: unknown): string[] {
  return [
    event("content_block_start", {
      type: "content_block_start",
      index: 0,
      content_block: { type: "tool_use", id: `toolu_conv_${step}`, name, input: {} },
    }),
    event("content_block_delta", {
      type: "content_block_delta",
      index: 0,
      delta: { type: "input_json_delta", partial_json: JSON.stringify(input) },
    }),
    event("content_block_stop", { type: "content_block_stop", index: 0 }),
    event("message_delta", {
      type: "message_delta",
      delta: { stop_reason: "tool_use", stop_sequence: null },
      usage: { output_tokens: 8 },
    }),
  ];
}

function text(value: string): string[] {
  return [
    event("content_block_start", {
      type: "content_block_start",
      index: 0,
      content_block: { type: "text", text: "" },
    }),
    event("content_block_delta", {
      type: "content_block_delta",
      index: 0,
      delta: { type: "text_delta", text: value },
    }),
    event("content_block_stop", { type: "content_block_stop", index: 0 }),
    event("message_delta", {
      type: "message_delta",
      delta: { stop_reason: "end_turn", stop_sequence: null },
      usage: { output_tokens: 2 },
    }),
  ];
}

export function startAnthropicMock(options: Options = {}) {
  const reads = options.reads ?? 40;
  const dir = options.dir ?? "/tmp/yolo-web-conversation/files";
  const allow = options.allow ?? "/tmp/yolo-web-conversation/allow";
  const deny = options.deny ?? "/tmp/yolo-web-conversation/deny";
  const state = { steps: 0 };
  const server = Bun.serve({
    hostname: options.hostname ?? "127.0.0.1",
    port: options.port ?? 0,
    async fetch(request) {
      const url = new URL(request.url);
      if (request.method === "GET" && url.pathname === "/state") {
        return Response.json({
          steps: state.steps,
          reads,
          allow: existsSync(allow),
          deny: existsSync(deny),
        });
      }
      if (request.method !== "POST") return Response.json({});
      const body = (await request.json().catch(() => ({}))) as { model?: string };
      state.steps += 1;
      const step = state.steps;
      const parts = [start(step, body.model ?? "claude-test")];
      if (step === 1) {
        parts.push(...tool(step, "Bash", { command: `touch ${allow}`, description: "allow marker" }));
      } else if (step <= reads + 1) {
        parts.push(...tool(step, "Read", { file_path: `${dir}/f${step - 1}.txt` }));
      } else if (step === reads + 2) {
        parts.push(...tool(step, "Bash", { command: `touch ${deny}`, description: "deny marker" }));
      } else {
        parts.push(...text("CONVERSATION_DONE"));
      }
      parts.push(event("message_stop", { type: "message_stop" }));
      return new Response(parts.join(""), {
        headers: { "content-type": "text/event-stream", "cache-control": "no-cache" },
      });
    },
  });
  return {
    port: server.port,
    url: `http://127.0.0.1:${server.port}`,
    state,
    stop: () => server.stop(true),
  };
}
