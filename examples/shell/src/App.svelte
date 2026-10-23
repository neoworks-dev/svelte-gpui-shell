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
  import Lock from "./Lock.svelte";
  import Hover from "./Hover.svelte";
  import Bytes from "./Bytes.svelte";
  import Push from "./Push.svelte";
  import Gradients from "./Gradients.svelte";
  import Numbers from "./Numbers.svelte";
  import VideoScene from "./Video.svelte";

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
  {:else if __SCENE__ === "lock"}
    <Lock monitor={__OUTPUT__} />
  {:else if __SCENE__ === "video"}
    <VideoScene monitor={__OUTPUT__} source={__VIDEO__} looping={true} />
  {:else if __SCENE__ === "video-once"}
    <VideoScene monitor={__OUTPUT__} source={__VIDEO__} looping={false} />
  {:else if __SCENE__ === "bytes"}
    <Bytes monitor={__OUTPUT__} />
  {:else if __SCENE__ === "push"}
    <Push monitor={__OUTPUT__} />
  {:else if __SCENE__ === "gradients"}
    <Gradients monitor={__OUTPUT__} />
  {:else if __SCENE__ === "numbers"}
    <Numbers monitor={__OUTPUT__} />
  {:else if __SCENE__ === "hover"}
    <Hover monitor={__OUTPUT__} />
  {:else if __SCENE__ === "empty"}
    <Empty monitor={__OUTPUT__} />
  {/if}
{/if}
