<!--
  Layer surfaces that draw nothing: a window with no children and no background, and one whose
  only content is a zero-size view. Each must be mapped by the compositor and stay still.
-->
<script lang="ts">
  import { View, Window } from "@neoworks/svelte-gpui/ui";

  let { monitor }: { monitor: string } = $props();
</script>

<Window {monitor} layer="top" anchor={["top", "bottom", "left", "right"]} namespace="shell-empty" />
<Window {monitor} layer="top" anchor={["top", "left", "right"]} height={40} namespace="shell-empty-marker">
  <View style={{ width: 0, height: 0 }} />
</Window>
