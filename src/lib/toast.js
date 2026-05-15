// Global toast queue. Mount <Toast /> once in App.svelte; everything else
// just imports `showToast` and calls it.
//
// Used by:
//   - Plugins, via lcd.notify(level, message)
//   - Any feature that wants a transient, non-blocking notice

import { writable } from "svelte/store";

export const toasts = writable([]); // Array<{ id, message, level }>

let nextId = 1;

const LEVELS = new Set(["info", "success", "warning", "error"]);

export function showToast(message, level = "info", durationMs = 3500) {
  const lvl = LEVELS.has(level) ? level : "info";
  const id = nextId++;
  toasts.update((arr) => [...arr, { id, message: String(message), level: lvl }]);
  if (durationMs > 0) {
    setTimeout(() => dismissToast(id), durationMs);
  }
  return id;
}

export function dismissToast(id) {
  toasts.update((arr) => arr.filter((t) => t.id !== id));
}
