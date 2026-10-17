// The WebExtension namespace: `browser` on Firefox, `chrome` on Chrome, Edge
// and Brave. Both are promise-based for the calls this extension makes.
// Looked up on every call (not once at import) so tests can swap it.

export function ext() {
  return globalThis.browser ?? globalThis.chrome;
}
