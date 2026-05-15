// Hello World — demonstrates the PR 1-3 plugin API.
//
// After installing (copy this folder into ~/.local/share/linux-claude-desktop/plugins/
// and reload), try:
//
//   /greet     — inserts a greeting and auto-sends it
//   /count     — increments a counter persisted via lcd.storage
//   /reset     — clears the counter and shows a toast
//   /stats     — shows how many Claude responses you've received in this session
//
// And try ending a message with "!!" — the message:before-send hook will rewrite
// it to be slightly more polite before the API call.
//
// You can also watch the plugin's console panel in Settings → Plugins.

export function activate(lcd) {
  lcd.log("hello-world activated; host version:", lcd.version);

  // ── Slash commands (PR 2) ──────────────────────────────────────────────────

  lcd.registerCommand({
    name: "greet",
    description: "Send a friendly greeting to Claude",
    handler: () => ({ text: "Hello, Claude! How are you today?", send: true }),
  });

  lcd.registerCommand({
    name: "count",
    description: "Bump a persistent counter and report the new value",
    handler: async () => {
      const current = (await lcd.storage.get("counter")) ?? 0;
      const next = current + 1;
      await lcd.storage.set("counter", next);
      lcd.notify(`Counter is now ${next}`, "info");
      lcd.log("count →", next);
    },
  });

  lcd.registerCommand({
    name: "reset",
    description: "Reset the counter to zero",
    handler: async () => {
      await lcd.storage.delete("counter");
      lcd.notify("Counter reset", "success");
    },
  });

  lcd.registerCommand({
    name: "stats",
    description: "Show how many Claude responses arrived this session",
    handler: async () => {
      const n = (await lcd.storage.get("session_responses")) ?? 0;
      lcd.notify(`Claude has replied ${n} time${n === 1 ? "" : "s"} this session`, "info");
    },
  });

  // ── Event hooks (PR 3) ─────────────────────────────────────────────────────

  // Mutable hook: rewrite enthusiastic messages.
  lcd.on("message:before-send", (payload) => {
    if (payload.text.endsWith("!!")) {
      const rewritten = payload.text.replace(/!!$/, ".");
      lcd.log("rewrote outgoing message:", payload.text, "→", rewritten);
      return { text: rewritten };
    }
  });

  // Observable hook: count Claude's replies this session.
  lcd.on("response:complete", async (payload) => {
    const prev = (await lcd.storage.get("session_responses")) ?? 0;
    await lcd.storage.set("session_responses", prev + 1);
    lcd.log("response complete", "len:", payload.text.length, "total:", prev + 1);
  });

  // Observable hook: log artifact creation.
  lcd.on("artifact:create", (payload) => {
    lcd.log("artifact created:", payload.title, `(${payload.language || payload.artifactType})`);
  });
}

export function deactivate() {
  // Event subscriptions and commands are auto-removed by the host.
}
