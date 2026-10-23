import { start } from "@neoworks/svelte-gpui";
import App from "./App.svelte";

start(App, {}, { window: { title: "native io", width: 900, height: 560 } });
