import { mount } from "svelte";
import App from "./App.svelte";

if ("serviceWorker" in navigator) {
  const base = import.meta.env.BASE_URL;
  navigator.serviceWorker
    .register(base + "sw.js", { scope: base })
    .catch(() => {});
}

mount(App, { target: document.getElementById("app") });
