// Hello World — demonstrates the PR 2 plugin API.
//
// After installing (copy this folder into ~/.local/share/linux-claude-desktop/plugins/
// and reload), try these commands in chat:
//
//   /greet     — inserts a greeting and auto-sends it
//   /count     — increments a counter persisted via lcd.storage
//   /reset     — clears the counter and shows a toast
//
// You can also watch the plugin's console panel in Settings → Plugins.

export function activate(lcd) {
  lcd.log("hello-world activated; host version:", lcd.version);

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
      return undefined; // side-effect only
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

  // PR 3 will start emitting these events into the bus; subscribing now is safe.
  lcd.on("response:complete", (payload) => {
    lcd.log("Claude replied:", payload?.text?.slice?.(0, 60) ?? "(no text)");
  });
}

export function deactivate() {
  // Event subscriptions and commands are auto-removed by the host.
}
