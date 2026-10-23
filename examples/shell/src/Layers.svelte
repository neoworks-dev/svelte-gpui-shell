<!--
  Transparent layer surfaces over a wallpaper: a full-output window with no background that
  holds a notch and a dock, each with its own translucent colours, and nothing else. The
  striped wallpaper is a layer surface of its own, under them.
-->
<script lang="ts">
  import { Text, View, Window } from "@neoworks/svelte-gpui/ui";

  let { monitor }: { monitor: string } = $props();
  const stripes = ["#f97316", "#facc15", "#22c55e", "#06b6d4", "#6366f1", "#d946ef"];
</script>

<Window {monitor} layer="bottom" anchor={["top", "bottom", "left", "right"]} namespace="shell-wallpaper">
  <View style={{ flex: 1, direction: "row" }}>
    {#each stripes as stripe}
      <View style={{ flex: 1, background: stripe }} />
    {/each}
  </View>
</Window>

<Window {monitor} layer="top" anchor={["top", "bottom", "left", "right"]} namespace="shell-layers">
  <View style={{ flex: 1, justify: "between", align: "center", padding: 24 }}>
    <View style={{ background: "#00000099", radius: 14, padding: [8, 24], direction: "row", gap: 12, align: "center" }}>
      <Text style={{ color: "white", fontSize: 18 }}>notch: translucent black over the wallpaper</Text>
    </View>
    <View style={{ background: "#ffffff33", radius: 20, padding: 12, direction: "row", gap: 12, borderWidth: 1, borderColor: "#ffffff88" }}>
      <View style={{ size: 64, radius: 16, background: "#ef4444" }} />
      <View style={{ size: 64, radius: 16, background: "#22c55e" }} />
      <View style={{ size: 64, radius: 16, background: "#3b82f6" }} />
    </View>
  </View>
</Window>
