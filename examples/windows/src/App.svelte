<!--
  One app drawing in two windows: the count lives in this component, and both windows show
  it and change it. The second window opens while `open` is set and closes when it is not.
-->
<script lang="ts">
  import { Button, Text, View, Window, type Style } from "@neoworks/svelte-gpui/ui";

  let count = $state(0);
  let open = $state(true);

  const button: Style = { padding: [6, 12], radius: 6, background: "#27272a" };
</script>

<View style={{ padding: 24, gap: 12 }}>
  <Text style={{ fontSize: 20, fontWeight: "semibold" }}>Main window: {count}</Text>
  <View style={{ direction: "row", gap: 8 }}>
    <Button style={button} onpress={() => count++}><Text>Add one</Text></Button>
    <Button style={button} onpress={() => (open = !open)}><Text>{open ? "Close" : "Open"} the other window</Text></Button>
  </View>
</View>

{#if open}
  <Window
    title="windows: second"
    width={320}
    height={180}
    style={{ background: "#1e1b4b", padding: 24, gap: 12 }}
    onclose={() => (open = false)}
  >
    <Text style={{ fontSize: 20, color: "#c7d2fe" }}>Second window: {count}</Text>
    <Button style={button} onpress={() => count++}><Text>Add one here</Text></Button>
  </Window>
{/if}
