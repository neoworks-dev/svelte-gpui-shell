<!--
  Gradients with more than two stops, and radial ones: a four-stop ramp in each direction, a
  radial spot, a fillet (a quarter circle cut out of a square corner) and a vignette.
-->
<script lang="ts">
  import { Text, View, Window } from "@neoworks/svelte-gpui/ui";

  let { monitor }: { monitor: string } = $props();

  const ramp = ["#ef4444", "#facc15", "#22c55e", "#3b82f6"] as [string, string, string, string];
  const box = { width: 200, height: 120, radius: 12 };
</script>

<Window {monitor} layer="top" anchor={["top", "bottom", "left", "right"]} namespace="shell-gradients"
        style={{ background: "#202830" }}>
  <View style={{ padding: 20, gap: 20 }}>
    <View style={{ direction: "row", gap: 20 }}>
      <View style={{ ...box, background: { linear: { angle: 90, stops: ramp } } }} />
      <View style={{ ...box, background: { linear: { angle: 180, stops: ["#ffffff", ["#ef4444", 0.2], ["#3b82f6", 0.8]] } } }} />
      <View style={{ ...box, background: { linear: { angle: 135, stops: ramp } } }} />
      <View style={{ ...box, background: { linear: { angle: 90, stops: ["#000000", ["#ffffff", 0.5], ["#ffffff", 0.5], "#000000"] } } }} />
    </View>
    <View style={{ direction: "row", gap: 20 }}>
      <View style={{ ...box, background: { radial: { stops: ["#ffffff", "#7c3aed", "#1e1b4b"] } } }} />
      <View style={{ ...box, background: { radial: { center: [0.2, 0.3], radius: [0.4, 0.7], stops: ramp } } }} />
      <View style={{ width: 120, height: 120, background: { radial: { center: [1, 1], radius: 1, stops: ["#00000000", ["#00000000", 0.97], ["#f97316", 0.98], "#f97316"] } } }} />
      <View style={{ ...box, background: { radial: { radius: 0.7, stops: ["#00000000", "#000000cc"] } } }}>
        <Text style={{ color: "white" }}>vignette</Text>
      </View>
    </View>
  </View>
</Window>
