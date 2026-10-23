<!--
  Checks the process and unix socket APIs against the real system, and shows what each
  one produced. Run it with scripts/shot.sh and read the screenshot or the host log.
-->
<script lang="ts">
  import { processes, unixSockets } from "@neoworks/svelte-gpui";
  import { Scroll, Text, View } from "@neoworks/svelte-gpui/ui";

  declare const console: { log(...args: unknown[]): void };

  interface Check {
    name: string;
    state: "running" | "ok" | "FAIL";
    detail: string;
  }

  let checks = $state<Check[]>([]);

  function check(name: string, work: (report: (detail: string) => void) => Promise<string>): void {
    const entry: Check = $state({ name, state: "running", detail: "" });
    checks.push(entry);
    const finish = (state: "ok" | "FAIL", detail: string): void => {
      entry.state = state;
      entry.detail = detail;
      console.log(`${state} ${name}: ${detail}`);
    };
    work((detail) => (entry.detail = detail)).then(
      (detail) => finish("ok", detail),
      (error) => finish("FAIL", String(error?.message ?? error)),
    );
  }

  function expectEqual(actual: unknown, expected: unknown): string {
    const [left, right] = [JSON.stringify(actual), JSON.stringify(expected)];
    if (left !== right) {
      throw new Error(`expected ${right}, got ${left}`);
    }
    return left;
  }

  check("run printf", async () => {
    const result = await processes.run(["printf", "%s-%s", "hello", "world"]);
    return expectEqual(result, { status: 0, stdout: "hello-world", stderr: "" });
  });

  check("run with stdin, stderr and a failing status", async () => {
    const result = await processes.run(["sh", "-c", "cat; echo oops >&2; exit 4"], { stdin: "fed\0" });
    return expectEqual(result, { status: 4, stdout: "fed\0", stderr: "oops\n" });
  });

  check("run a missing program rejects", async () => {
    try {
      await processes.run(["svelte-gpui-no-such-program"]);
    } catch (error: any) {
      return expectEqual(error.code, "NotFound");
    }
    throw new Error("did not reject");
  });

  check("spawn streams three lines", async (report) => {
    const lines: string[] = [];
    const handle = processes.spawn(["sh", "-c", "for i in 1 2 3; do echo $i; sleep 0.2; done"], {
      onStdout: (line) => {
        lines.push(line);
        report(`so far: ${lines.join(",")}`);
      },
    });
    const exit = await handle.exited;
    return expectEqual({ lines, exit }, { lines: ["1", "2", "3"], exit: { status: 0 } });
  });

  check("spawn: write, closeStdin and kill", async () => {
    const echoed: string[] = [];
    const cat = processes.spawn(["cat"], { onStdout: (line) => echoed.push(line) });
    cat.write("one\ntwo\n");
    cat.closeStdin();
    const catExit = await cat.exited;
    const sleeper = processes.spawn(["sleep", "30"]);
    sleeper.kill();
    const sleeperExit = await sleeper.exited;
    return expectEqual({ echoed, catExit, sleeperExit }, {
      echoed: ["one", "two"],
      catExit: { status: 0 },
      sleeperExit: { status: 143, signal: "SIGTERM" },
    });
  });

  check("unix socket: request round trip against serve", async () => {
    const paths = await processes.env();
    const path = `${paths.XDG_RUNTIME_DIR ?? "/tmp"}/svelte-gpui-native-io.sock`;
    const server = unixSockets.serve(path, async (text) => `echo:${text.toUpperCase()}`);
    await server.ready;
    try {
      const answer = await unixSockets.request(path, "ping");
      return expectEqual(answer, "echo:PING");
    } finally {
      server.close();
    }
  });

  check("unix socket: line framing and connect", async () => {
    const env = await processes.env();
    const path = `${env.XDG_RUNTIME_DIR ?? "/tmp"}/svelte-gpui-native-io-lines.sock`;
    const server = unixSockets.serve(path, (line) => `got ${line}`, { framing: "line" });
    await server.ready;
    const received: string[] = [];
    const connection = unixSockets.connect(path, {
      onOpen: () => connection.write("a\nb\n"),
      onLine: (line) => {
        received.push(line);
        if (received.length === 2) {
          connection.close();
        }
      },
    });
    await connection.closed;
    server.close();
    return expectEqual(received, ["got a", "got b"]);
  });

  // Still running when the host quits: `pgrep -f "sleep 7777"` must find nothing afterwards.
  processes.spawn(["sleep", "7777"]);

  check("hyprland: j/monitors", async () => {
    const env = await processes.env();
    const path = `${env.XDG_RUNTIME_DIR}/hypr/${env.HYPRLAND_INSTANCE_SIGNATURE}/.socket.sock`;
    const monitors = JSON.parse(await unixSockets.request(path, "j/monitors"));
    return `${monitors.length} monitors: ${monitors.map((monitor: any) => monitor.name).join(", ")}`;
  });

  check("hyprland: .socket2.sock events", async () => {
    const env = await processes.env();
    const path = `${env.XDG_RUNTIME_DIR}/hypr/${env.HYPRLAND_INSTANCE_SIGNATURE}/.socket2.sock`;
    const connection = unixSockets.connect(path);
    await new Promise<void>((resolve) => setTimeout(resolve, 300));
    connection.close();
    await connection.closed;
    return "connected and closed";
  });

</script>

<Scroll style={{ padding: 16, background: "#18181b" }}>
  <View style={{ gap: 8 }}>
    <Text style={{ fontSize: 20, fontWeight: "bold", color: "#fafafa" }}>native io</Text>
    {#each checks as entry}
      <View style={{ direction: "row", gap: 12 }}>
        <Text style={{ width: 60, color: entry.state === "ok" ? "#4ade80" : entry.state === "FAIL" ? "#f87171" : "#facc15" }}>
          {entry.state}
        </Text>
        <View style={{ grow: 1, shrink: 1, minWidth: 0 }}>
          <Text style={{ color: "#fafafa" }}>{entry.name}</Text>
          <Text style={{ color: "#a1a1aa", fontSize: 12 }}>{entry.detail}</Text>
        </View>
      </View>
    {/each}
  </View>
</Scroll>
