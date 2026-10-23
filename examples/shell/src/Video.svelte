<!--
  A looping, muted video as a wallpaper on the `bottom` layer, and a panel on top of it with the
  controls an app would offer: pause, speed, and what the player reported. The panel is the
  only part of its window that takes the pointer.
-->
<script lang="ts">
  import { Button, Text, Video, View, Window } from "@neoworks/svelte-gpui/ui";

  let { monitor, source, looping }: { monitor: string; source: string; looping: boolean } = $props();

  let paused = $state(false);
  let rate = $state(1);
  let fit = $state<"contain" | "cover" | "fill">("cover");
  let duration = $state<number | undefined>();
  let failure = $state("");
  let ended = $state(false);
</script>

<Window {monitor} layer="bottom" anchor={["top", "bottom", "left", "right"]} exclusiveZone={-1} namespace="shell-video"
        style={{ background: "#000000" }}>
  <Video src={source} {paused} {rate} {fit} loop={looping} muted={true}
         onready={(event) => (duration = event.duration)} onend={() => (ended = true)} onerror={(event) => (failure = event.message)}
         style={{ position: "absolute", inset: 0 }} />
</Window>

<Window {monitor} layer="top" anchor={["top", "left"]} margin={16} width={420} height={200} namespace="shell-video-panel">
  <View input style={{ flex: 1, background: "#000000cc", radius: 14, padding: 14, gap: 10 }}>
    <Text style={{ color: "white", fontSize: 16 }}>
      {failure === "" ? `duration ${duration ?? "?"} ms, rate ${rate}, ${paused ? "paused" : "playing"}, ${fit}${ended ? ", ended" : ""}` : failure}
    </Text>
    <View style={{ direction: "row", gap: 8 }}>
      <Button onpress={() => (paused = !paused)} style={{ background: "#334155", padding: [6, 12], radius: 8 }}><Text style={{ color: "white" }}>pause</Text></Button>
      <Button onpress={() => (rate = 2)} style={{ background: "#334155", padding: [6, 12], radius: 8 }}><Text style={{ color: "white" }}>2x</Text></Button>
      <Button onpress={() => (rate = 0.5)} style={{ background: "#334155", padding: [6, 12], radius: 8 }}><Text style={{ color: "white" }}>0.5x</Text></Button>
      <Button onpress={() => (fit = "contain")} style={{ background: "#334155", padding: [6, 12], radius: 8 }}><Text style={{ color: "white" }}>contain</Text></Button>
    </View>
  </View>
</Window>
