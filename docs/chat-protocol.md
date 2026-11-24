# Managed Chat v1

A client can attach a Chat view to an already running agent through the Hub's
JSON-RPC method `chat_v1`. Claude Code, Codex, Gemini CLI, Qwen Code and Grok
Build feed the same history model. Provider hooks supply the live execution,
activity, control capabilities and pending interaction. The native transcript
supplies the history. A single Chat view needs both sources.

## Opening a session

After the regular Hub `hello`, choose an observed agent and its exact runtime
from the catalogue. Send `chat_v1` with a destination, an operation and its
arguments. For example, `binding` below is the pair of targets from that
catalogue row.

```typescript
const { address, sessionId } = await rpc("chat_v1", {
  destination, operation: "open", binding,
});
const request = {
  address, binding, cursor: null, limits: { entries: 64, bytes: 262144 },
  since_revision: null, since_generation: null,
};
const { reading } = await rpc("chat_v1", {
  destination, operation: "read", request,
});
```

The entry Hub advertises `chat_v1` in `extensions.methods` after hello.
All successful Chat results carry `version: 1`. `sessionId` is opaque and identifies
the conversation on its origin, including across resumed executions. A new
execution requires a fresh binding. The stable session ID grants no authority.
The entry Hub translates each operation before applying existing routing,
authorization, request IDs and origin checks. An upgraded entry can project an
older origin's supported conversation methods. An older entry returns JSON-RPC
`-32601`. The legacy conversation and agent methods continue to work.

| Operation | Arguments | Result |
| --- | --- | --- |
| `open` | `binding` with `agent` and `runtime` | `sessionId`, `address`, `binding` |
| `read` | `request` as above, optional `wait_ms` for a conditional watch | `reading` with a replacement page or unchanged checkpoint |
| `state` | Exact execution `target` | Current `agent` snapshot with capabilities, lifecycle, activity and any pending interaction |
| `prompt` | `request` using an execution reference and a durable request ID | `prompt` with delivery receipt and observed outcome |
| `respond` | `request` with the current observation cursor and the pending interaction reference, plus the writer `grant` | `receipt` |
| `cancel` | `request` with the current observation cursor and `action: {operation: "interrupt"}`, plus the writer `grant` | `receipt` for the interruption request |

## Rendering updates

For a changed reading, visit the page's chronological `entries`. Each entry
has its original stable `id`, `role`, optional `timestamp` and ordered
`updates`. The updates use ACP v1 field names. A client using ACP rendering
can forward each update as the `params` of a `session/update` notification.

```typescript
if (reading.status === "page") {
  for (const entry of reading.page.entries) {
    for (const update of entry.updates) {
      renderUpdate({ sessionId: reading.page.sessionId, update });
    }
  }
}
```

| Native evidence | ACP session update |
| --- | --- |
| User or assistant text | `user_message_chunk` or `agent_message_chunk`, with a `text` content block |
| Recorded reasoning | `agent_thought_chunk` |
| Tool invocation | `tool_call` with `toolCallId`, `title` and recorded arguments in `rawInput` |
| Tool result | `tool_call_update` for the same ID, `completed` or `failed`, with text output |
| Context compaction | `compaction_update` with an entry-scoped ID and summary |
| Runtime notices, unavailable images, unsupported records | `notice` with severity, title and optional description |

Use the latest page as a replacement window. Replaying the page twice must
render each entry once. A tool result can refer to an earlier page. Pass
`next_cursor` verbatim to read older history. Send the latest
`history_generation` and `revision` together on the next conditional read.
An unchanged reply preserves that checkpoint. On reconnect, resume from the
same binding and checkpoint. On `history_changed`, clear the old pages and
cursors and read the current page. On `binding_changed`, resolve the observed
execution again. `availability` and `partial` describe transcript coverage.

## Input and interactions

`state.agent.capabilities` independently reports `submit_prompt`,
`respond_interaction` and `interrupt`. Offer each action when its support is
`available`. For approvals, choices and text questions, display the exact
prompt and choices in `agent.interaction`. Carry the observed interaction ID,
cursor and choice ID or text answer into `respond`. Agent control and terminal
write rights, and the writer lease where required, are enforced at the origin.

A prompt may contain a string or an ordered array of ACP `text` and
`resource_link` blocks. Upload local bytes with `stage_attachment` to the
selected runtime first. Convert its returned absolute path into a percent-encoded
`file:///` URI on that machine, then send the resource link in the same prompt.

```json
[
  {"type":"text","text":"Review the attached screenshot."},
  {"type":"resource_link","name":"Screen 1.png","uri":"file:///path/on/origin/Screen%201.png","mimeType":"image/png"}
]
```

The running agent receives each resource as a labeled Markdown link to the
origin file URI alongside the text blocks. Labels and URI delimiters are
escaped. The provider can read the referenced file through its native tools.
This format also lets interactive providers submit the prompt in one action. Source
images whose bytes were never published remain explicit notices. A delivered
prompt, permission response or interrupt is confirmed only to the degree
reported by its receipt. Retain the request ID across uncertain delivery.

This profile uses the running terminal as its process owner. The supported ACP
surface is the session update and content subset above. Yolo supplies
session binding, snapshots, playback windows, permission references and
receipts through its Hub. The ACP v1 wire schema used for mapping is pinned
here. [ACP v1 schema](https://github.com/agentclientprotocol/agent-client-protocol/blob/01a54c84f39508f73fab4d41362baf51e1320374/schema/v1/schema.json)
