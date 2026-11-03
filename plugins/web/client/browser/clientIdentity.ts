function randomId(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  return [...bytes].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

/**
 * Unique per page load and shared by the session and terminal sockets.
 * Deliberately not sessionStorage: browsers can clone sessionStorage into a
 * duplicated tab, which must not alias terminal control ownership.
 */
const ID = randomId();

export function clientId(): string {
  return ID;
}
