<!--
  The session lock, without ever locking. `LockSurface` is drawn in an ordinary window so it can
  be looked at, and "request lock" mounts the lock windows a real locker would, one per monitor.
  The host opens lock windows only when SVELTE_GPUI_ALLOW_SESSION_LOCK=1 is set. Without it, as
  under scripts/shot.sh, it refuses and `sessionLock.state` goes to "finished". With it, the same
  press locks the session, so never set it to try this scene.
-->
<script lang="ts">
  import { monitors, sessionLock } from "@neoworks/svelte-gpui";
  import { Button, Text, View, Window } from "@neoworks/svelte-gpui/ui";
  import LockSurface from "./LockSurface.svelte";

  let { monitor }: { monitor: string } = $props();

  let requested = $state(false);
</script>

<Window title="lock preview" width={640} height={360} style={{ background: "#020617" }}>
  <View style={{ flex: 1, padding: 12, gap: 12 }}>
    <View style={{ direction: "row", gap: 12, align: "center" }}>
      <Button onpress={() => (requested = true)} style={{ background: "#be123c", padding: [6, 12], radius: 8 }}>
        <Text style={{ color: "white" }}>request lock</Text>
      </Button>
      <Button onpress={() => (requested = false)} style={{ background: "#334155", padding: [6, 12], radius: 8 }}>
        <Text style={{ color: "white" }}>release</Text>
      </Button>
      <Text style={{ color: "white" }}>sessionLock.state = {sessionLock.state}</Text>
    </View>
    <View style={{ flex: 1, radius: 12, overflow: "hidden" }}>
      <LockSurface name={monitor} onunlock={() => (requested = false)} />
    </View>
  </View>
</Window>

{#if requested}
  {#each monitors.list as screen (screen.name)}
    <Window lock monitor={screen.name}>
      <LockSurface name={screen.name} onunlock={() => (requested = false)} />
    </Window>
  {/each}
{/if}
