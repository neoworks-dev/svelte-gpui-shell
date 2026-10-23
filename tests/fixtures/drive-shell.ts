// Runs the app in shell-app.ts the way the host does, and prints the operations it sent after each step as
// JSON, for tests/shell.test.ts. It runs in a process of its own, as drive-element.ts does.

import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { build } from "../../build.mjs";

const globals = globalThis as any;
const operations: unknown[][] = [];
globals.host = {
  apply: (batch: string) => operations.push(...JSON.parse(batch)),
  io: () => {},
  fieldState: () => [],
  frameStats: () => [0, 0],
  clipboardText: () => "",
  windowOptions: () => {},
};

/** The operations sent since the last time this was asked. */
function sent(): unknown[][] {
  return operations.splice(0, operations.length);
}

const outfile = join(mkdtempSync(join(tmpdir(), "svelte-gpui-shell-")), "app.js");
await build({ entry: new URL("./shell-app.ts", import.meta.url).pathname, outfile, sourcemap: false });
const log: Record<string, unknown> = {};
await import(outfile);
log.mounted = sent();

globals.__page.setMarked(false);
globals.__settle();
log.unmarked = sent();

globals.__page.setMarked(true);
globals.__settle();
log.marked = sent();

process.stdout.write(`\n@@${JSON.stringify(log)}\n`);
