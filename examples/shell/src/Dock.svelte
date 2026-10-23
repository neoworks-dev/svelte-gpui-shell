<!--
  A dock whose popover rises above it. The dock's window is `tall` enough to hold the popover and
  keeps only the dock's own height as its exclusive zone, so other windows stay clear of the dock
  alone. Only the dock and the open popover take the pointer (`input`); the transparent rest of
  the window lets it through. Without `tall`, the window is the dock's own height and the popover
  has no room to open above it.
-->
<script lang="ts">
  import { Button, Popover, Text, View, Window } from "@neoworks/svelte-gpui/ui";

  let { monitor, tall }: { monitor: string; tall: boolean } = $props();

  const dockHeight = 96;
  let open = $state(false);
  let beside = $state(false);
  let behind = $state(false);
</script>

<Window {monitor} layer="bottom" anchor={["top", "bottom", "left", "right"]} exclusiveZone={-1} namespace="shell-wallpaper"
        style={{ background: "#1e293b" }}>
  <View onhover={(hovered) => (beside = hovered)}
        style={{ position: "absolute", left: 40, top: 220, width: 260, height: 100, padding: 12, radius: 12, background: beside ? "#15803d" : "#334155" }}>
    <Text style={{ color: "white" }}>below, beside the popover: {beside ? "pointer over it" : "no pointer"}</Text>
  </View>
  <View onhover={(hovered) => (behind = hovered)}
        style={{ position: "absolute", left: 360, top: 260, width: 240, height: 180, padding: 12, radius: 12, background: behind ? "#15803d" : "#334155" }}>
    <Text style={{ color: "white" }}>below, behind the popover: {behind ? "pointer over it" : "no pointer"}</Text>
  </View>
</Window>

<Window {monitor} layer="top" anchor={["bottom", "left", "right"]} height={tall ? 360 : dockHeight}
        exclusiveZone={dockHeight} namespace="shell-dock">
  <View style={{ flex: 1, justify: "end", align: "center" }}>
    <View input={tall} style={{ height: dockHeight - 24, margin: 12, padding: [0, 16], direction: "row", align: "center", gap: 12,
                                background: "#ffffff22", borderWidth: 1, borderColor: "#ffffff55", radius: 24 }}>
      <View style={{ size: 48, radius: 12, background: "#ef4444" }} />
      <Popover bind:open side="top" align="center" offset={12} input={tall} anchorStyle={{ alignSelf: "center" }}>
        {#snippet trigger({ toggle })}
          <Button onpress={toggle} style={{ size: 48, radius: 12, background: "#3b82f6", align: "center", justify: "center" }}>
            <Text style={{ color: "white" }}>app</Text>
          </Button>
        {/snippet}
        <View style={{ width: 240, height: 180, gap: 6 }}>
          <Text style={{ color: "white", fontSize: 16 }}>popover above the dock</Text>
          <Text style={{ color: "#cbd5e1" }}>drawn outside the dock's own 96 px</Text>
        </View>
      </Popover>
      <View style={{ size: 48, radius: 12, background: "#22c55e" }} />
    </View>
  </View>
</Window>
