<!--
  A line-framed server that pushes to a client that subscribed, with no request pending: the way a
  control socket delivers bus topics. The client is the app's own `unixSockets.connect`.
-->
<script lang="ts">
  import { unixSockets } from "@neoworks/svelte-gpui";
  import { Text, View, Window } from "@neoworks/svelte-gpui/ui";

  let { monitor }: { monitor: string } = $props();

  const path = "/tmp/shell-host-push.sock";
  let received = $state<string[]>([]);
  let status = $state("starting");

  $effect(() => {
    let tick = 0;
    const server = unixSockets.serve(path, (request, peer) => {
      if (request !== "subscribe") {
        return "unknown";
      }
      const timer = setInterval(() => peer.write(`tick ${(tick += 1)}\n`), 200);
      setTimeout(() => clearInterval(timer), 1500);
      return "subscribed";
    }, {
      framing: "line",
      onPeer: (peer) => (status = `client ${peer.id} connected`),
      onPeerGone: (peer) => (status = `client ${peer.id} gone`),
    });
    server.ready.then(() => {
      const client = unixSockets.connect(path, {
        onOpen: () => client.write("subscribe\n"),
        onLine: (line) => (received = [...received, line]),
      });
      setTimeout(() => server.broadcast("broadcast to everyone\n"), 1000);
    });
    return () => server.close();
  });
</script>

<Window {monitor} layer="top" anchor={["top", "bottom", "left", "right"]} namespace="shell-push"
        style={{ background: "#101820" }}>
  <View style={{ padding: 20, gap: 6 }}>
    <Text style={{ color: "white", fontSize: 18 }}>{status}</Text>
    {#each received as line}
      <Text style={{ color: "#9ae6b4", fontSize: 16 }}>{line}</Text>
    {/each}
  </View>
</Window>
