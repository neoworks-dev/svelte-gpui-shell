<!--
  Hover targets at the surface's top-left corner and in the middle. Nothing is hovered until a
  pointer enters the surface, and nothing is after it leaves.
-->
<script lang="ts">
  import { Text, View, Window } from "@neoworks/svelte-gpui/ui";

  let { monitor }: { monitor: string } = $props();

  let corner = $state(false);
  let middle = $state(false);
  let changes = $state(0);
</script>

<Window {monitor} layer="top" anchor={["top", "bottom", "left", "right"]} namespace="shell-hover"
        style={{ background: "#101820" }}>
  <View onhover={(hovered) => { corner = hovered; changes += 1; }}
        style={{ position: "absolute", left: 0, top: 0, width: 200, height: 200, background: "#aa2222" }}
        hover={{ background: "#22aa22" }}>
    <Text style={{ color: "white" }}>corner {corner ? "hovered" : "idle"}</Text>
  </View>
  <View onhover={(hovered) => { middle = hovered; changes += 1; }}
        style={{ position: "absolute", left: 600, top: 300, width: 200, height: 200, background: "#aa2222" }}
        hover={{ background: "#22aa22" }}>
    <Text style={{ color: "white" }}>middle {middle ? "hovered" : "idle"}</Text>
  </View>
  <View style={{ position: "absolute", left: 300, top: 40 }}>
    <Text style={{ color: "white", fontSize: 20 }}>hover changes: {changes}</Text>
  </View>
</Window>
