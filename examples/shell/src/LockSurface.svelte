<!--
  What one monitor shows while the session is locked: the time, where the lock is, and a field
  for the password. It is plain content, so it can be drawn in an ordinary window to look at it
  without locking anything.
-->
<script lang="ts">
  import { sessionLock } from "@neoworks/svelte-gpui";
  import { Input, Text, View } from "@neoworks/svelte-gpui/ui";

  let { name, onunlock }: { name: string; onunlock: (password: string) => void } = $props();

  let password = $state("");
  const time = new Date().toTimeString().slice(0, 5);
</script>

<View style={{ flex: 1, background: "#0f172a", justify: "center", align: "center", gap: 16 }}>
  <Text style={{ color: "white", fontSize: 72, fontWeight: "bold" }}>{time}</Text>
  <Text style={{ color: "#94a3b8", fontSize: 18 }}>{name} is {sessionLock.state}</Text>
  <Input bind:value={password} placeholder="password" onsubmit={() => onunlock(password)}
         style={{ width: 280, padding: [10, 14], radius: 10, background: "#1e293b", color: "white" }} />
</View>
