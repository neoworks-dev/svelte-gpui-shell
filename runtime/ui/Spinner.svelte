<!--
  An activity indicator for something that takes an unknown time: a ring that turns. It draws a
  frame every display refresh while it is on screen, so remove it when the work is done.

    {#if loading}<Spinner />{/if}
    <Spinner size={24} color="#8b5cf6" period={1200} />
-->
<script lang="ts">
  import Icon from "./Icon.svelte";
  import { ring } from "./icons";
  import type { Color, Style } from "./style";

  interface Props {
    /** Width and height in pixels. The default is 16. */
    size?: number;
    /** The default is the text colour around it. */
    color?: Color;
    /** Milliseconds for one turn. The default is 800. */
    period?: number;
    style?: Style;
  }

  let { size = 16, color, period = 800, style }: Props = $props();

  let angle = $state(0);

  $effect(() => {
    const turn = Math.max(period, 1);
    let handle = requestAnimationFrame(function step() {
      angle = ((performance.now() % turn) / turn) * 360;
      handle = requestAnimationFrame(step);
    });
    return () => cancelAnimationFrame(handle);
  });
</script>

<Icon svg={ring} {size} {color} style={{ rotate: angle, ...style }} />
