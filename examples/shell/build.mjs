import { fileURLToPath } from "node:url";
import { build } from "../../build.mjs";
const here = (path) => fileURLToPath(new URL(path, import.meta.url));
// OUTPUT is the monitor the scenes draw on, SCENE the one to build (see src/App.svelte).
const output = process.env.OUTPUT ?? "svelte-gpui";
const scene = process.env.SCENE ?? "layers";
await build({
  entry: here("./src/main.ts"),
  outfile: here(`./dist/shell-${scene}.js`),
  define: {
    __OUTPUT__: JSON.stringify(output),
    __SCENE__: JSON.stringify(scene),
    __VIDEO__: JSON.stringify(process.env.VIDEO ?? "/tmp/shell-host-shots/test.mp4"),
  },
});
