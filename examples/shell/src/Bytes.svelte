<!--
  Binary standard output. ffmpeg writes one raw RGBA test frame; `run` returns it as bytes and the
  canvas paints it pixel by pixel, while `spawn` counts the chunks of a second run of the same.
-->
<script lang="ts">
  import { processes } from "@neoworks/svelte-gpui";
  import { Canvas, Text, View, Window } from "@neoworks/svelte-gpui/ui";

  let { monitor }: { monitor: string } = $props();

  const SIDE = 32;
  const SCALE = 8;
  const frame = ["ffmpeg", "-v", "error", "-f", "lavfi", "-i", `testsrc=size=${SIDE}x${SIDE}`, "-frames:v", "1",
    "-f", "rawvideo", "-pix_fmt", "rgba", "-"];

  let canvas: Canvas;
  let summary = $state("running");
  let streamed = $state("running");

  function paint(bytes: Uint8Array) {
    for (let index = 0; index < SIDE * SIDE; index += 1) {
      const [red, green, blue] = bytes.subarray(index * 4, index * 4 + 3);
      const color = `rgb(${red}, ${green}, ${blue})`;
      canvas.fillRect((index % SIDE) * SCALE, Math.floor(index / SIDE) * SCALE, SCALE, SCALE, color);
    }
  }

  $effect(() => {
    processes.run(frame, { encoding: "bytes" }).then((result) => {
      summary = `run: ${result.stdout.length} bytes, status ${result.status}, first ${[...result.stdout.subarray(0, 4)].join(" ")}`;
      paint(result.stdout);
    });
    let total = 0;
    let chunks = 0;
    processes.spawn(frame, {
      onStdoutBytes: (chunk) => { total += chunk.length; chunks += 1; },
      onExit: () => (streamed = `spawn: ${total} bytes in ${chunks} chunks`),
    });
  });
</script>

<Window {monitor} layer="top" anchor={["top", "bottom", "left", "right"]} namespace="shell-bytes"
        style={{ background: "#101820" }}>
  <View style={{ padding: 20, gap: 12 }}>
    <Text style={{ color: "white", fontSize: 18 }}>{summary}</Text>
    <Text style={{ color: "white", fontSize: 18 }}>{streamed}</Text>
    <Canvas bind:this={canvas} style={{ width: SIDE * SCALE, height: SIDE * SCALE }} />
  </View>
</Window>
