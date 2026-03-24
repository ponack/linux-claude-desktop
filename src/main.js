import { mount } from "svelte";
import App from "./App.svelte";
import QuickAsk from "./lib/QuickAsk.svelte";
import ChatWindow from "./lib/ChatWindow.svelte";

const params = new URLSearchParams(window.location.search);
const isQuickAsk = params.has("quickask");
const conversationId = params.get("conversation");

let component;
let props = {};

if (isQuickAsk) {
  component = QuickAsk;
} else if (conversationId) {
  component = ChatWindow;
  props = { conversationId };
} else {
  component = App;
}

const app = mount(component, {
  target: document.getElementById("app"),
  props,
});

export default app;
