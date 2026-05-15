// Hello World — the simplest possible LCD plugin.
//
// Copy this folder into ~/.local/share/linux-claude-desktop/plugins/,
// open Settings → Plugins, click "Reload Plugins", and you should see this
// plugin listed as "Active" with a console line below.

export function activate(lcd) {
  lcd.log("Hello from the hello-world plugin!");
  lcd.log("Host version:", lcd.version);

  // PR 3 will start emitting these events. Subscribing now is safe —
  // the handler simply never fires until then.
  lcd.on("response:complete", (payload) => {
    lcd.log("Claude finished replying:", payload?.text?.slice?.(0, 60) ?? "(no text)");
  });
}

export function deactivate() {
  // Event subscriptions are auto-removed by the host; nothing to do here.
}
