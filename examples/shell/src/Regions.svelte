<!--
  Input regions. A full-output window on the `top` layer has a panel marked `input`; the rest of
  it lets the pointer through to a window on the `bottom` layer. The bottom window has a zone
  at logical 100,300 and another exactly under the panel at 640,20, and each of them and the
  panel say whether the pointer is over them. Pressing "hide panel" hides the panel, and the
  region with it. With `enabled` off the top window takes the pointer everywhere. `marker` swaps
  the panel for a zero-size marker, which makes the whole top window click-through.
-->
<script lang="ts">
  import { Button, Text, View, Window } from "@neoworks/svelte-gpui/ui";

  let { monitor, enabled, marker = false }: { monitor: string; enabled: boolean; marker?: boolean } = $props();

  let zone = $state(false);
  let under = $state(false);
  let panel = $state(false);
  let hidden = $state(false);
</script>

<Window {monitor} layer="bottom" anchor={["top", "bottom", "left", "right"]} namespace="shell-below"
        style={{ background: "#1e293b" }}>
  <View style={{ position: "absolute", left: 100, top: 300, width: 300, height: 200, radius: 16, padding: 16,
                 background: zone ? "#15803d" : "#334155" }}
        onhover={(hovered) => (zone = hovered)}>
    <Text style={{ color: "white", fontSize: 18 }}>bottom zone: {zone ? "pointer over it" : "no pointer"}</Text>
  </View>
  <View style={{ position: "absolute", left: 640, top: 20, width: 300, height: 200, radius: 16, padding: 16,
                 background: under ? "#15803d" : "#334155" }}
        onhover={(hovered) => (under = hovered)}>
    <Text style={{ color: "white", fontSize: 18 }}>bottom zone under the panel: {under ? "pointer over it" : "no pointer"}</Text>
  </View>
</Window>

<Window {monitor} layer="top" anchor={["top", "bottom", "left", "right"]} namespace="shell-regions">
  <View style={{ flex: 1 }}>
    {#if marker}
      <View input={enabled} style={{ width: 0, height: 0 }} />
    {:else}
      <View input={enabled} onhover={(hovered) => (panel = hovered)}
            style={{ display: hidden ? "none" : "flex", alignSelf: "end", margin: 20, width: 300, height: 200, radius: 16, padding: 16, background: panel ? "#e11d48" : "#9f1239" }}>
        <Text style={{ color: "white", fontSize: 18 }}>top panel: {panel ? "pointer over it" : "no pointer"}</Text>
        <Button onpress={() => (hidden = true)} style={{ background: "#00000055", padding: [6, 12], radius: 8, marginTop: 12 }}>
          <Text style={{ color: "white" }}>hide panel</Text>
        </Button>
      </View>
    {/if}
  </View>
</Window>
