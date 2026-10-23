<!-- An app with the shell primitives, for tests/shell.test.ts. -->
<script lang="ts">
  import { sessionLock } from "@neoworks/svelte-gpui";
  import { Popover, Text, View, Window } from "@neoworks/svelte-gpui/ui";

  let marked = $state(true);
  let locking = $state(false);

  // What the test drives, since nothing can click here.
  (globalThis as any).__page = {
    setMarked: (value: boolean) => (marked = value),
    setLocking: (value: boolean) => (locking = value),
  };
</script>

<Window layer="top" anchor={["top", "bottom", "left", "right"]} namespace="shell-test">
  <View input={marked} style={{ width: 0, height: 0 }} />
  <Popover open input>
    <View />
  </Popover>
  <Text>lock is {sessionLock.state}</Text>
</Window>

{#if locking}
  <Window lock monitor="DP-2"><View /></Window>
{/if}
