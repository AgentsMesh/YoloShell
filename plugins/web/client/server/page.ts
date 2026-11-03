export const page = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<meta name="theme-color" content="#0d0b09">
<title>Yolo Web</title>
<link rel="icon" href="/icon.svg" type="image/svg+xml">
<link rel="manifest" href="/manifest.webmanifest">
<link rel="stylesheet" href="/app.css">
<link rel="stylesheet" href="/vendor/xterm.css">
</head>
<body>
<div id="gate" class="gate">
<form id="gate-form" class="gate-card">
<div class="gate-mark" aria-hidden="true">Y</div>
<p class="eyebrow">Trusted plugin · local Hub</p>
<h1>Yolo <span>web</span></h1>
<p class="gate-copy">Interactive terminals and agent conversations.</p>
<label for="token">Access token</label>
<input id="token" type="password" autocomplete="off" spellcheck="false" placeholder="Paste the plugin token">
<button id="gate-submit" type="submit" disabled>Connect</button>
<p id="gate-message" class="note" role="status" aria-live="polite">Read the token from the plugin data directory.</p>
</form>
</div>
<div id="app" class="app" hidden>
<aside id="sidebar" class="sidebar" aria-label="Yolo navigation">
<div class="side-head"><div class="brand"><span class="brand-mark" aria-hidden="true">Y</span><strong>Yolo <span>web</span></strong></div><button id="archive-open" class="icon quiet" type="button">History</button><button id="notify" class="icon quiet" type="button" aria-pressed="false">Notify</button><button id="quiet" class="icon quiet" type="button" aria-pressed="false">Quiet</button><button id="lock" class="icon" type="button">Lock</button></div>
<div class="hub-card"><div class="hub-copy"><strong>Connected</strong><span id="hub-status">Connecting…</span></div><span id="hub-dot" class="state-dot state-unknown" aria-hidden="true"></span></div>
<section id="machines" class="side-section machines" aria-labelledby="machines-heading" hidden><h2 id="machines-heading">Machines</h2><ul id="machine-list" class="rows"></ul></section>
<label class="filter" for="filter"><span class="sr-only">Filter agents and shells</span><input id="filter" type="search" placeholder="Filter agents and shells"></label>
<section id="needs" class="side-section needs" aria-labelledby="needs-heading" hidden><h2 id="needs-heading">Needs you <span id="needs-count" class="count">0</span></h2><ul id="needs-list" class="rows"></ul></section>
<section class="side-section agents" aria-labelledby="agents-heading"><h2 id="agents-heading">Agents <span id="agent-count" class="count">0</span><button id="new-agent" class="section-add" type="button" aria-label="New agent" title="New agent">+</button></h2><ul id="agent-list" class="rows"></ul></section>
<section class="side-section terminals" aria-labelledby="terminals-heading"><h2 id="terminals-heading">Shells <span id="terminal-count" class="count">0</span><button id="new-shell" class="section-add" type="button" aria-label="New shell" title="New shell">+</button></h2><ul id="terminal-list" class="rows"></ul></section>
<p id="updated" class="updated">Not connected</p>
</aside>
<main class="pane">
<header class="pane-head"><button id="menu" class="icon menu" type="button" aria-label="Toggle navigation" aria-expanded="false">Menu</button><div class="pane-copy"><h1 id="pane-title">No selection</h1><p id="pane-subtitle">Choose an agent or shell on the left.</p></div><div id="lens" class="lens" role="group" aria-label="Pane view" hidden><button id="lens-chat" type="button" aria-pressed="false">Chat</button><button id="lens-terminal" type="button" aria-pressed="false">Terminal</button></div></header>
<section id="interaction" class="interaction" aria-label="Pending agent interaction" hidden><p id="interaction-prompt" class="interaction-prompt"></p><div id="interaction-choices" class="interaction-choices" role="group" aria-label="Answer the agent"></div></section>
<section id="conversation" class="conversation" aria-label="Conversation" hidden><button id="older" class="older" type="button" hidden>Load older messages</button><div id="turns" class="turns"></div><p id="conversation-note" class="note" hidden></p></section>
<p id="chat-locked" class="note chat-locked" hidden>Prompting is unavailable for this agent. Switch to Terminal to interact.</p>
<p id="lifecycle-note" class="note lifecycle-note" hidden><span id="lifecycle-note-text"></span><button id="lifecycle-archive-ended" class="terminal-lifecycle" type="button">Archive</button></p>
<section id="terminal-view" class="terminal-view" aria-label="Terminal" hidden><div class="terminal-meta"><span id="terminal-size"></span><span id="terminal-input-state" class="terminal-input-state">Connecting…</span><span class="terminal-actions"><button id="terminal-log" class="terminal-take" type="button">Log</button><button id="terminal-take" class="terminal-take" type="button" hidden>Take control</button><button id="terminal-end" class="terminal-lifecycle" type="button" hidden>End</button><button id="terminal-archive" class="terminal-lifecycle" type="button" hidden>Archive</button></span></div><div id="terminal-host" class="terminal-host"></div><p id="terminal-finished" class="note terminal-finished" hidden>Session finished. Archive it to remove the row; its history stays on the machine.</p><p id="terminal-error" class="note" hidden></p></section>
<section id="empty" class="empty"><div><strong>No selection</strong><p>Pick an agent or shell to begin.</p></div></section>
<form id="composer" class="composer" hidden><label class="sr-only" for="prompt">Message</label><textarea id="prompt" rows="1" placeholder="Message…" autocomplete="off" spellcheck="false"></textarea><button id="send" type="submit">Send</button><p id="send-status" class="send-status" role="status" aria-live="polite"></p><p id="composer-hint" class="composer-hint">Enter sends · Shift+Enter adds a line · Hub acquires the writer lease</p></form>
</div>
<div id="alerts" class="alerts" role="status" aria-live="polite"></div>
<dialog id="create" class="create" aria-labelledby="create-title">
<form id="create-form" class="create-card">
<h2 id="create-title">New shell</h2>
<p class="eyebrow">Start work in a new terminal</p>
<label>Machine<select id="create-machine"></select></label>
<label id="create-provider-row">Agent<select id="create-provider">
<option value="codex">Codex</option>
<option value="claude_code">Claude Code</option>
<option value="qwen_code">Qwen Code</option>
<option value="gemini_cli">Gemini CLI</option>
<option value="grok">Grok Build</option>
</select></label>
<label>Directory<input id="create-cwd" type="text" spellcheck="false" autocomplete="off" placeholder="/tmp"></label>
<label><span>Name <span class="optional">optional</span></span><input id="create-name" type="text" spellcheck="false" autocomplete="off" placeholder="e.g. api-work"></label>
<p id="create-message" class="note" role="status" aria-live="polite"></p>
<div class="create-actions"><button id="create-cancel" type="button">Cancel</button><button id="create-submit" type="submit">Create shell</button></div>
</form>
</dialog>
<dialog id="lifecycle" class="create" aria-labelledby="lifecycle-title">
<form id="lifecycle-form" class="create-card">
<h2 id="lifecycle-title">End session?</h2>
<p id="lifecycle-copy" class="note"></p>
<p id="lifecycle-message" class="note" role="status" aria-live="polite"></p>
<div class="create-actions"><button id="lifecycle-cancel" type="button">Cancel</button><button id="lifecycle-confirm" type="submit">End session</button></div>
</form>
</dialog>
<dialog id="archive" class="create archive" aria-labelledby="archive-title">
<div class="create-card archive-card">
<h2 id="archive-title">Session history</h2>
<p id="archive-note" class="note" role="status" aria-live="polite">Loading…</p>
<ul id="archive-list" class="rows archive-list"></ul>
<div id="archive-history" class="archive-history" hidden><div id="archive-terminal" class="terminal-host"></div><p id="archive-history-note" class="note"></p></div>
<div class="create-actions"><button id="archive-finish" type="button" hidden>Archive</button><button id="archive-back" type="button" hidden>Back</button><button id="archive-close" type="button">Close</button></div>
</div>
</dialog>
<dialog id="terminal-log-dialog" class="create" aria-labelledby="terminal-log-title">
<div class="create-card archive-card">
<h2 id="terminal-log-title">Terminal log</h2>
<p id="terminal-log-note" class="note" role="status" aria-live="polite"></p>
<pre id="terminal-log-text" class="terminal-log"></pre>
<div class="create-actions"><button id="terminal-log-refresh" type="button">Refresh</button><button id="terminal-log-close" type="button">Close</button></div>
</div>
</dialog>
<noscript>Yolo Web needs JavaScript.</noscript>
<script src="/vendor/xterm.js"></script>
<script src="/vendor/addon-fit.js"></script>
<script type="module" src="/app/main.js"></script>
</body>
</html>`;
