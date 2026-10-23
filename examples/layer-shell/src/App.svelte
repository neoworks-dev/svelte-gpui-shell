<!--
  A bar and a card as layer surfaces, on one monitor. It draws on the headless output that
  scripts/shot.sh creates (`hyprctl output create headless svelte-gpui`), and on nothing when
  that is missing: a Window that names a monitor that is not there does not open. Set `target`
  to a monitor from `monitors.list` to see it on a real one.
-->
<script lang="ts">
  import { monitors } from "@neoworks/svelte-gpui";
  import { Button, Text, Window } from "@neoworks/svelte-gpui/ui";

  const target = "svelte-gpui";
  const present = $derived(monitors.list.some((monitor) => monitor.name === target));
  let tall = $state(false);
  let zone = $state(32);
</script>

{#if present}
  <Window monitor={target} layer="top" anchor={["top", "left", "right"]} height={tall ? 64 : 32}
          exclusiveZone={zone} namespace="svelte-gpui-example-bar" style={{ background: "#3730a3", direction: "row", align: "center", padding: [0, 12], gap: 12 }}>
    <Text style={{ color: "white" }}>bar on {target}: {monitors.list.map((monitor) => monitor.name).join(", ")}</Text>
    <Button style={{ background: "#6366f1", padding: [2, 8], radius: 4 }} onpress={() => { tall = !tall; zone = tall ? 64 : 32; }}><Text>Taller</Text></Button>
  </Window>
  <Window monitor={target} layer="bottom" anchor={["bottom", "right"]} width={220} height={120} margin={16}
          namespace="svelte-gpui-example-card" style={{ background: "#065f46", radius: 12, padding: 12 }}>
    <Text style={{ color: "white" }}>card, bottom right</Text>
  </Window>
{/if}
