const QUIET_KEY = "yolo-quiet";
const NOTIFY_KEY = "yolo-notify";

/** In-page alerts are the default; browser notifications are an extra channel. */
export function loadNotify(): boolean {
  try {
    return localStorage.getItem(NOTIFY_KEY) !== "0";
  } catch {
    return true;
  }
}

export function saveNotify(enabled: boolean): void {
  try {
    localStorage.setItem(NOTIFY_KEY, enabled ? "1" : "0");
  } catch {
    // private mode keeps the in-memory preference for this tab only
  }
}

export function loadQuiet(): boolean {
  try {
    return localStorage.getItem(QUIET_KEY) === "1";
  } catch {
    return false;
  }
}

export function saveQuiet(value: boolean): void {
  try {
    localStorage.setItem(QUIET_KEY, value ? "1" : "0");
  } catch {
    // private mode keeps the in-memory preference for this tab only
  }
}

/** Quiet mode suppresses the title badge but never the Needs-you list. */
export function documentTitle(needs: number, quiet: boolean): string {
  return needs > 0 && !quiet ? `(${needs}) Yolo Web` : "Yolo Web";
}

export function shouldNotify(
  previous: number,
  current: number,
  quiet: boolean,
  enabled: boolean,
): boolean {
  return enabled && !quiet && current > previous;
}

export async function enableNotify(): Promise<boolean> {
  if (typeof Notification === "undefined") return false;
  try {
    if (Notification.permission === "granted") return true;
    if (Notification.permission === "denied") return false;
    return (await Notification.requestPermission()) === "granted";
  } catch {
    return false;
  }
}

export function notifyNeeds(count: number): void {
  if (typeof Notification === "undefined" || Notification.permission !== "granted") return;
  try {
    new Notification("Yolo needs you", {
      body: `${count} agent${count === 1 ? " needs" : "s need"} you`,
      tag: "yolo-needs",
    });
  } catch {
    // Notification construction can fail in restricted environments.
  }
}
