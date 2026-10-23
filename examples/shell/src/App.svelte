<!--
  Scenes for the shell features, one per build: `SCENE=layers OUTPUT=shell-host node build.mjs`.
  They draw on the headless output named by OUTPUT, and on nothing while it is missing.
-->
<script lang="ts">
  import { monitors } from "@neoworks/svelte-gpui";
  import Layers from "./Layers.svelte";
  import Regions from "./Regions.svelte";
  import Empty from "./Empty.svelte";
  import Dock from "./Dock.svelte";

  const present = $derived(monitors.list.some((monitor) => monitor.name === __OUTPUT__));
</script>

{#if present}
  {#if __SCENE__ === "layers"}
    <Layers monitor={__OUTPUT__} />
  {:else if __SCENE__ === "regions"}
    <Regions monitor={__OUTPUT__} enabled={true} />
  {:else if __SCENE__ === "regions-off"}
    <Regions monitor={__OUTPUT__} enabled={false} />
  {:else if __SCENE__ === "marker"}
    <Regions monitor={__OUTPUT__} enabled={true} marker={true} />
  {:else if __SCENE__ === "dock"}
    <Dock monitor={__OUTPUT__} tall={true} />
  {:else if __SCENE__ === "dock-short"}
    <Dock monitor={__OUTPUT__} tall={false} />
  {:else if __SCENE__ === "empty"}
    <Empty monitor={__OUTPUT__} />
  {/if}
{/if}
