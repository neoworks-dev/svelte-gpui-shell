<!--
  Rows of digits with and without tabularNumbers: in the tabular column every digit is as wide as
  the others, so the right edges and the columns line up.
-->
<script lang="ts">
  import { Text, View, Window } from "@neoworks/svelte-gpui/ui";

  let { monitor }: { monitor: string } = $props();

  const rows = ["1111111", "0000000", "1234567", "8888888", "11:11:11", "00:08:41"];
</script>

<Window {monitor} layer="top" anchor={["top", "bottom", "left", "right"]} namespace="shell-numbers"
        style={{ background: "#101820" }}>
  <View style={{ direction: "row", gap: 60, padding: 30 }}>
    <View style={{ gap: 6 }}>
      <Text style={{ color: "#9ca3af", fontSize: 14 }}>proportional</Text>
      {#each rows as row}
        <Text style={{ color: "white", fontSize: 32 }}>{row}</Text>
      {/each}
    </View>
    <View style={{ gap: 6 }}>
      <Text style={{ color: "#9ca3af", fontSize: 14 }}>tabularNumbers</Text>
      {#each rows as row}
        <Text style={{ color: "white", fontSize: 32, tabularNumbers: true }}>{row}</Text>
      {/each}
    </View>
  </View>
</Window>
