pub(crate) const HELP_KEY: char = '?';
pub(crate) const NEW_SHELL_KEY: char = 'c';
pub(crate) const ACQUIRE_KEY: char = 'a';
pub(crate) const TAKEOVER_KEY: char = 't';
pub(crate) const DETACH_KEY: char = 'd';
pub(crate) const QUIT_KEY: char = 'q';
pub(crate) const SESSIONS_KEY: char = 's';
pub(crate) const WORKSPACE_KEY: char = 'W';
pub(crate) const HELP_TITLE: &str = " Yolo · tabs and terminals ";
pub(crate) const HELP_SCROLL_STEP: u16 = 1;
pub(crate) const HELP_FOOTER_ROWS: u16 = 1;
pub(crate) const HELP_FOOTER: &str =
    "Esc/q close · ↑↓/PgUp/PgDn scroll · Home/End · Ctrl-b d detach";
pub(crate) const HELP_COMPACT_FOOTER: &str = "Esc/q close · ↑↓ scroll";
pub(crate) const HELP_CLOSE_HINT: &str = "Esc/q close";
pub(crate) const HELP_TEXT: &str = concat!(
    "Type normally in a pane to use its shell or agent.\n",
    "Start claude, codex, or any other program from the shell.\n\n",
    "Ctrl-b c        New tab with a shell in the current directory and Hub\n",
    "Ctrl-b v / %    Split right with a new shell\n",
    "Ctrl-b - / \"    Split down with a new shell\n",
    "Ctrl-b n / p    Next / previous tab\n",
    "Ctrl-b o/arrows Focus another pane\n",
    "Ctrl-b H/J/K/L  Resize the focused split; Ctrl-b = equalizes panes\n",
    "Ctrl-b z        Zoom the focused pane; Ctrl-b { / } swaps panes\n",
    "Ctrl-b s        Focus the machine/agent tree; Enter opens the selection\n",
    "Terminal mode: arrows and Tab go to the PTY; use Ctrl-b s before sidebar navigation\n",
    "Ctrl-b b        Show / hide sidebar\n",
    "Ctrl-b m        Toggle agent Chat; Enter appends into its terminal input and submits\n",
    "Chat: Esc terminal · Alt-Enter newline · PgUp history · Ctrl-x discard local draft\n",
    "Ctrl-b g        Open existing-terminal picker\n",
    "Ctrl-b w        Manage tabs\n",
    "Ctrl-b x / X    Close view and discard its Chat draft; shells keep running\n",
    "Ctrl-b R        View focused terminal's recovery history\n",
    "Ctrl-b a/t/r    Acquire / explicitly take over / release input\n",
    "Ctrl-b Ctrl-b   Send Ctrl-b to the shell or agent\n",
    "Ctrl-b d / q    Detach Yolo; shells and agents keep running\n",
    "Ctrl-b ? / F1   Show help; Esc, q or ? closes it\n",
    "Ctrl-b W        Workspace filter; c creates, e renames, l edits labels\n\n",
    "Sidebar: c new tab · n directory · / filter · I details · i interrupt · 1-9 answer · h history\n",
    "Tree: Enter opens a machine workspace; Left/Right or Space collapses/expands; m manages machines.\n",
    "Tab/Agent badges: ✎ writable · ◇ read-only · ‖ paused · … syncing; spinner = working.\n",
    "Machine rows: ● reachable · ○ unreachable · ? stale or unknown.\n",
    "Wheel: local history; full-screen apps page; Codex transcript scrolls 3 lines (Esc returns).\n",
    "Agent exit returns to its shell. Shell exit ends its PTY; Ctrl-b c opens a new tab.\n",
    "Ordinary q and ? remain shell input."
);
