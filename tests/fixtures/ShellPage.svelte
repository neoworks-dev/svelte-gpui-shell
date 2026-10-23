<!-- An app with the shell primitives, for tests/shell.test.ts. -->
<script lang="ts">
  import { Popover, View, Window } from "@neoworks/svelte-gpui/ui";

  let marked = $state(true);

  // What the test drives, since nothing can click here.
  (globalThis as any).__page = {
    setMarked: (value: boolean) => (marked = value),
  };
</script>

<Window layer="top" anchor={["top", "bottom", "left", "right"]} namespace="shell-test">
  <View input={marked} style={{ width: 0, height: 0 }} />
  <Popover open input>
    <View />
  </Popover>
</Window>
